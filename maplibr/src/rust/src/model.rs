use crate::errors::argument_error;
use crate::errors::maplib_error;
use crate::templates::RTemplate;
use maplib::model::MapOptions;
use oxrdf::NamedNode;
use oxrdfio::RdfFormat;
use polars::prelude::{
    coalesce, col, concat, Column, DataFrame, DataType, Expr, IntoLazy, Series, UnionArgs,
};
use representation::cats::LockedCats;
use representation::dataset::NamedGraph;
use representation::rdf_to_polars::rdf_named_node_to_polars_literal_value;
use representation::solution_mapping::EagerSolutionMappings;
use representation::{
    BaseRDFNodeType, RDFNodeState, LANG_STRING_VALUE_FIELD, OBJECT_COL_NAME, PREDICATE_COL_NAME,
    SUBJECT_COL_NAME,
};
use savvy::{savvy, ListSexp, Sexp};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use triplestore::triples_read::ExtendedRdfFormat;
use triplestore::IndexingOptions;

const DEFAULT_TRIPLES_BATCH_SIZE: usize = 10_000_000;

/// Mirrors py_maplib's `resolve_normal_format` (py_maplib/src/lib.rs:303-316).
fn resolve_normal_format(format: &str) -> savvy::Result<RdfFormat> {
    match format.to_lowercase().as_str() {
        "ntriples" => Ok(RdfFormat::NTriples),
        "turtle" => Ok(RdfFormat::Turtle),
        "rdf/xml" | "xml" | "rdfxml" => Ok(RdfFormat::RdfXml),
        _ => Err(argument_error(format!("Unknown format: {}", format))),
    }
}

/// Mirrors py_maplib's `resolve_format` (py_maplib/src/lib.rs:318-327).
/// CIM XML and HDT deliberately unsupported here (out of scope for v1, see plan).
fn resolve_format(format: &str) -> savvy::Result<ExtendedRdfFormat> {
    resolve_normal_format(format).map(ExtendedRdfFormat::Normal)
}

fn parse_optional_named_graph(graph: Option<&str>) -> savvy::Result<NamedGraph> {
    let nn = graph
        .map(|g| NamedNode::new(g).map_err(argument_error))
        .transpose()?;
    Ok(NamedGraph::from_maybe_named_node(nn.as_ref()))
}

/// Parses `query()`'s `graph` argument. Unlike `reads`/`writes`, where a
/// missing graph means "the default graph" (`parse_optional_named_graph`),
/// a missing graph here means "no restriction to one named graph" -- mirrors
/// py_maplib's own `query`, which passes `graph: Option<&NamedGraph>`
/// through to `Model::query` unchanged rather than defaulting it (py_model.rs:359-360).
fn parse_query_graph(graph: Option<&str>) -> savvy::Result<Option<NamedGraph>> {
    graph
        .map(|g| {
            let nn = NamedNode::new(g).map_err(argument_error)?;
            Ok(NamedGraph::from_maybe_named_node(Some(&nn)))
        })
        .transpose()
}

/// Flattens a CONSTRUCT query's per-pattern results (each pattern's own
/// solution mappings, plus a constant predicate when the pattern used one
/// literally rather than binding it, e.g. `CONSTRUCT { ?s a ?type }`) into a
/// single subject/predicate/object `EagerSolutionMappings`, matching
/// `RModel::query()`'s single-DataFrame-per-call design. Mirrors py_maplib's
/// own per-pattern handling (`query_to_result`, py_maplib/src/lib.rs:235-263)
/// for adding the constant predicate column, but concatenates every
/// pattern's rows into one combined DataFrame instead of returning a
/// separate DataFrame per pattern -- py_maplib can return a heterogeneous
/// Python list; a single R data.frame per query() call cannot.
///
/// Deliberately simplified relative to `query()`'s SELECT path: every
/// pattern's subject/predicate/object columns are cast to plain strings
/// before concatenating, rather than preserved as natively-typed (e.g.
/// integer/date) or per-row multi-typed columns. Different CONSTRUCT
/// patterns can produce genuinely different column dtypes (one pattern's
/// object might be an integer literal, another's a string), which a single
/// concatenated DataFrame can't represent without collapsing to one common
/// type anyway -- so the `rdf_node_types` side-channel reports predicate as
/// IRI (always true per the CONSTRUCT grammar) and subject/object as
/// untyped, rather than claiming a precision this can't actually preserve.
fn construct_result_to_solution_mappings(
    groups: Vec<(EagerSolutionMappings, Option<NamedNode>)>,
    global_cats: LockedCats,
) -> savvy::Result<EagerSolutionMappings> {
    let mut lfs = Vec::with_capacity(groups.len());
    for (sm, predicate) in groups {
        let EagerSolutionMappings {
            mappings,
            mut rdf_node_types,
        } = sm;
        // A pattern's own subject/object binds against a WHERE clause
        // variable, which is very commonly multi-typed (its RDF node type
        // can vary by row, e.g. one row's ?o is a string, another's an
        // integer) -- format_native_columns represents that as a Struct
        // column (one field per possible type, at most one non-null per
        // row), the same shape query()'s SELECT path sends to R for
        // R-side collapsing (.collapse_multitype_columns). That collapsing
        // has to happen here instead, before casting to String and
        // concatenating: a Struct column can't be cast to String directly
        // (confirmed live -- it silently casts every row to NA rather than
        // erroring), and each pattern's own R-side round trip is skipped
        // for CONSTRUCT (only the final combined frame goes to R).
        let subject_state = rdf_node_types.get(SUBJECT_COL_NAME).cloned();
        let object_state = rdf_node_types.get(OBJECT_COL_NAME).cloned();
        let mut lf = representation::formatting::format_native_columns(
            mappings.lazy(),
            &mut rdf_node_types,
            global_cats.clone(),
        );
        if let Some(state) = subject_state {
            lf = lf.with_column(flatten_to_string(SUBJECT_COL_NAME, &state));
        }
        if let Some(state) = object_state {
            lf = lf.with_column(flatten_to_string(OBJECT_COL_NAME, &state));
        }
        if let Some(predicate) = &predicate {
            lf = lf.with_column(
                polars::prelude::lit(rdf_named_node_to_polars_literal_value(predicate))
                    .alias(PREDICATE_COL_NAME),
            );
        } else {
            lf = lf.with_column(col(PREDICATE_COL_NAME).cast(DataType::String));
        }
        lf = lf.select([
            col(SUBJECT_COL_NAME),
            col(PREDICATE_COL_NAME),
            col(OBJECT_COL_NAME),
        ]);
        lfs.push(lf);
    }
    let mappings = if lfs.is_empty() {
        // A CONSTRUCT template with literally zero triple patterns (valid
        // but pointless SPARQL) -- build an empty, correctly-shaped
        // DataFrame rather than concat()ing an empty list of LazyFrames
        // (which would produce a schema-less frame, not the three empty
        // String columns export_solution_mappings expects).
        let empty_col =
            |name: &str| Column::from(Series::new_empty(name.into(), &DataType::String));
        DataFrame::new(
            0,
            vec![
                empty_col(SUBJECT_COL_NAME),
                empty_col(PREDICATE_COL_NAME),
                empty_col(OBJECT_COL_NAME),
            ],
        )
        .map_err(|e| crate::errors::runtime_error(e.to_string()))?
    } else {
        concat(
            lfs,
            UnionArgs {
                parallel: true,
                rechunk: true,
                ..Default::default()
            },
        )
        .map_err(|e| crate::errors::runtime_error(e.to_string()))?
        .collect()
        .map_err(|e| crate::errors::runtime_error(e.to_string()))?
    };

    let rdf_node_types = HashMap::from([
        (
            SUBJECT_COL_NAME.to_string(),
            BaseRDFNodeType::None.into_default_input_rdf_node_state(),
        ),
        (
            PREDICATE_COL_NAME.to_string(),
            BaseRDFNodeType::IRI.into_default_input_rdf_node_state(),
        ),
        (
            OBJECT_COL_NAME.to_string(),
            BaseRDFNodeType::None.into_default_input_rdf_node_state(),
        ),
    ]);
    Ok(EagerSolutionMappings {
        mappings,
        rdf_node_types,
    })
}

/// Flattens `col(name)` -- already run through `format_native_columns`, so
/// either a single decoded column or (if `state.is_multi()`) a Struct with
/// one field per possible type -- down to one plain String column. For a
/// multi-typed column, coalesces across the Struct's fields (named per
/// `BaseRDFNodeType::field_col_name()`, matching exactly what
/// `expression_to_native`'s multi branch built them as; the lang-string
/// type is the one exception, itself a 2-field value/lang Struct rather
/// than a single field, hence the `LANG_STRING_VALUE_FIELD` special case --
/// its language tag is dropped here, same simplification as
/// `.collapse_multitype_columns` on the R side), each field individually
/// cast to String first so mismatched underlying dtypes (e.g. an integer
/// field alongside a string field) don't need a common supertype.
fn flatten_to_string(name: &str, state: &RDFNodeState) -> Expr {
    if !state.is_multi() {
        // A single-typed rdf:langString column is *also* Struct-shaped
        // (value + language tag fields, not one flat column) --
        // expression_to_native's non-multi branch itself falls back to
        // as_struct(exprs) whenever the base type needs more than one
        // expression, which is exactly the lang-string case.
        return if state.is_literal() && state.get_base_type().unwrap().is_lang_string() {
            col(name)
                .struct_()
                .field_by_name(LANG_STRING_VALUE_FIELD)
                .cast(DataType::String)
                .alias(name)
        } else {
            col(name).cast(DataType::String).alias(name)
        };
    }
    let candidates: Vec<Expr> = state
        .get_sorted_types()
        .into_iter()
        .map(|t| {
            let field = if t.is_lang_string() {
                LANG_STRING_VALUE_FIELD.to_string()
            } else {
                t.field_col_name()
            };
            col(name)
                .struct_()
                .field_by_name(&field)
                .cast(DataType::String)
        })
        .collect();
    coalesce(&candidates).alias(name)
}

/// A maplib knowledge graph model.
///
/// @export
#[savvy]
pub struct RModel {
    inner: Mutex<maplib::model::Model>,
}

impl RModel {
    /// Lock the inner Model, recovering from poison rather than panicking.
    ///
    /// std's Mutex poisons itself if a panic happens while it's held (e.g.
    /// serialize()/deserialize() hitting the lib/disk stub's unimplemented!())
    /// -- left as the default `.lock().unwrap()`, every later call on the
    /// SAME RModel would then panic forever with PoisonError, even for
    /// operations unrelated to whatever originally panicked. Python's GIL
    /// mutex has no poisoning concept, so `into_inner()` (ignore the poison,
    /// keep using the data) is the closest match to py_maplib's behavior --
    /// the data itself is still perfectly valid, since maplib's own methods
    /// don't panic mid-mutation in a way that would leave it inconsistent.
    fn lock(&self) -> std::sync::MutexGuard<'_, maplib::model::Model> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[savvy]
impl RModel {
    /// Create a new, empty Model.
    ///
    /// @export
    fn new() -> savvy::Result<Self> {
        // Required: see CLAUDE.md's pyo3-leak note. lib/utils/src/polars.rs's
        // pl_interruptable_collect (used by query/insert/update/map/
        // infer_rdfs -- anything that materializes a LazyFrame) calls
        // pyo3::Python::attach unconditionally when built with the pyo3
        // feature, which panics unless some Python interpreter has been
        // initialized in this process. Python::initialize() is pyo3's own
        // documented mechanism for embedding in a non-Python host; safe to
        // call on every RModel::new() (verified idempotent -- a second call
        // in the same session doesn't panic or error).
        pyo3::Python::initialize();

        let model = maplib::model::Model::new(None, None, None, None)
            .map_err(maplib_error)?;
        Ok(Self {
            inner: Mutex::new(model),
        })
    }

    /// Number of triples in a graph (mirrors PyModel::size,
    /// py_maplib/src/py_model.rs:144-151 / size_mutex, py_maplib/src/
    /// mutexes.rs:66-73).
    ///
    /// @param graph Optional named graph IRI to count (default graph if NULL).
    /// @export
    fn size(&self, graph: Option<&str>) -> savvy::Result<savvy::Sexp> {
        let named_graph = parse_optional_named_graph(graph)?;
        let inner = self.lock();
        (inner.graph_size(&named_graph) as i32).try_into()
    }

    /// Run a SPARQL SELECT or CONSTRUCT query, streaming the result out
    /// through the Arrow C Stream Interface (mirrors PyModel::query,
    /// py_maplib/src/py_model.rs:344-387; CONSTRUCT results are flattened
    /// into one combined subject/predicate/object DataFrame, see
    /// construct_result_to_solution_mappings).
    ///
    /// @param sparql The SPARQL query string.
    /// @param stream_ptr An external pointer from `nanoarrow::nanoarrow_allocate_array_stream()`.
    /// @param include_transient Whether to include transient (e.g. inferred)
    ///   triples in the query. Defaults to FALSE.
    /// @param graph Optional named graph IRI to restrict the query to (searches
    ///   across the whole store if NULL, matching py_maplib's default).
    /// @returns The `rdf_node_types` side-channel, as a JSON string.
    /// @export
    fn query(
        &self,
        sparql: &str,
        stream_ptr: savvy::Sexp,
        include_transient: bool,
        graph: Option<&str>,
    ) -> savvy::Result<savvy::Sexp> {
        let named_graph = parse_query_graph(graph)?;
        let mut inner = self.lock();
        let global_cats = inner.triplestore.global_cats.clone();
        let result = inner
            .query(
                sparql,
                None,
                named_graph.as_ref(),
                false,
                include_transient,
                None,
                false,
                None,
            )
            .map_err(maplib_error)?;
        match result.kind {
            representation::result::QueryResultKind::Select(sm) => {
                // Query results store repeated IRI/literal values as category
                // codes into a Model-wide dictionary (Triplestore::global_cats,
                // LockedCats) rather than plain polars Categorical-typed
                // columns -- format_native_columns (mirrors py_maplib's
                // native_dataframe=True path, py_model.rs query_to_result via
                // fix_cats_and_multicolumns) is what actually resolves those
                // codes back to real values (via maybe_decode_expr) and
                // collapses "multi" RDF-typed columns into one Struct field
                // per possible type, matching the shape export_solution_mappings
                // expects. A plain Series-level Categorical->String cast
                // (arrow_bridge::decategorize) does NOT do this -- confirmed
                // live, a query result round-tripped to R without this step
                // came back as raw integer category codes, not strings.
                let representation::solution_mapping::EagerSolutionMappings {
                    mappings,
                    mut rdf_node_types,
                } = sm;
                let lf = representation::formatting::format_native_columns(
                    mappings.lazy(),
                    &mut rdf_node_types,
                    global_cats,
                );
                let mappings = lf
                    .collect()
                    .map_err(|e| crate::errors::runtime_error(e.to_string()))?;
                let sm = representation::solution_mapping::EagerSolutionMappings {
                    mappings,
                    rdf_node_types,
                };
                crate::arrow_bridge::export_solution_mappings(sm, stream_ptr)
            }
            representation::result::QueryResultKind::Construct(groups) => {
                construct_result_to_solution_mappings(groups, global_cats)
                    .and_then(|sm| crate::arrow_bridge::export_solution_mappings(sm, stream_ptr))
            }
        }
    }

    /// Parse RDF triples from a string into this Model.
    ///
    /// @param s RDF data as a string.
    /// @param format One of "ntriples", "turtle", "xml" (rdf/xml).
    /// @param graph Optional named graph IRI to read into (default graph if NULL).
    /// @export
    fn reads(&self, s: &str, format: &str, graph: Option<&str>) -> savvy::Result<()> {
        let format = resolve_format(format)?;
        let named_graph = parse_optional_named_graph(graph)?;
        let mut inner = self.lock();
        inner
            .reads(
                s,
                format,
                None,
                false,
                None,
                true,
                &named_graph,
                false,
                DEFAULT_TRIPLES_BATCH_SIZE,
                HashMap::new(),
            )
            .map_err(maplib_error)
    }

    /// Serialize this Model's triples to a string.
    ///
    /// @param format One of "ntriples", "turtle", "xml" (rdf/xml). Defaults to "ntriples".
    /// @param graph Optional named graph IRI to write (default graph if NULL).
    /// @export
    fn writes(&self, format: Option<&str>, graph: Option<&str>) -> savvy::Result<savvy::Sexp> {
        let format = format
            .map(resolve_normal_format)
            .transpose()?
            .unwrap_or(RdfFormat::NTriples);
        let named_graph = parse_optional_named_graph(graph)?;
        let mut inner = self.lock();
        let mut out = Vec::new();
        inner
            .write_triples(&mut out, &named_graph, format, None)
            .map_err(maplib_error)?;
        String::from_utf8(out)
            .map_err(crate::errors::runtime_error)?
            .try_into()
    }

    /// Build the default (non-FTS) indexes, matching py_maplib's create_index()
    /// with no options (create_index_mutex, py_maplib/src/mutexes.rs:402-413).
    ///
    /// @export
    fn create_index(&self) -> savvy::Result<()> {
        let mut inner = self.lock();
        inner
            .create_index(IndexingOptions::default())
            .map_err(maplib_error)
    }

    /// Remove all triples from a graph (mirrors truncate_graph_mutex,
    /// py_maplib/src/mutexes.rs:98-104 -- delegates straight to the
    /// triplestore, Model has no dedicated wrapper method).
    ///
    /// @param graph Optional named graph IRI to truncate (default graph if NULL).
    /// @export
    fn truncate_graph(&self, graph: Option<&str>) -> savvy::Result<()> {
        let named_graph = parse_optional_named_graph(graph)?;
        let mut inner = self.lock();
        inner.triplestore.truncate(&named_graph);
        Ok(())
    }

    /// Add prefix -> IRI mappings, used when serializing (mirrors
    /// add_prefixes_mutex, py_maplib/src/mutexes.rs:90-96 -- direct field
    /// extend, no dedicated Model method).
    ///
    /// @param prefixes A named R list of single strings: names are prefixes,
    ///   values are IRIs, e.g. `list(ex = "http://example.org/")`.
    /// @export
    fn add_prefixes(&self, prefixes: ListSexp) -> savvy::Result<()> {
        let mut parsed = HashMap::new();
        for (name, value) in prefixes.iter() {
            let iri = <&str>::try_from(value)?;
            let nn = NamedNode::new(iri).map_err(argument_error)?;
            parsed.insert(name.to_string(), nn);
        }
        let mut inner = self.lock();
        inner.prefixes.extend(parsed);
        Ok(())
    }

    /// Copy a graph from another Model's triplestore into this one (mirrors
    /// PyModel::add_graph, py_maplib/src/py_model.rs:997-1015).
    ///
    /// @param other Another Model to copy a graph from.
    /// @param source_graph Optional named graph IRI in `other` (default graph if NULL).
    /// @param target_graph Optional named graph IRI in this Model to copy into (default graph if NULL).
    /// @export
    fn add_graph(
        &self,
        other: &RModel,
        source_graph: Option<&str>,
        target_graph: Option<&str>,
    ) -> savvy::Result<()> {
        let source_graph = parse_optional_named_graph(source_graph)?;
        let target_graph = parse_optional_named_graph(target_graph)?;
        let other_inner = other.lock();
        let mut inner = self.lock();
        inner
            .add_graph(&other_inner.triplestore, source_graph, target_graph)
            .map_err(maplib_error)
    }

    /// Split a graph out of this Model into a brand new, standalone Model
    /// (mirrors detach_graph_mutex, py_maplib/src/mutexes.rs:106-118).
    ///
    /// @param preserve_name Keep the graph's own name in the new Model rather
    ///   than moving it to the default graph there.
    /// @param graph Optional named graph IRI to detach (default graph if NULL).
    /// @returns A new Model containing only the detached graph.
    /// @export
    fn detach_graph(&self, preserve_name: bool, graph: Option<&str>) -> savvy::Result<RModel> {
        let named_graph = parse_optional_named_graph(graph)?;
        let mut inner = self.lock();
        let sprout = inner
            .detach_graph(&named_graph, preserve_name)
            .map_err(maplib_error)?;
        Ok(RModel {
            inner: Mutex::new(sprout),
        })
    }

    /// Run RDFS inference over a graph in place, returning the number of new
    /// triples inferred -- a triple count, not a rule count
    /// (Triplestore::interesting_rdfs_rules, lib/triplestore/src/
    /// rdfs_inferencing.rs, sums per-rule inserted-triple counts) -- mirrors
    /// PyModel::infer_rdfs (py_maplib/src/py_model.rs:903-916).
    ///
    /// @param graph Optional named graph IRI to infer over (default graph if NULL).
    /// @export
    fn infer_rdfs(&self, graph: Option<&str>) -> savvy::Result<savvy::Sexp> {
        let named_graph = parse_optional_named_graph(graph)?;
        let mut inner = self.lock();
        let n = inner
            .infer_rdfs(&named_graph)
            .map_err(maplib_error)?;
        (n as i32).try_into()
    }

    /// Compact on-disk storage (mirrors PyModel::compact, py_maplib/src/py_model.rs:919-924).
    ///
    /// @export
    fn compact(&self) -> savvy::Result<()> {
        let mut inner = self.lock();
        inner.compact().map_err(maplib_error)
    }

    /// Serialize this Model's triples to a directory in maplib's own compact
    /// on-disk format (not an RDF interchange format -- use write()/writes()
    /// for that). Mirrors PyModel::serialize, py_maplib/src/py_model.rs:926-934.
    ///
    /// @param path Directory to serialize into. Defaults to
    ///   "./serialized_triples", matching py_maplib.
    /// @export
    fn serialize(&self, path: Option<&str>) -> savvy::Result<()> {
        let mut inner = self.lock();
        inner
            .serialize_triples(Path::new(path.unwrap_or("./serialized_triples")))
            .map_err(maplib_error)
    }

    /// Register an already-built Template (mirrors `Model::add_template`,
    /// lib/maplib/src/model.rs:165 -- py_maplib's own `add_template` just
    /// forwards a parsed `Template` here too, py_model.rs:74-80).
    ///
    /// @param template An RTemplate (the raw pointer behind maplibr's S7
    ///   `Template` class, `template@raw`).
    /// @export
    fn add_template(&self, template: &RTemplate) -> savvy::Result<()> {
        let mut inner = self.lock();
        inner
            .add_template(template.inner.clone())
            .map_err(maplib_error)
    }

    /// Parse an stOTTR document string and register every template it
    /// defines (mirrors `Model::add_templates_from_string`,
    /// lib/maplib/src/model.rs:177-194). Errors if the document defines no
    /// templates at all, matching py_maplib's own message for that case
    /// (py_maplib/src/mutexes.rs:132-138).
    ///
    /// @param doc An stOTTR document, as a string.
    /// @returns The IRI of the first template the document defines.
    /// @export
    fn add_template_string(&self, doc: &str) -> savvy::Result<savvy::Sexp> {
        let mut inner = self.lock();
        let iri = inner
            .add_templates_from_string(doc)
            .map_err(maplib_error)?
            .ok_or_else(|| {
                argument_error("Template stOTTR document contained no templates")
            })?;
        iri.as_str().to_string().try_into()
    }

    /// Expand a template against a data.frame, adding the resulting triples
    /// to this Model (mirrors `map_mutex`'s data-driven branch,
    /// py_maplib/src/mutexes.rs:120-169 -- `Model::expand`,
    /// lib/maplib/src/model/expansion.rs:58).
    ///
    /// @param template_iri IRI of an already-registered template (see
    ///   `add_template`/`add_template_string`).
    /// @param stream_ptr A *filled* Arrow C Stream Interface pointer (e.g.
    ///   from `nanoarrow::as_nanoarrow_array_stream(df)`), one row per
    ///   template instantiation.
    /// @param graph Optional named graph IRI to add the resulting triples to
    ///   (default graph if NULL).
    /// @param validate_iris Whether to validate that IRI-typed columns
    ///   contain valid IRIs. Defaults to TRUE, matching py_maplib.
    /// @export
    fn map(
        &self,
        template_iri: &str,
        stream_ptr: Sexp,
        graph: Option<&str>,
        validate_iris: Option<bool>,
    ) -> savvy::Result<()> {
        let df = crate::arrow_bridge::import_dataframe(stream_ptr)?;
        if df.height() == 0 {
            // Matches py_maplib's own map_mutex (py_maplib/src/mutexes.rs:
            // 158-162): an empty-but-present DataFrame is a silent no-op,
            // not the same as no DataFrame at all -- expand(None) assumes a
            // signature with no variables, which isn't true here.
            return Ok(());
        }
        let named_graph = parse_optional_named_graph(graph)?;
        let options = MapOptions::from_args(named_graph, validate_iris);
        let mut inner = self.lock();
        inner
            .expand(template_iri, Some(df), None, options)
            .map_err(maplib_error)
    }

    /// Expand a template with no input data.frame -- for templates whose
    /// instances are entirely made of constant terms (mirrors `map_mutex`'s
    /// `df.is_none()` branch, py_maplib/src/mutexes.rs:163-168).
    ///
    /// @param template_iri IRI of an already-registered template.
    /// @param graph Optional named graph IRI to add the resulting triples to
    ///   (default graph if NULL).
    /// @param validate_iris Whether to validate that IRI-typed columns
    ///   contain valid IRIs. Defaults to TRUE.
    /// @export
    fn map_no_data(
        &self,
        template_iri: &str,
        graph: Option<&str>,
        validate_iris: Option<bool>,
    ) -> savvy::Result<()> {
        let named_graph = parse_optional_named_graph(graph)?;
        let options = MapOptions::from_args(named_graph, validate_iris);
        let mut inner = self.lock();
        inner
            .expand(template_iri, None, None, options)
            .map_err(maplib_error)
    }

    /// Load a Model previously written by serialize() (mirrors
    /// PyModel::deserialize, py_maplib/src/py_model.rs:936-954). This is an
    /// associated function, not a method -- call as `RModel$deserialize(path)`.
    ///
    /// @param path Directory previously written by serialize(). Defaults to
    ///   "./serialized_triples", matching py_maplib.
    /// @param storage_folder Optional folder for on-disk (rather than
    ///   in-memory) triplestore storage.
    /// @export
    fn deserialize(path: Option<&str>, storage_folder: Option<&str>) -> savvy::Result<RModel> {
        let model = maplib::model::Model::deserialize_triples(
            Path::new(path.unwrap_or("./serialized_triples")),
            storage_folder.map(String::from),
        )
        .map_err(maplib_error)?;
        Ok(RModel {
            inner: Mutex::new(model),
        })
    }
}
