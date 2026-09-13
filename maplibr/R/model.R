#' A maplib knowledge graph model
#'
#' `Model` is the user-facing wrapper around the underlying `RModel` savvy
#' object. It mirrors py_maplib's `Model` class: methods mutate the
#' underlying graph in place and are chainable (each mutating method
#' returns `self` invisibly).
#'
#' @export
Model <- R6::R6Class(
  "Model",
  public = list(
    #' @description
    #' Create a new, empty Model. Not used to wrap an existing `RModel`
    #' (see `Model$deserialize()` and `$detach_graph()` for that) -- `rmodel`
    #' is for internal use only.
    #' @param rmodel Internal use only.
    initialize = function(rmodel = NULL) {
      if (is.null(rmodel)) {
        rmodel <- .rethrow(RModel$new())
      } else if (!inherits(rmodel, "RModel")) {
        stop("`rmodel` must be an RModel object", call. = FALSE)
      }
      private$rmodel <- rmodel
    },

    #' @description Number of triples in the default graph.
    size = function() {
      private$rmodel$size()
    },

    #' @description Parse RDF triples from a string into this Model.
    #' @param s RDF data as a string.
    #' @param format One of "ntriples", "turtle", "xml" (rdf/xml).
    #' @param graph Optional named graph IRI to read into (default graph if NULL).
    reads = function(s, format, graph = NULL) {
      .rethrow(private$rmodel$reads(s, format, graph))
      invisible(self)
    },

    #' @description Serialize this Model's triples to a string.
    #' @param format One of "ntriples", "turtle", "xml" (rdf/xml). Defaults to "ntriples".
    #' @param graph Optional named graph IRI to write (default graph if NULL).
    writes = function(format = NULL, graph = NULL) {
      .rethrow(private$rmodel$writes(format, graph))
    },

    #' @description Build the default (non-FTS) indexes.
    create_index = function() {
      .rethrow(private$rmodel$create_index())
      invisible(self)
    },

    #' @description Remove all triples from a graph.
    #' @param graph Optional named graph IRI to truncate (default graph if NULL).
    truncate_graph = function(graph = NULL) {
      .rethrow(private$rmodel$truncate_graph(graph))
      invisible(self)
    },

    #' @description Add prefix -> IRI mappings, used when serializing.
    #' @param prefixes A named R list of single strings: names are prefixes,
    #'   values are IRIs, e.g. `list(ex = "http://example.org/")`.
    add_prefixes = function(prefixes) {
      .rethrow(private$rmodel$add_prefixes(prefixes))
      invisible(self)
    },

    #' @description Copy a graph from another Model's triplestore into this one.
    #' @param other Another Model to copy a graph from.
    #' @param source_graph Optional named graph IRI in `other` (default graph if NULL).
    #' @param target_graph Optional named graph IRI in this Model to copy into (default graph if NULL).
    add_graph = function(other, source_graph = NULL, target_graph = NULL) {
      if (!inherits(other, "Model")) {
        stop("`other` must be a Model object", call. = FALSE)
      }
      .rethrow(private$rmodel$add_graph(private$as_rmodel(other), source_graph, target_graph))
      invisible(self)
    },

    #' @description Split a graph out of this Model into a brand new, standalone Model.
    #' @param preserve_name Keep the graph's own name in the new Model rather
    #'   than moving it to the default graph there.
    #' @param graph Optional named graph IRI to detach (default graph if NULL).
    #' @return A new Model containing only the detached graph.
    detach_graph = function(preserve_name = FALSE, graph = NULL) {
      Model$new(.rethrow(private$rmodel$detach_graph(preserve_name, graph)))
    },

    #' @description Run a SPARQL SELECT query and return the result as a data.frame.
    #' Mirrors py_maplib's Model.query() (py_maplib/src/py_model.rs:344-387),
    #' but only the SELECT case -- CONSTRUCT queries are not yet supported
    #' (raises a maplibr_argument_error), see maplib-l2j.
    #' @param sparql The SPARQL query string.
    #' @param graph Optional named graph IRI to restrict the query to (searches
    #'   across the whole store if NULL).
    #' @param include_transient Whether to include transient (e.g. inferred)
    #'   triples in the query.
    #' @return A data.frame. A column whose values are always the same RDF
    #'   node type (the common case) comes back as a plain vector of that
    #'   type's underlying value (an IRI/blank node id/literal value as a
    #'   plain string, exactly as py_maplib returns by default). A column
    #'   whose RDF node type can vary by row instead comes back from Arrow as
    #'   one sub-column per possible type (only one of which is non-NA on any
    #'   given row) -- collapsed here into a single vector by taking
    #'   whichever sub-column is non-NA per row. The precise per-column RDF
    #'   node type information (which py_maplib exposes via
    #'   SolutionMappings.rdf_types) is attached as the data.frame's
    #'   `"rdf_node_types"` attribute instead of a richer typed object, parsed
    #'   from the underlying JSON side-channel (maplibr/src/rust/src/
    #'   arrow_bridge.rs) -- a full RDFType-based port of that accessor is
    #'   left for later.
    query = function(sparql, graph = NULL, include_transient = FALSE) {
      stream <- nanoarrow::nanoarrow_allocate_array_stream()
      rdf_node_types_json <- .rethrow(private$rmodel$query(sparql, stream, include_transient, graph))
      df <- as.data.frame(stream)
      df <- .collapse_multitype_columns(df)
      attr(df, "rdf_node_types") <- jsonlite::fromJSON(rdf_node_types_json, simplifyVector = FALSE)
      df
    },

    #' @description Run RDFS inference over a graph in place.
    #' @param graph Optional named graph IRI to infer over (default graph if NULL).
    #' @return The number of new triples inferred (a triple count, not a rule
    #'   count -- Triplestore::interesting_rdfs_rules sums per-rule
    #'   inserted-triple counts).
    infer_rdfs = function(graph = NULL) {
      .rethrow(private$rmodel$infer_rdfs(graph))
    },

    #' @description Compact on-disk storage.
    compact = function() {
      .rethrow(private$rmodel$compact())
      invisible(self)
    },

    #' @description Serialize this Model's triples to a directory in
    #' maplib's own compact on-disk format (not an RDF interchange format --
    #' use `$writes()` for that).
    #' @param path Directory to serialize into.
    serialize = function(path) {
      .rethrow(private$rmodel$serialize(path))
      invisible(self)
    },

    #' @description Print a short summary of this Model.
    #' @param ... Unused.
    print = function(...) {
      cat("<Model>", private$rmodel$size(), "triples (default graph)\n")
      invisible(self)
    }
  ),
  private = list(
    rmodel = NULL,
    as_rmodel = function(model) {
      model$.__enclos_env__$private$rmodel
    }
  )
)

# A column whose RDF node type varies by row comes back from
# as.data.frame(stream) as a nested data.frame -- one sub-column per
# possible type (e.g. "I" for IRI, "B" for blank node, the datatype IRI for
# a literal; see MULTI_IRI_DT/MULTI_BLANK_DT, lib/representation/src/
# multitype.rs:11-13), with exactly one non-NA per row. Collapses each such
# column into a single vector by coalescing across its sub-columns via
# ifelse(), which is safe today only because every sub-column is plain
# character (IRI/blank-node id/literal all surface as strings via
# format_native_columns) -- ifelse() silently coerces on a type mismatch, so
# if a future multi-typed column ever carried a non-character sub-column
# (e.g. a natively-typed numeric literal), values could be silently
# mangled. Revisit with an explicit type check (or vctrs::vec_coalesce) if
# native-typed literal columns are ever added.
.collapse_multitype_columns <- function(df) {
  for (col in names(df)) {
    if (is.data.frame(df[[col]])) {
      sub <- df[[col]]
      collapsed <- sub[[1]]
      if (ncol(sub) > 1) {
        for (j in 2:ncol(sub)) {
          collapsed <- ifelse(is.na(collapsed), sub[[j]], collapsed)
        }
      }
      df[[col]] <- collapsed
    }
  }
  df
}

# Load a Model previously written by `$serialize()`. Attached directly to
# the (already @export'd) Model generator, so users call it as
# `Model$deserialize(path)` -- mirrors RModel's own associated-function
# pattern in 000-wrappers.R.
Model$deserialize <- function(path, storage_folder = NULL) {
  Model$new(.rethrow(RModel$deserialize(path, storage_folder)))
}
