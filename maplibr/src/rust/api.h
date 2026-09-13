SEXP savvy_export_test_series__ffi(SEXP c_arg__stream_ptr);
SEXP savvy_export_test_solution_mappings__ffi(SEXP c_arg__stream_ptr);
SEXP savvy_import_test_series__ffi(SEXP c_arg__stream_ptr);
SEXP savvy_import_test_solution_mappings__ffi(SEXP c_arg__stream_ptr, SEXP c_arg__rdf_node_types_json);
SEXP savvy_validate_blank_node_id__ffi(SEXP c_arg__id);
SEXP savvy_validate_iri__ffi(SEXP c_arg__iri);
SEXP savvy_validate_variable_name__ffi(SEXP c_arg__name);

// methods and associated functions for RModel
SEXP savvy_RModel_add_graph__ffi(SEXP self__, SEXP c_arg__other, SEXP c_arg__source_graph, SEXP c_arg__target_graph);
SEXP savvy_RModel_add_prefixes__ffi(SEXP self__, SEXP c_arg__prefixes);
SEXP savvy_RModel_compact__ffi(SEXP self__);
SEXP savvy_RModel_create_index__ffi(SEXP self__);
SEXP savvy_RModel_deserialize__ffi(SEXP c_arg__path, SEXP c_arg__storage_folder);
SEXP savvy_RModel_detach_graph__ffi(SEXP self__, SEXP c_arg__preserve_name, SEXP c_arg__graph);
SEXP savvy_RModel_infer_rdfs__ffi(SEXP self__, SEXP c_arg__graph);
SEXP savvy_RModel_new__ffi(void);
SEXP savvy_RModel_reads__ffi(SEXP self__, SEXP c_arg__s, SEXP c_arg__format, SEXP c_arg__graph);
SEXP savvy_RModel_serialize__ffi(SEXP self__, SEXP c_arg__path);
SEXP savvy_RModel_size__ffi(SEXP self__);
SEXP savvy_RModel_truncate_graph__ffi(SEXP self__, SEXP c_arg__graph);
SEXP savvy_RModel_writes__ffi(SEXP self__, SEXP c_arg__format, SEXP c_arg__graph);
