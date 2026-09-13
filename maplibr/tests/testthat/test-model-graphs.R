# Ports the non-query subset of py_maplib/tests/test_named_graphs.py.
# Those tests confirm named-graph routing via query() and a per-graph
# m.size(graph) that RModel$size() doesn't support yet (it always counts the
# default graph -- see model.rs; a real gap, not something to route around
# here). Routing is instead confirmed via writes(graph=), which does take a
# graph argument, without needing query() at all.

gr1 <- '<http://example.net/ns#myObject> <http://example.net/ns#hasValue> "A" .'
gr2 <- '<http://example.net/ns#myObject> <http://example.net/ns#hasValue> "B" .'

test_that("reads() routes triples into the requested named graph", {
  # Mirrors test_basic_named_graph (py_maplib/tests/test_named_graphs.py:17-30).
  ng2 <- "urn:graph:gr2"
  m <- Model$new()
  m$reads(gr1, format = "ntriples")
  m$reads(gr2, format = "ntriples", graph = ng2)

  expect_equal(m$size(), 1) # default graph only has gr1's triple
  expect_match(m$writes(format = "ntriples", graph = ng2), '"B"', fixed = TRUE)
  expect_false(grepl('"B"', m$writes(format = "ntriples"), fixed = TRUE))
})

test_that("add_graph() copies a named graph's triples into another Model", {
  m1 <- Model$new()
  m1$reads(gr1, format = "ntriples")

  m2 <- Model$new()
  m2$add_graph(m1)
  expect_equal(m2$size(), 1)
  expect_match(m2$writes(format = "ntriples"), '"A"', fixed = TRUE)
})

test_that("detach_graph() splits a graph into a new, standalone Model", {
  ng2 <- "urn:graph:gr2"
  m <- Model$new()
  m$reads(gr1, format = "ntriples")
  m$reads(gr2, format = "ntriples", graph = ng2)

  detached <- m$detach_graph(graph = ng2)
  expect_true(inherits(detached, "Model"))
  expect_equal(detached$size(), 1)
  expect_match(detached$writes(format = "ntriples"), '"B"', fixed = TRUE)
})
