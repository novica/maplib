SEXP savvy_export_test_series__ffi(SEXP c_arg__stream_ptr);
SEXP savvy_import_test_series__ffi(SEXP c_arg__stream_ptr);

// methods and associated functions for RModel
SEXP savvy_RModel_add_prefixes__ffi(SEXP self__, SEXP c_arg__prefixes);
SEXP savvy_RModel_create_index__ffi(SEXP self__);
SEXP savvy_RModel_new__ffi(void);
SEXP savvy_RModel_reads__ffi(SEXP self__, SEXP c_arg__s, SEXP c_arg__format, SEXP c_arg__graph);
SEXP savvy_RModel_size__ffi(SEXP self__);
SEXP savvy_RModel_truncate_graph__ffi(SEXP self__, SEXP c_arg__graph);
SEXP savvy_RModel_writes__ffi(SEXP self__, SEXP c_arg__format, SEXP c_arg__graph);
