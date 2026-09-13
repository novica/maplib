# Attempted to port py_maplib/tests/test_rdfs_inference.py against the same
# fixture data (still kept in tests/testthat/testdata/rdfs/, in case a
# future query()-based port wants it), comparing writes()-serialized
# N-Triples between an inferred Model and a fixture "expected.ttl" Model.
#
# That approach turned out to be unsound, not just blocked by query() not
# existing yet: confirmed live that Model$writes() never serializes
# transient/inferred triples at all. infer_rdfs() reports a positive count
# (e.g. 1 for rdfs2's fixture), but the newly-inferred triple never shows up
# in writes() output -- Triplestore::write_triples (lib/maplib/src/model.rs)
# has no include_transient parameter to opt into seeing it, unlike
# get_predicate_eager_solution_mappings/query-path methods, which do. So a
# writes()-based comparison can never observe what infer_rdfs() actually
# did, regardless of whether the entailment itself is correct -- this
# needed query() (or write_triples gaining an include_transient option) to
# test properly, same blocker as the rest of maplib-a2d.
#
# Left as a smoke test only: infer_rdfs() must run without erroring and
# report a plausible (non-negative) count on real RDFS input.

test_that("infer_rdfs() runs without erroring and returns a plausible count", {
  read_fixture <- function(path) paste(readLines(path, warn = FALSE), collapse = "\n")

  dir <- test_path("testdata", "rdfs", "rdfs2")
  m <- Model$new()
  m$reads(read_fixture(file.path(dir, "input.ttl")), format = "turtle")

  n <- m$infer_rdfs()
  expect_true(is.numeric(n))
  expect_gte(n, 0)
})
