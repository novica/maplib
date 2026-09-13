# Ports test_rdfs_inference.py (py_maplib/tests/test_rdfs_inference.py)
# against the same fixture data (tests/testthat/testdata/rdfs/). An earlier
# version of this file judged this unportable without writes() gaining an
# include_transient option -- that turned out to be a misdiagnosis: the
# py_maplib test itself only ever uses query(), which already supports
# include_transient (maplib-l2j's query() port). Now genuinely portable.

read_fixture <- function(path) paste(readLines(path, warn = FALSE), collapse = "\n")

rdfs_folders <- file.path(
  test_path("testdata", "rdfs"),
  c("rdfs2", "rdfs3", "rdfs5", "rdfs6", "rdfs8", "rdfs9", "rdfs10", "rdfs11")
)

for (dir in rdfs_folders) {
  test_that(paste0("infer_rdfs() matches the expected fixture (", basename(dir), ")"), {
    m_input <- Model$new()
    m_input$reads(read_fixture(file.path(dir, "input.ttl")), format = "turtle")
    m_input$infer_rdfs()

    m_expected <- Model$new()
    m_expected$reads(read_fixture(file.path(dir, "expected.ttl")), format = "turtle")

    order_rows <- function(df) df[do.call(order, df), , drop = FALSE]
    df_inferred <- order_rows(m_input$query(
      "SELECT * WHERE { ?a ?b ?c }",
      include_transient = TRUE
    ))
    df_expected <- order_rows(m_expected$query("SELECT * WHERE { ?a ?b ?c }"))

    rownames(df_inferred) <- NULL
    rownames(df_expected) <- NULL
    attr(df_inferred, "rdf_node_types") <- NULL
    attr(df_expected, "rdf_node_types") <- NULL
    expect_equal(df_inferred, df_expected)
  })
}

test_that("writes(include_transient = TRUE) serializes inferred triples", {
  # Confirms the fix for the gap this file used to work around: writes()
  # used to never serialize transient/inferred triples at all, regardless
  # of any flag, because Triplestore::write_triples had no include_transient
  # parameter (unlike the query path).
  dir <- test_path("testdata", "rdfs", "rdfs2")
  m <- Model$new()
  m$reads(read_fixture(file.path(dir, "input.ttl")), format = "turtle")
  n <- m$infer_rdfs()
  expect_gt(n, 0)

  without <- m$writes(format = "ntriples")
  with_transient <- m$writes(format = "ntriples", include_transient = TRUE)
  expect_gt(nchar(with_transient), nchar(without))

  # rdfs2's fixture: "moon rdfs:domain Planet" + "Mars moon Phobos" entails
  # "Mars rdf:type Planet", which only the include_transient = TRUE output
  # should contain.
  expect_false(grepl("22-rdf-syntax-ns#type", without, fixed = TRUE))
  expect_match(with_transient, "22-rdf-syntax-ns#type", fixed = TRUE)
})

test_that("writes(include_transient = TRUE, format = \"turtle\") errors clearly", {
  # Pretty-Turtle output doesn't merge in transient triples yet -- refuses
  # rather than silently dropping them.
  dir <- test_path("testdata", "rdfs", "rdfs2")
  m <- Model$new()
  m$reads(read_fixture(file.path(dir, "input.ttl")), format = "turtle")
  m$infer_rdfs()
  expect_error(m$writes(format = "turtle", include_transient = TRUE))
})
