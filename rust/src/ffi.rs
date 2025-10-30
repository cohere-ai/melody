//! FFI bindings for the melody parsing library for Go

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::slice;

use crate::filter::{Filter, FilterImpl};
use crate::options::{FilterOptions, new_filter};
use crate::types::*;

/// Opaque pointer to a Filter instance
#[repr(C)]
pub struct CFilter {
    _private: [u8; 0],
}

/// Opaque pointer to FilterOptions
#[repr(C)]
pub struct CFilterOptions {
    _private: [u8; 0],
}

/// C-compatible representation of FilterOutput
#[repr(C)]
pub struct CFilterOutput {
    pub text: *mut c_char,
    pub text_len: usize,

    pub token_ids: *mut u32,
    pub token_ids_len: usize,
    pub logprobs: *mut f32,
    pub logprobs_len: usize,

    pub search_query_index: i32, // -1 if None
    pub search_query_text: *mut c_char,

    pub citations: *mut CFilterCitation,
    pub citations_len: usize,

    pub tool_call_index: i32, // -1 if None
    pub tool_call_id: *mut c_char,
    pub tool_call_name: *mut c_char,
    pub tool_call_param_name: *mut c_char,
    pub tool_call_param_value_delta: *mut c_char,
    pub tool_call_raw_param_delta: *mut c_char,

    pub is_post_answer: bool,
    pub is_tools_reason: bool,
}

/// C-compatible representation of FilterCitation
#[repr(C)]
pub struct CFilterCitation {
    pub start_index: usize,
    pub end_index: usize,
    pub text: *mut c_char,
    pub sources: *mut CSource,
    pub sources_len: usize,
    pub is_thinking: bool,
}

/// C-compatible representation of Source
#[repr(C)]
pub struct CSource {
    pub tool_call_index: usize,
    pub tool_result_indices: *mut usize,
    pub tool_result_indices_len: usize,
}

/// C-compatible representation of an array of FilterOutput
#[repr(C)]
pub struct CFilterOutputArray {
    pub outputs: *mut CFilterOutput,
    pub len: usize,
}

// ============================================================================
// FilterOptions FFI functions
// ============================================================================

/// Creates a new FilterOptions instance
///
/// # Safety
/// The returned pointer must be freed with melody_filter_options_free
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_new() -> *mut CFilterOptions {
    let options = Box::new(FilterOptions::new());
    Box::into_raw(options) as *mut CFilterOptions
}

/// Frees a FilterOptions instance
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_free(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let _ = Box::from_raw(options as *mut FilterOptions);
        }
    }
}

/// Configures options for multi-hop CMD3 format
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_handle_multi_hop_cmd3(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            *opts = opts.clone().handle_multi_hop_cmd3();
        }
    }
}

/// Configures options for multi-hop CMD4 format
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_handle_multi_hop_cmd4(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            *opts = opts.clone().handle_multi_hop_cmd4();
        }
    }
}

/// Configures options for RAG format
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_handle_rag(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            *opts = opts.clone().handle_rag();
        }
    }
}

/// Configures options for search query format
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_handle_search_query(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            *opts = opts.clone().handle_search_query();
        }
    }
}

/// Configures options for multi-hop format
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_handle_multi_hop(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            *opts = opts.clone().handle_multi_hop();
        }
    }
}

/// Enables streaming of non-grounded answers
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_stream_non_grounded_answer(
    options: *mut CFilterOptions,
) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            *opts = opts.clone().stream_non_grounded_answer();
        }
    }
}

/// Enables streaming of tool actions
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_stream_tool_actions(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            *opts = opts.clone().stream_tool_actions();
        }
    }
}

/// Enables streaming of processed parameters
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_stream_processed_params(
    options: *mut CFilterOptions,
) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            *opts = opts.clone().stream_processed_params();
        }
    }
}

/// Sets left trimming
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_with_left_trimmed(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            *opts = opts.clone().with_left_trimmed();
        }
    }
}

/// Sets right trimming
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_with_right_trimmed(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            *opts = opts.clone().with_right_trimmed();
        }
    }
}

/// Sets prefix trim
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
/// prefix must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_with_prefix_trim(
    options: *mut CFilterOptions,
    prefix: *const c_char,
) {
    if !options.is_null() && !prefix.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            let prefix_str = CStr::from_ptr(prefix).to_string_lossy().into_owned();
            *opts = opts.clone().with_prefix_trim(prefix_str);
        }
    }
}

/// Sets chunk size
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_with_chunk_size(
    options: *mut CFilterOptions,
    size: usize,
) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            *opts = opts.clone().with_chunk_size(size);
        }
    }
}

/// Adds inclusive stops
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
/// stops must be valid null-terminated C strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_with_inclusive_stops(
    options: *mut CFilterOptions,
    stops: *const *const c_char,
    stops_len: usize,
) {
    if !options.is_null() && !stops.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            let stops_slice = slice::from_raw_parts(stops, stops_len);
            let stop_strings: Vec<String> = stops_slice
                .iter()
                .map(|&s| CStr::from_ptr(s).to_string_lossy().into_owned())
                .collect();
            *opts = opts.clone().with_inclusive_stops(stop_strings);
        }
    }
}

/// Adds exclusive stops
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
/// stops must be valid null-terminated C strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_with_exclusive_stops(
    options: *mut CFilterOptions,
    stops: *const *const c_char,
    stops_len: usize,
) {
    if !options.is_null() && !stops.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            let stops_slice = slice::from_raw_parts(stops, stops_len);
            let stop_strings: Vec<String> = stops_slice
                .iter()
                .map(|&s| CStr::from_ptr(s).to_string_lossy().into_owned())
                .collect();
            *opts = opts.clone().with_exclusive_stops(stop_strings);
        }
    }
}

/// Removes a token from the special token map
///
/// # Safety
/// options must be a valid pointer returned from melody_filter_options_new
/// token must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_remove_token(
    options: *mut CFilterOptions,
    token: *const c_char,
) {
    if !options.is_null() && !token.is_null() {
        unsafe {
            let opts = &mut *(options as *mut FilterOptions);
            let token_str = CStr::from_ptr(token).to_string_lossy().into_owned();
            *opts = opts.clone().remove_token(token_str);
        }
    }
}

// ============================================================================
// Filter FFI functions
// ============================================================================

/// Creates a new filter with the given options
///
/// # Safety
/// - options can be null for default options, or must be a valid pointer from melody_filter_options_new
/// - The returned pointer must be freed with melody_filter_free
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_new(options: *const CFilterOptions) -> *mut CFilter {
    unsafe {
        let filter = if options.is_null() {
            Box::new(FilterImpl::new())
        } else {
            let opts = &*(options as *const FilterOptions);
            Box::new(new_filter(opts.clone()))
        };
        Box::into_raw(filter) as *mut CFilter
    }
}

/// Frees a filter instance
///
/// # Safety
/// filter must be a valid pointer returned from melody_filter_new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_free(filter: *mut CFilter) {
    if !filter.is_null() {
        unsafe {
            let _ = Box::from_raw(filter as *mut FilterImpl);
        }
    }
}

/// Writes a decoded token to the filter
///
/// # Safety
/// - filter must be a valid pointer returned from melody_filter_new
/// - decoded_token must be a valid null-terminated C string
/// - The returned CFilterOutputArray must be freed with melody_filter_output_array_free
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_write_decoded(
    filter: *mut CFilter,
    decoded_token: *const c_char,
    token_ids: *const u32,
    token_ids_len: usize,
    logprobs: *const f32,
    logprobs_len: usize,
) -> *mut CFilterOutputArray {
    if filter.is_null() || decoded_token.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        let filter = &mut *(filter as *mut FilterImpl);
        let token_str = CStr::from_ptr(decoded_token).to_string_lossy();

        let token_ids_vec = if !token_ids.is_null() && token_ids_len > 0 {
            slice::from_raw_parts(token_ids, token_ids_len).to_vec()
        } else {
            Vec::new()
        };

        let logprobs_vec = if !logprobs.is_null() && logprobs_len > 0 {
            slice::from_raw_parts(logprobs, logprobs_len).to_vec()
        } else {
            Vec::new()
        };

        let log_prob = TokenIDsWithLogProb {
            token_ids: token_ids_vec,
            logprobs: logprobs_vec,
        };

        let outputs = filter.write_decoded(&token_str, log_prob);
        convert_outputs_to_c(outputs)
    }
}

/// Flushes any partial outputs from the filter
///
/// # Safety
/// - filter must be a valid pointer returned from melody_filter_new
/// - The returned CFilterOutputArray must be freed with melody_filter_output_array_free
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_flush_partials(
    filter: *mut CFilter,
) -> *mut CFilterOutputArray {
    if filter.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        let filter = &mut *(filter as *mut FilterImpl);
        let outputs = filter.flush_partials();
        convert_outputs_to_c(outputs)
    }
}

/// Helper function to convert Rust FilterOutput to C representation
unsafe fn convert_outputs_to_c(outputs: Vec<FilterOutput>) -> *mut CFilterOutputArray {
    unsafe {
        let c_outputs: Vec<CFilterOutput> = outputs
            .into_iter()
            .map(|output| convert_output_to_c(output))
            .collect();

        let len = c_outputs.len();
        let ptr = if len > 0 {
            let boxed = c_outputs.into_boxed_slice();
            Box::into_raw(boxed) as *mut CFilterOutput
        } else {
            std::ptr::null_mut()
        };

        Box::into_raw(Box::new(CFilterOutputArray { outputs: ptr, len }))
    }
}

unsafe fn convert_output_to_c(output: FilterOutput) -> CFilterOutput {
    unsafe {
        let text = if !output.text.is_empty() {
            CString::new(output.text).unwrap().into_raw()
        } else {
            std::ptr::null_mut()
        };

        let token_ids_len = output.logprobs.token_ids.len();
        let token_ids = if token_ids_len > 0 {
            let boxed = output.logprobs.token_ids.into_boxed_slice();
            Box::into_raw(boxed) as *mut u32
        } else {
            std::ptr::null_mut()
        };

        let logprobs_len = output.logprobs.logprobs.len();
        let logprobs = if logprobs_len > 0 {
            let boxed = output.logprobs.logprobs.into_boxed_slice();
            Box::into_raw(boxed) as *mut f32
        } else {
            std::ptr::null_mut()
        };

        let (search_query_index, search_query_text) = if let Some(sq) = output.search_query {
            (sq.index as i32, CString::new(sq.text).unwrap().into_raw())
        } else {
            (-1, std::ptr::null_mut())
        };

        let citations_len = output.citations.len();
        let citations = if citations_len > 0 {
            let c_citations: Vec<CFilterCitation> = output
                .citations
                .into_iter()
                .map(|c| convert_citation_to_c(c))
                .collect();
            let boxed = c_citations.into_boxed_slice();
            Box::into_raw(boxed) as *mut CFilterCitation
        } else {
            std::ptr::null_mut()
        };

        let (
            tool_call_index,
            tool_call_id,
            tool_call_name,
            tool_call_param_name,
            tool_call_param_value_delta,
            tool_call_raw_param_delta,
        ) = if let Some(tc) = output.tool_calls {
            let param_name = if let Some(param) = tc.param_delta {
                (
                    CString::new(param.name).unwrap().into_raw(),
                    CString::new(param.value_delta).unwrap().into_raw(),
                )
            } else {
                (std::ptr::null_mut(), std::ptr::null_mut())
            };

            (
                tc.index as i32,
                CString::new(tc.id).unwrap().into_raw(),
                CString::new(tc.name).unwrap().into_raw(),
                param_name.0,
                param_name.1,
                CString::new(tc.raw_param_delta).unwrap().into_raw(),
            )
        } else {
            (
                -1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        CFilterOutput {
            text,
            text_len: if text.is_null() {
                0
            } else {
                CStr::from_ptr(text).to_bytes().len()
            },
            token_ids,
            token_ids_len,
            logprobs,
            logprobs_len,
            search_query_index,
            search_query_text,
            citations,
            citations_len,
            tool_call_index,
            tool_call_id,
            tool_call_name,
            tool_call_param_name,
            tool_call_param_value_delta,
            tool_call_raw_param_delta,
            is_post_answer: output.is_post_answer,
            is_tools_reason: output.is_tools_reason,
        }
    }
}

unsafe fn convert_citation_to_c(citation: FilterCitation) -> CFilterCitation {
    let text = CString::new(citation.text).unwrap().into_raw();

    let sources_len = citation.sources.len();
    let sources = if sources_len > 0 {
        let c_sources: Vec<CSource> = citation
            .sources
            .into_iter()
            .map(|s| {
                let indices_len = s.tool_result_indices.len();
                let indices = if indices_len > 0 {
                    let boxed = s.tool_result_indices.into_boxed_slice();
                    Box::into_raw(boxed) as *mut usize
                } else {
                    std::ptr::null_mut()
                };

                CSource {
                    tool_call_index: s.tool_call_index,
                    tool_result_indices: indices,
                    tool_result_indices_len: indices_len,
                }
            })
            .collect();
        let boxed = c_sources.into_boxed_slice();
        Box::into_raw(boxed) as *mut CSource
    } else {
        std::ptr::null_mut()
    };

    CFilterCitation {
        start_index: citation.start_index,
        end_index: citation.end_index,
        text,
        sources,
        sources_len,
        is_thinking: citation.is_thinking,
    }
}

/// Frees a CFilterOutputArray
///
/// # Safety
/// arr must be a valid pointer returned from melody_filter_write_decoded or melody_filter_flush_partials
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_output_array_free(arr: *mut CFilterOutputArray) {
    if arr.is_null() {
        return;
    }

    unsafe {
        let arr_box = Box::from_raw(arr);

        if !arr_box.outputs.is_null() && arr_box.len > 0 {
            let outputs = Vec::from_raw_parts(arr_box.outputs, arr_box.len, arr_box.len);

            for output in outputs {
                // Free all strings
                if !output.text.is_null() {
                    let _ = CString::from_raw(output.text);
                }
                if !output.search_query_text.is_null() {
                    let _ = CString::from_raw(output.search_query_text);
                }
                if !output.tool_call_id.is_null() {
                    let _ = CString::from_raw(output.tool_call_id);
                }
                if !output.tool_call_name.is_null() {
                    let _ = CString::from_raw(output.tool_call_name);
                }
                if !output.tool_call_param_name.is_null() {
                    let _ = CString::from_raw(output.tool_call_param_name);
                }
                if !output.tool_call_param_value_delta.is_null() {
                    let _ = CString::from_raw(output.tool_call_param_value_delta);
                }
                if !output.tool_call_raw_param_delta.is_null() {
                    let _ = CString::from_raw(output.tool_call_raw_param_delta);
                }

                // Free token_ids and logprobs
                if !output.token_ids.is_null() && output.token_ids_len > 0 {
                    let _ = Vec::from_raw_parts(
                        output.token_ids,
                        output.token_ids_len,
                        output.token_ids_len,
                    );
                }
                if !output.logprobs.is_null() && output.logprobs_len > 0 {
                    let _ = Vec::from_raw_parts(
                        output.logprobs,
                        output.logprobs_len,
                        output.logprobs_len,
                    );
                }

                // Free citations
                if !output.citations.is_null() && output.citations_len > 0 {
                    let citations = Vec::from_raw_parts(
                        output.citations,
                        output.citations_len,
                        output.citations_len,
                    );
                    for citation in citations {
                        if !citation.text.is_null() {
                            let _ = CString::from_raw(citation.text);
                        }

                        // Free sources
                        if !citation.sources.is_null() && citation.sources_len > 0 {
                            let sources = Vec::from_raw_parts(
                                citation.sources,
                                citation.sources_len,
                                citation.sources_len,
                            );
                            for source in sources {
                                if !source.tool_result_indices.is_null()
                                    && source.tool_result_indices_len > 0
                                {
                                    let _ = Vec::from_raw_parts(
                                        source.tool_result_indices,
                                        source.tool_result_indices_len,
                                        source.tool_result_indices_len,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
