# Ports the non-query subset of py_maplib/tests/test_basics.py. Most of
# that file's assertions go through Model.query()/Model.map(), neither of
# which exist in maplibr yet (see maplib-a2d's close notes) -- only the
# handful of assertions reachable without them are ported here.

test_that("writes() on an empty Model returns without erroring", {
  # Mirrors test_write_empty_model (py_maplib/tests/test_basics.py:20-22).
  m <- Model$new()
  s <- m$writes()
  expect_type(s, "character")
})

test_that("reads()/writes() round-trip and chain", {
  m <- Model$new()
  result <- m$reads('<http://a> <http://b> "c" .', format = "ntriples")
  expect_identical(result, m) # chainable, returns self invisibly

  expect_equal(m$size(), 1)
  expect_match(m$writes(format = "ntriples"), "<http://a>", fixed = TRUE)
})

test_that("create_index() is chainable and doesn't error on a populated Model", {
  m <- Model$new()
  m$reads('<http://a> <http://b> "c" .', format = "ntriples")
  result <- m$create_index()
  expect_identical(result, m)
})

test_that("truncate_graph() empties the default graph", {
  m <- Model$new()
  m$reads('<http://a> <http://b> "c" .', format = "ntriples")
  expect_equal(m$size(), 1)
  m$truncate_graph()
  expect_equal(m$size(), 0)
})

test_that("add_prefixes() affects writes() output", {
  m <- Model$new()
  m$reads('<http://example.org/a> <http://example.org/b> <http://example.org/c> .', format = "ntriples")
  m$add_prefixes(list(ex = "http://example.org/"))
  out <- m$writes(format = "turtle")
  expect_match(out, "@prefix ex: <http://example.org/>", fixed = TRUE)
})
