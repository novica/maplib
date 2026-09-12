use oxrdf::NamedNode;
use oxrdfio::RdfFormat;
use representation::dataset::NamedGraph;
use savvy::{savvy, ListSexp};
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
        _ => Err(savvy::Error::new(&format!("Unknown format: {}", format))),
    }
}

/// Mirrors py_maplib's `resolve_format` (py_maplib/src/lib.rs:318-327).
/// CIM XML and HDT deliberately unsupported here (out of scope for v1, see plan).
fn resolve_format(format: &str) -> savvy::Result<ExtendedRdfFormat> {
    resolve_normal_format(format).map(ExtendedRdfFormat::Normal)
}

fn parse_optional_named_graph(graph: Option<&str>) -> savvy::Result<NamedGraph> {
    let nn = graph
        .map(|g| NamedNode::new(g).map_err(|e| savvy::Error::new(&e.to_string())))
        .transpose()?;
    Ok(NamedGraph::from_maybe_named_node(nn.as_ref()))
}

/// A maplib knowledge graph model.
///
/// @export
#[savvy]
pub struct RModel {
    inner: Mutex<maplib::model::Model>,
}

#[savvy]
impl RModel {
    /// Create a new, empty Model.
    ///
    /// @export
    fn new() -> savvy::Result<Self> {
        let model = maplib::model::Model::new(None, None, None, None)
            .map_err(|e| savvy::Error::new(&e.to_string()))?;
        Ok(Self {
            inner: Mutex::new(model),
        })
    }

    /// Number of triples in the default graph.
    ///
    /// @export
    fn size(&self) -> savvy::Result<savvy::Sexp> {
        let inner = self.inner.lock().unwrap();
        let graph = NamedGraph::from_maybe_named_node(None);
        (inner.graph_size(&graph) as i32).try_into()
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
        let mut inner = self.inner.lock().unwrap();
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
            .map_err(|e| savvy::Error::new(&e.to_string()))
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
        let mut inner = self.inner.lock().unwrap();
        let mut out = Vec::new();
        inner
            .write_triples(&mut out, &named_graph, format, None)
            .map_err(|e| savvy::Error::new(&e.to_string()))?;
        String::from_utf8(out)
            .map_err(|e| savvy::Error::new(&e.to_string()))?
            .try_into()
    }

    /// Build the default (non-FTS) indexes, matching py_maplib's create_index()
    /// with no options (create_index_mutex, py_maplib/src/mutexes.rs:402-413).
    ///
    /// @export
    fn create_index(&self) -> savvy::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .create_index(IndexingOptions::default())
            .map_err(|e| savvy::Error::new(&e.to_string()))
    }

    /// Remove all triples from a graph (mirrors truncate_graph_mutex,
    /// py_maplib/src/mutexes.rs:98-104 -- delegates straight to the
    /// triplestore, Model has no dedicated wrapper method).
    ///
    /// @param graph Optional named graph IRI to truncate (default graph if NULL).
    /// @export
    fn truncate_graph(&self, graph: Option<&str>) -> savvy::Result<()> {
        let named_graph = parse_optional_named_graph(graph)?;
        let mut inner = self.inner.lock().unwrap();
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
            let nn = NamedNode::new(iri).map_err(|e| savvy::Error::new(&e.to_string()))?;
            parsed.insert(name.to_string(), nn);
        }
        let mut inner = self.inner.lock().unwrap();
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
        let other_inner = other.inner.lock().unwrap();
        let mut inner = self.inner.lock().unwrap();
        inner
            .add_graph(&other_inner.triplestore, source_graph, target_graph)
            .map_err(|e| savvy::Error::new(&e.to_string()))
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
        let mut inner = self.inner.lock().unwrap();
        let sprout = inner
            .detach_graph(&named_graph, preserve_name)
            .map_err(|e| savvy::Error::new(&e.to_string()))?;
        Ok(RModel {
            inner: Mutex::new(sprout),
        })
    }

    /// Compact on-disk storage (mirrors PyModel::compact, py_maplib/src/py_model.rs:919-924).
    ///
    /// @export
    fn compact(&self) -> savvy::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.compact().map_err(|e| savvy::Error::new(&e.to_string()))
    }

    /// Serialize this Model's triples to a directory in maplib's own compact
    /// on-disk format (not an RDF interchange format -- use write()/writes()
    /// for that). Mirrors PyModel::serialize, py_maplib/src/py_model.rs:926-934.
    ///
    /// @param path Directory to serialize into.
    /// @export
    fn serialize(&self, path: &str) -> savvy::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .serialize_triples(Path::new(path))
            .map_err(|e| savvy::Error::new(&e.to_string()))
    }

    /// Load a Model previously written by serialize() (mirrors
    /// PyModel::deserialize, py_maplib/src/py_model.rs:936-954). This is an
    /// associated function, not a method -- call as `RModel$deserialize(path)`.
    ///
    /// @param path Directory previously written by serialize().
    /// @param storage_folder Optional folder for on-disk (rather than
    ///   in-memory) triplestore storage.
    /// @export
    fn deserialize(path: &str, storage_folder: Option<&str>) -> savvy::Result<RModel> {
        let model = maplib::model::Model::deserialize_triples(
            Path::new(path),
            storage_folder.map(String::from),
        )
        .map_err(|e| savvy::Error::new(&e.to_string()))?;
        Ok(RModel {
            inner: Mutex::new(model),
        })
    }
}
