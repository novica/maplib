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
        rmodel <- RModel$new()
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
      private$rmodel$reads(s, format, graph)
      invisible(self)
    },

    #' @description Serialize this Model's triples to a string.
    #' @param format One of "ntriples", "turtle", "xml" (rdf/xml). Defaults to "ntriples".
    #' @param graph Optional named graph IRI to write (default graph if NULL).
    writes = function(format = NULL, graph = NULL) {
      private$rmodel$writes(format, graph)
    },

    #' @description Build the default (non-FTS) indexes.
    create_index = function() {
      private$rmodel$create_index()
      invisible(self)
    },

    #' @description Remove all triples from a graph.
    #' @param graph Optional named graph IRI to truncate (default graph if NULL).
    truncate_graph = function(graph = NULL) {
      private$rmodel$truncate_graph(graph)
      invisible(self)
    },

    #' @description Add prefix -> IRI mappings, used when serializing.
    #' @param prefixes A named R list of single strings: names are prefixes,
    #'   values are IRIs, e.g. `list(ex = "http://example.org/")`.
    add_prefixes = function(prefixes) {
      private$rmodel$add_prefixes(prefixes)
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
      private$rmodel$add_graph(private$as_rmodel(other), source_graph, target_graph)
      invisible(self)
    },

    #' @description Split a graph out of this Model into a brand new, standalone Model.
    #' @param preserve_name Keep the graph's own name in the new Model rather
    #'   than moving it to the default graph there.
    #' @param graph Optional named graph IRI to detach (default graph if NULL).
    #' @return A new Model containing only the detached graph.
    detach_graph = function(preserve_name = FALSE, graph = NULL) {
      Model$new(private$rmodel$detach_graph(preserve_name, graph))
    },

    #' @description Run RDFS inference over a graph in place.
    #' @param graph Optional named graph IRI to infer over (default graph if NULL).
    #' @return The number of interesting inference rules applied.
    infer_rdfs = function(graph = NULL) {
      private$rmodel$infer_rdfs(graph)
    },

    #' @description Compact on-disk storage.
    compact = function() {
      private$rmodel$compact()
      invisible(self)
    },

    #' @description Serialize this Model's triples to a directory in
    #' maplib's own compact on-disk format (not an RDF interchange format --
    #' use `$writes()` for that).
    #' @param path Directory to serialize into.
    serialize = function(path) {
      private$rmodel$serialize(path)
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

# Load a Model previously written by `$serialize()`. Attached directly to
# the (already @export'd) Model generator, so users call it as
# `Model$deserialize(path)` -- mirrors RModel's own associated-function
# pattern in 000-wrappers.R.
Model$deserialize <- function(path, storage_folder = NULL) {
  Model$new(RModel$deserialize(path, storage_folder))
}
