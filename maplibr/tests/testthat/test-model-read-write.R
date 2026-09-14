# Tests for Model$read()/Model$write() (maplib-5us) -- file-based
# counterparts to $reads()/$writes().

test_that("write() then read() round-trips triples through a file, explicit format", {
  m <- Model$new()
  m$reads('<http://ex/a> <http://ex/p> <http://ex/o> .', format = "ntriples")

  path <- tempfile(fileext = ".nt")
  on.exit(unlink(path))
  m$write(path, format = "ntriples")

  m2 <- Model$new()
  m2$read(path, format = "ntriples")
  expect_equal(m2$size(), 1)
  df <- m2$query("SELECT ?s ?p ?o WHERE { ?s ?p ?o }")
  expect_equal(df$s, "http://ex/a")
})

test_that("write() defaults to N-Triples format", {
  m <- Model$new()
  m$reads('<http://ex/a> <http://ex/p> <http://ex/o> .', format = "ntriples")

  path <- tempfile(fileext = ".nt")
  on.exit(unlink(path))
  m$write(path)
  expect_match(readLines(path), "http://ex/a", fixed = TRUE, all = FALSE)
})

test_that("read() guesses the format from the file extension when format is NULL", {
  m <- Model$new()
  m$reads('<http://ex/a> <http://ex/p> <http://ex/o> .', format = "ntriples")
  path <- tempfile(fileext = ".nt")
  on.exit(unlink(path))
  m$write(path, format = "ntriples")

  m2 <- Model$new()
  m2$read(path)
  expect_equal(m2$size(), 1)
})

test_that("read()/write()'s graph argument scopes to a single named graph", {
  m <- Model$new()
  m$reads('<http://ex/a> <http://ex/p> <http://ex/o> .', format = "ntriples", graph = "http://ex/g1")

  path <- tempfile(fileext = ".nt")
  on.exit(unlink(path))
  m$write(path, graph = "http://ex/g1")

  m2 <- Model$new()
  m2$read(path, graph = "http://ex/g2")
  expect_equal(m2$size(graph = "http://ex/g2"), 1)
  expect_equal(m2$size(), 0)
})

test_that("read() raises a catchable error for a nonexistent file", {
  m <- Model$new()
  expect_error(
    m$read(tempfile(fileext = ".nt")),
    class = "maplibr_maplib_error"
  )
})
