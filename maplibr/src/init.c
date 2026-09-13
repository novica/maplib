
// clang-format sorts includes unless SortIncludes: Never. However, the ordering
// does matter here. So, we need to disable clang-format for safety.

// clang-format off
#include <stdint.h>
#include <Rinternals.h>
#include <R_ext/Parse.h>
// clang-format on

#include "rust/api.h"

static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;

SEXP handle_result(SEXP res_) {
    uintptr_t res = (uintptr_t)res_;

    // An error is indicated by tag.
    if ((res & TAGGED_POINTER_MASK) == 1) {
        // Remove tag
        SEXP res_aligned = (SEXP)(res & ~TAGGED_POINTER_MASK);

        // Currently, there are two types of error cases:
        //
        //   1. Error from Rust code
        //   2. Error from R's C API, which is caught by R_UnwindProtect()
        //
        if (TYPEOF(res_aligned) == CHARSXP) {
            // In case 1, the result is an error message that can be passed to
            // Rf_errorcall() directly.
            Rf_errorcall(R_NilValue, "%s", CHAR(res_aligned));
        } else {
            // In case 2, the result is the token to restart the
            // cleanup process on R's side.
            R_ContinueUnwind(res_aligned);
        }
    }

    return (SEXP)res;
}

SEXP savvy_export_test_series__impl(SEXP c_arg__stream_ptr) {
    SEXP res = savvy_export_test_series__ffi(c_arg__stream_ptr);
    return handle_result(res);
}

SEXP savvy_export_test_solution_mappings__impl(SEXP c_arg__stream_ptr) {
    SEXP res = savvy_export_test_solution_mappings__ffi(c_arg__stream_ptr);
    return handle_result(res);
}

SEXP savvy_import_test_series__impl(SEXP c_arg__stream_ptr) {
    SEXP res = savvy_import_test_series__ffi(c_arg__stream_ptr);
    return handle_result(res);
}

SEXP savvy_import_test_solution_mappings__impl(SEXP c_arg__stream_ptr, SEXP c_arg__rdf_node_types_json) {
    SEXP res = savvy_import_test_solution_mappings__ffi(c_arg__stream_ptr, c_arg__rdf_node_types_json);
    return handle_result(res);
}

SEXP savvy_validate_blank_node_id__impl(SEXP c_arg__id) {
    SEXP res = savvy_validate_blank_node_id__ffi(c_arg__id);
    return handle_result(res);
}

SEXP savvy_validate_iri__impl(SEXP c_arg__iri) {
    SEXP res = savvy_validate_iri__ffi(c_arg__iri);
    return handle_result(res);
}

SEXP savvy_validate_variable_name__impl(SEXP c_arg__name) {
    SEXP res = savvy_validate_variable_name__ffi(c_arg__name);
    return handle_result(res);
}

SEXP savvy_RModel_add_graph__impl(SEXP self__, SEXP c_arg__other, SEXP c_arg__source_graph, SEXP c_arg__target_graph) {
    SEXP res = savvy_RModel_add_graph__ffi(self__, c_arg__other, c_arg__source_graph, c_arg__target_graph);
    return handle_result(res);
}

SEXP savvy_RModel_add_prefixes__impl(SEXP self__, SEXP c_arg__prefixes) {
    SEXP res = savvy_RModel_add_prefixes__ffi(self__, c_arg__prefixes);
    return handle_result(res);
}

SEXP savvy_RModel_compact__impl(SEXP self__) {
    SEXP res = savvy_RModel_compact__ffi(self__);
    return handle_result(res);
}

SEXP savvy_RModel_create_index__impl(SEXP self__) {
    SEXP res = savvy_RModel_create_index__ffi(self__);
    return handle_result(res);
}

SEXP savvy_RModel_deserialize__impl(SEXP c_arg__path, SEXP c_arg__storage_folder) {
    SEXP res = savvy_RModel_deserialize__ffi(c_arg__path, c_arg__storage_folder);
    return handle_result(res);
}

SEXP savvy_RModel_detach_graph__impl(SEXP self__, SEXP c_arg__preserve_name, SEXP c_arg__graph) {
    SEXP res = savvy_RModel_detach_graph__ffi(self__, c_arg__preserve_name, c_arg__graph);
    return handle_result(res);
}

SEXP savvy_RModel_infer_rdfs__impl(SEXP self__, SEXP c_arg__graph) {
    SEXP res = savvy_RModel_infer_rdfs__ffi(self__, c_arg__graph);
    return handle_result(res);
}

SEXP savvy_RModel_new__impl(void) {
    SEXP res = savvy_RModel_new__ffi();
    return handle_result(res);
}

SEXP savvy_RModel_reads__impl(SEXP self__, SEXP c_arg__s, SEXP c_arg__format, SEXP c_arg__graph) {
    SEXP res = savvy_RModel_reads__ffi(self__, c_arg__s, c_arg__format, c_arg__graph);
    return handle_result(res);
}

SEXP savvy_RModel_serialize__impl(SEXP self__, SEXP c_arg__path) {
    SEXP res = savvy_RModel_serialize__ffi(self__, c_arg__path);
    return handle_result(res);
}

SEXP savvy_RModel_size__impl(SEXP self__) {
    SEXP res = savvy_RModel_size__ffi(self__);
    return handle_result(res);
}

SEXP savvy_RModel_truncate_graph__impl(SEXP self__, SEXP c_arg__graph) {
    SEXP res = savvy_RModel_truncate_graph__ffi(self__, c_arg__graph);
    return handle_result(res);
}

SEXP savvy_RModel_writes__impl(SEXP self__, SEXP c_arg__format, SEXP c_arg__graph) {
    SEXP res = savvy_RModel_writes__ffi(self__, c_arg__format, c_arg__graph);
    return handle_result(res);
}


static const R_CallMethodDef CallEntries[] = {
    {"savvy_export_test_series__impl", (DL_FUNC) &savvy_export_test_series__impl, 1},
    {"savvy_export_test_solution_mappings__impl", (DL_FUNC) &savvy_export_test_solution_mappings__impl, 1},
    {"savvy_import_test_series__impl", (DL_FUNC) &savvy_import_test_series__impl, 1},
    {"savvy_import_test_solution_mappings__impl", (DL_FUNC) &savvy_import_test_solution_mappings__impl, 2},
    {"savvy_validate_blank_node_id__impl", (DL_FUNC) &savvy_validate_blank_node_id__impl, 1},
    {"savvy_validate_iri__impl", (DL_FUNC) &savvy_validate_iri__impl, 1},
    {"savvy_validate_variable_name__impl", (DL_FUNC) &savvy_validate_variable_name__impl, 1},
    {"savvy_RModel_add_graph__impl", (DL_FUNC) &savvy_RModel_add_graph__impl, 4},
    {"savvy_RModel_add_prefixes__impl", (DL_FUNC) &savvy_RModel_add_prefixes__impl, 2},
    {"savvy_RModel_compact__impl", (DL_FUNC) &savvy_RModel_compact__impl, 1},
    {"savvy_RModel_create_index__impl", (DL_FUNC) &savvy_RModel_create_index__impl, 1},
    {"savvy_RModel_deserialize__impl", (DL_FUNC) &savvy_RModel_deserialize__impl, 2},
    {"savvy_RModel_detach_graph__impl", (DL_FUNC) &savvy_RModel_detach_graph__impl, 3},
    {"savvy_RModel_infer_rdfs__impl", (DL_FUNC) &savvy_RModel_infer_rdfs__impl, 2},
    {"savvy_RModel_new__impl", (DL_FUNC) &savvy_RModel_new__impl, 0},
    {"savvy_RModel_reads__impl", (DL_FUNC) &savvy_RModel_reads__impl, 4},
    {"savvy_RModel_serialize__impl", (DL_FUNC) &savvy_RModel_serialize__impl, 2},
    {"savvy_RModel_size__impl", (DL_FUNC) &savvy_RModel_size__impl, 1},
    {"savvy_RModel_truncate_graph__impl", (DL_FUNC) &savvy_RModel_truncate_graph__impl, 2},
    {"savvy_RModel_writes__impl", (DL_FUNC) &savvy_RModel_writes__impl, 3},
    {NULL, NULL, 0}
};

void R_init_maplibr(DllInfo *dll) {
    R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols(dll, FALSE);

    // Functions for initialization, if any.

}
