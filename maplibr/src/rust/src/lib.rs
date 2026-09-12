
use oxrdf::NamedNode;
use oxrdfio::RdfFormat;
use polars::prelude::*;
use polars_arrow::ffi::{export_iterator, ArrowArrayStream, ArrowArrayStreamReader};
use representation::dataset::NamedGraph;
use savvy::{savvy, ExternalPointerSexp, Sexp};
use std::collections::HashMap;
use std::sync::Mutex;
use triplestore::triples_read::ExtendedRdfFormat;

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
struct RModel {
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
}

/// Build a small hardcoded i32 Series in Rust and stream it out to R
/// through the Arrow C Stream Interface.
///
/// @param stream_ptr An external pointer from `nanoarrow::nanoarrow_allocate_array_stream()`.
/// @export
#[savvy]
fn export_test_series(stream_ptr: Sexp) -> savvy::Result<()> {
    let series = Series::new("x".into(), &[1i32, 2, 3, 4, 5]);

    let field = series.field().to_arrow(CompatLevel::newest());
    let chunks = series.chunks().clone();
    let iter = Box::new(chunks.into_iter().map(Ok));
    let stream = export_iterator(iter, field);

    let stream_ptr = unsafe {
        ExternalPointerSexp::try_from(stream_ptr)?.cast_mut_unchecked::<ArrowArrayStream>()
    };
    unsafe { std::ptr::replace(stream_ptr, stream) };

    Ok(())
}

/// Read an Arrow C stream from R back into a Rust Series, then hand back
/// its length and first value as a sanity check (i32 only, for this prototype).
///
/// @param stream_ptr An external pointer holding a filled ArrowArrayStream.
/// @returns An integer vector: `c(length, first_value)`.
/// @export
#[savvy]
fn import_test_series(stream_ptr: Sexp) -> savvy::Result<Sexp> {
    let mut stream = unsafe {
        let boxed = Box::new(std::ptr::replace(
            ExternalPointerSexp::try_from(stream_ptr)?.cast_mut_unchecked::<ArrowArrayStream>(),
            ArrowArrayStream::empty(),
        ));
        ArrowArrayStreamReader::try_new(boxed).map_err(|e| savvy::Error::new(&e.to_string()))?
    };

    let mut arrays = Vec::new();
    while let Some(arr) = unsafe { stream.next() } {
        arrays.push(arr.map_err(|e| savvy::Error::new(&e.to_string()))?);
    }

    let series = Series::from_arrow_chunks("x".into(), arrays)
        .map_err(|e| savvy::Error::new(&e.to_string()))?;

    let mut out = savvy::OwnedIntegerSexp::new(2)?;
    out[0] = series.len() as i32;
    out[1] = series
        .i32()
        .map_err(|e| savvy::Error::new(&e.to_string()))?
        .get(0)
        .unwrap_or(0);
    Ok(out.into())
}
