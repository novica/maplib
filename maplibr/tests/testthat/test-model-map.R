# Tests for Model$map()/Model$add_template() (maplib-cgd). Ports the subset
# of py_maplib/tests/test_basics.py that only needs map()/add_template() plus
# query() (already available, maplib-l2j) -- OTTR features query() itself
# doesn't cover yet (CONSTRUCT, streaming=) are out of scope here.

test_that("add_template(doc) registers a template usable by IRI in map()", {
  # Ports test_add_template_instead_of_constructor_df (py_maplib/tests/
  # test_basics.py:43-54).
  doc <- '
  @prefix ex:<http://example.net/ns#>.
  ex:ExampleTemplate [?MyValue] :: {
    ottr:Triple(ex:myObject, ex:hasValue, ?MyValue)
  } .
  '
  m <- Model$new()
  iri <- m$add_template(doc)
  expect_equal(iri, "http://example.net/ns#ExampleTemplate")

  m$map("http://example.net/ns#ExampleTemplate", data.frame(MyValue = "A"))
  df <- m$query("SELECT ?o WHERE { <http://example.net/ns#myObject> <http://example.net/ns#hasValue> ?o }")
  expect_equal(df$o, "A")
})

test_that("map() accepts a Template object directly, with an optional missing parameter", {
  # Ports test_create_programmatic_model_with_optional_value_missing_df
  # (py_maplib/tests/test_basics.py:101-145), sans the streaming= parametrization
  # (query() has no streaming argument to port).
  ex <- Prefix("http://example.net/ns#", "ex")
  my_value <- Variable("MyValue")
  my_other_value <- Variable("MyOtherValue")
  my_object <- suf(ex, "MyObject")
  template <- Template(
    iri = suf(ex, "ExampleTemplate"),
    parameters = list(Parameter(my_value), Parameter(my_other_value, optional = TRUE)),
    instances = list(
      Triple(my_object, suf(ex, "hasValue"), my_value),
      Triple(my_object, suf(ex, "hasOtherValue"), my_other_value)
    )
  )
  m <- Model$new()
  m$map(template, data.frame(MyValue = "A"))

  df <- m$query("PREFIX ex:<http://example.net/ns#> SELECT ?A WHERE { ?obj1 ex:hasValue ?A }")
  expect_equal(df$A, "A")

  df2 <- m$query("PREFIX ex:<http://example.net/ns#> SELECT ?A WHERE { ?obj1 ex:hasOtherValue ?A }")
  expect_equal(nrow(df2), 0)
})

test_that("map() with no data.frame expands a constant-only template once", {
  doc <- '
  @prefix ex:<http://example.net/ns#>.
  ex:StaticTemplate [] :: {
    ottr:Triple(ex:a, ex:b, ex:c)
  } .
  '
  m <- Model$new()
  m$add_template(doc)
  m$map("http://example.net/ns#StaticTemplate")
  expect_equal(m$size(), 1)
})

test_that("map() with a zero-row data.frame is a no-op", {
  # Ports test_create_model_from_empty_polars_df (py_maplib/tests/
  # test_basics.py:24-35).
  doc <- '
  @prefix ex:<http://example.net/ns#>.
  ex:ExampleTemplate [?MyValue] :: {
    ottr:Triple(ex:myObject, ex:hasValue, ?MyValue)
  } .
  '
  m <- Model$new()
  m$add_template(doc)
  m$map("http://example.net/ns#ExampleTemplate", data.frame(MyValue = character(0)))
  expect_equal(m$size(), 0)
})

test_that("add_template()/map() reject an unrecognized `template` argument", {
  m <- Model$new()
  expect_error(m$add_template(42), "Template")
  expect_error(m$map(42), "Template")
})
