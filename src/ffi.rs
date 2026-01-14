//! C Foreign Function Interface (FFI) bindings
//!
//! This module provides C-compatible bindings for the Melody parsing library,
//! primarily intended for use from Go but compatible with any language that can
//! call C functions.
//!
//! # Memory Management
//!
//! **Critical**: All pointers returned by this API must be freed using the
//! corresponding `_free` functions. Failure to do so will result in memory leaks.
//!
//! # Ownership Rules
//!
//! - Pointers returned by `_new` functions: **Caller owns**, must call `_free`
//! - Pointers returned by `write_decoded` and `flush_partials`: **Caller owns**, must call `melody_filter_output_array_free`
//! - Pointers passed as arguments: **Caller retains ownership**, must remain valid for call duration
//!
//! # Thread Safety
//!
//! Filter instances are NOT thread-safe. Each filter must be used by only one
//! thread at a time, or protected by external synchronization.
//!

use crate::filter::{Filter, FilterImpl};
use crate::options::{FilterOptions, new_filter};
use crate::templating::templating::{
    RenderCmd3Options, RenderCmd4Options, render_cmd3, render_cmd4,
};
use crate::templating::types::{
    CitationQuality, Content, ContentType, Document, Grounding, Image, Message, ReasoningType,
    Role, SafetyMode, Tool, ToolCall,
};
use crate::types::{FilterCitation, FilterOutput, TokenIDsWithLogProb};
use serde_json::{Map, Value};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::slice;

/// Opaque pointer to a Filter instance
#[repr(C)]
pub struct CFilter {
    /// Internal marker for opaque pointer
    _private: [u8; 0],
}

/// Opaque pointer to `FilterOptions`
#[repr(C)]
pub struct CFilterOptions {
    /// Internal marker for opaque pointer
    _private: [u8; 0],
}

/// C-compatible representation of `FilterOutput`
#[repr(C)]
pub struct CFilterOutput {
    /// Null-terminated C string containing the output text
    pub text: *mut c_char,
    /// Length of the text in bytes (excluding null terminator)
    pub text_len: usize,

    /// Array of token IDs
    pub token_ids: *mut u32,
    /// Number of token IDs
    pub token_ids_len: usize,
    /// Array of log probabilities (parallel to `token_ids`)
    pub logprobs: *mut f32,
    /// Number of log probabilities
    pub logprobs_len: usize,

    /// Index of the search query (-1 if None)
    pub search_query_index: i32,
    /// Null-terminated C string containing the search query text
    pub search_query_text: *mut c_char,

    /// Array of citations
    pub citations: *mut CFilterCitation,
    /// Number of citations
    pub citations_len: usize,

    /// Index of the tool call (-1 if None)
    pub tool_call_index: i32,
    /// Null-terminated C string containing the tool call ID
    pub tool_call_id: *mut c_char,
    /// Null-terminated C string containing the tool call name
    pub tool_call_name: *mut c_char,
    /// Null-terminated C string containing the parameter name
    pub tool_call_param_name: *mut c_char,
    /// Null-terminated C string containing the parameter value delta
    pub tool_call_param_value_delta: *mut c_char,
    /// Null-terminated C string containing the raw parameter delta
    pub tool_call_raw_param_delta: *mut c_char,

    /// Whether this is post-answer content
    pub is_post_answer: bool,
    /// Whether this is reasoning/thinking content
    pub is_reasoning: bool,
}

/// C-compatible representation of `FilterCitation`
#[repr(C)]
pub struct CFilterCitation {
    /// Character index where the citation starts
    pub start_index: usize,
    /// Character index where the citation ends (exclusive)
    pub end_index: usize,
    /// Null-terminated C string containing the cited text
    pub text: *mut c_char,
    /// Array of sources for this citation
    pub sources: *mut CSource,
    /// Number of sources
    pub sources_len: usize,
    /// Whether this citation appears in a thinking block
    pub is_thinking: bool,
}

/// C-compatible representation of Source
#[repr(C)]
pub struct CSource {
    /// Index of the tool call that produced these results
    pub tool_call_index: usize,
    /// Array of tool result indices
    pub tool_result_indices: *mut usize,
    /// Number of tool result indices
    pub tool_result_indices_len: usize,
}

/// C-compatible representation of an array of `FilterOutput`
#[repr(C)]
pub struct CFilterOutputArray {
    /// Array of filter outputs
    pub outputs: *mut CFilterOutput,
    /// Number of outputs in the array
    pub len: usize,
}

// ============================================================================
// FilterOptions FFI functions
// ============================================================================

/// Creates a new `FilterOptions` instance
///
/// # Safety
/// The returned pointer must be freed with `melody_filter_options_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_new() -> *mut CFilterOptions {
    let options = Box::new(FilterOptions::new());
    Box::into_raw(options).cast::<CFilterOptions>()
}

/// Frees a `FilterOptions` instance
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_free(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let _ = Box::from_raw(options.cast::<FilterOptions>());
        }
    }
}

/// Configures options for multi-hop CMD3 format
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_cmd3(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            *opts = std::mem::take(opts).cmd3();
        }
    }
}

/// Configures options for multi-hop CMD4 format
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_cmd4(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            *opts = std::mem::take(opts).cmd4();
        }
    }
}

/// Configures options for RAG format
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_handle_rag(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            *opts = std::mem::take(opts).handle_rag();
        }
    }
}

/// Configures options for search query format
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_handle_search_query(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            *opts = std::mem::take(opts).handle_search_query();
        }
    }
}

/// Configures options for multi-hop format
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_handle_multi_hop(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            *opts = std::mem::take(opts).handle_multi_hop();
        }
    }
}

/// Enables streaming of non-grounded answers
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_stream_non_grounded_answer(
    options: *mut CFilterOptions,
) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            *opts = std::mem::take(opts).stream_non_grounded_answer();
        }
    }
}

/// Enables streaming of tool actions
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_stream_tool_actions(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            *opts = std::mem::take(opts).stream_tool_actions();
        }
    }
}

/// Enables streaming of processed parameters
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_stream_processed_params(
    options: *mut CFilterOptions,
) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            *opts = std::mem::take(opts).stream_processed_params();
        }
    }
}

/// Sets left trimming
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_with_left_trimmed(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            *opts = std::mem::take(opts).with_left_trimmed();
        }
    }
}

/// Sets right trimming
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_with_right_trimmed(options: *mut CFilterOptions) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            *opts = std::mem::take(opts).with_right_trimmed();
        }
    }
}

/// Sets chunk size
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_with_chunk_size(
    options: *mut CFilterOptions,
    size: usize,
) {
    if !options.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            *opts = std::mem::take(opts).with_chunk_size(size);
        }
    }
}

/// Adds inclusive stops
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
/// `stops` must be valid null-terminated C strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_with_inclusive_stops(
    options: *mut CFilterOptions,
    stops: *const *const c_char,
    stops_len: usize,
) {
    if !options.is_null() && !stops.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            let stops_slice = slice::from_raw_parts(stops, stops_len);
            let stop_strings: Vec<String> = stops_slice
                .iter()
                .map(|&s| CStr::from_ptr(s).to_string_lossy().into_owned())
                .collect();
            *opts = std::mem::take(opts).with_inclusive_stops(stop_strings);
        }
    }
}

/// Adds exclusive stops
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
/// `stops` must be valid null-terminated C strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_with_exclusive_stops(
    options: *mut CFilterOptions,
    stops: *const *const c_char,
    stops_len: usize,
) {
    if !options.is_null() && !stops.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            let stops_slice = slice::from_raw_parts(stops, stops_len);
            let stop_strings: Vec<String> = stops_slice
                .iter()
                .map(|&s| CStr::from_ptr(s).to_string_lossy().into_owned())
                .collect();
            *opts = std::mem::take(opts).with_exclusive_stops(stop_strings);
        }
    }
}

/// Removes a token from the special token map
///
/// # Safety
/// `options` must be a valid pointer returned from `melody_filter_options_new`
/// `token` must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_options_remove_token(
    options: *mut CFilterOptions,
    token: *const c_char,
) {
    if !options.is_null() && !token.is_null() {
        unsafe {
            let opts = &mut *(options.cast::<FilterOptions>());
            let token_str = CStr::from_ptr(token).to_string_lossy();
            *opts = std::mem::take(opts).remove_token(&token_str);
        }
    }
}

// ============================================================================
// Filter FFI functions
// ============================================================================

/// Creates a new filter with the given options
///
/// # Safety
/// - `options` can be null for default options, or must be a valid pointer from `melody_filter_options_new`
/// - The returned pointer must be freed with `melody_filter_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_new(options: *const CFilterOptions) -> *mut CFilter {
    unsafe {
        let filter = if options.is_null() {
            Box::new(FilterImpl::new())
        } else {
            let opts = &*(options.cast::<FilterOptions>());
            Box::new(new_filter(opts.clone()))
        };
        Box::into_raw(filter).cast::<CFilter>()
    }
}

/// Frees a filter instance
///
/// # Safety
/// `filter` must be a valid pointer returned from `melody_filter_new`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_free(filter: *mut CFilter) {
    if !filter.is_null() {
        unsafe {
            let _ = Box::from_raw(filter.cast::<FilterImpl>());
        }
    }
}

/// Writes a decoded token to the filter
///
/// # Safety
/// - `filter` must be a valid pointer returned from `melody_filter_new`
/// - `decoded_token` must be a valid null-terminated C string
/// - The returned `CFilterOutputArray` must be freed with `melody_filter_output_array_free`
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
        let filter = &mut *(filter.cast::<FilterImpl>());
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
/// - `filter` must be a valid pointer returned from `melody_filter_new`
/// - The returned `CFilterOutputArray` must be freed with `melody_filter_output_array_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_filter_flush_partials(
    filter: *mut CFilter,
) -> *mut CFilterOutputArray {
    if filter.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        let filter = &mut *(filter.cast::<FilterImpl>());
        let outputs = filter.flush_partials();
        convert_outputs_to_c(outputs)
    }
}

/// Helper function to convert Rust `FilterOutput` to C representation
///
/// # Safety
/// The returned pointer must be freed appropriately.
unsafe fn convert_outputs_to_c(outputs: Vec<FilterOutput>) -> *mut CFilterOutputArray {
    unsafe {
        let c_outputs: Vec<CFilterOutput> = outputs
            .into_iter()
            .map(|output| convert_output_to_c(output))
            .collect();

        let len = c_outputs.len();
        let ptr = if len > 0 {
            let boxed = c_outputs.into_boxed_slice();
            Box::into_raw(boxed).cast::<CFilterOutput>()
        } else {
            std::ptr::null_mut()
        };

        Box::into_raw(Box::new(CFilterOutputArray { outputs: ptr, len }))
    }
}

/// Converts a single `FilterOutput` to its C representation.
///
/// # Safety
/// The returned struct contains heap-allocated pointers that must be freed.
#[allow(clippy::too_many_lines)]
unsafe fn convert_output_to_c(output: FilterOutput) -> CFilterOutput {
    unsafe {
        let text = if output.text.is_empty() {
            std::ptr::null_mut()
        } else {
            CString::new(output.text).unwrap().into_raw()
        };

        let token_ids_len = output.logprobs.token_ids.len();
        let token_ids = if token_ids_len > 0 {
            let boxed = output.logprobs.token_ids.into_boxed_slice();
            Box::into_raw(boxed).cast::<u32>()
        } else {
            std::ptr::null_mut()
        };

        let logprobs_len = output.logprobs.logprobs.len();
        let logprobs = if logprobs_len > 0 {
            let boxed = output.logprobs.logprobs.into_boxed_slice();
            Box::into_raw(boxed).cast::<f32>()
        } else {
            std::ptr::null_mut()
        };

        let (search_query_index, search_query_text) = if let Some(sq) = output.search_query {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let index = sq.index.min(i32::MAX as usize) as i32;
            (index, CString::new(sq.text).unwrap().into_raw())
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
            Box::into_raw(boxed).cast::<CFilterCitation>()
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
        ) = if let Some(tc) = output.tool_call_delta {
            let param_name = if let Some(param) = tc.param_delta {
                (
                    CString::new(param.name).unwrap().into_raw(),
                    CString::new(param.value_delta).unwrap().into_raw(),
                )
            } else {
                (std::ptr::null_mut(), std::ptr::null_mut())
            };

            (
                {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                    {
                        tc.index.min(i32::MAX as usize) as i32
                    }
                },
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
            is_reasoning: output.is_reasoning,
        }
    }
}

/// Converts a single `FilterCitation` to its C representation.
///
/// # Safety
/// The returned struct contains heap-allocated pointers that must be freed.
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
                    Box::into_raw(boxed).cast::<usize>()
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
        Box::into_raw(boxed).cast::<CSource>()
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

/// Frees a `CFilterOutputArray`
///
/// # Safety
/// `arr` must be a valid pointer returned from `melody_filter_write_decoded` or `melody_filter_flush_partials`
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

// ============================================================================
// Templating FFI types (C-compatible equivalents)
// ============================================================================

/// C-compatible enum for role types.
///
/// Represents the role of a message in the conversation.
#[repr(C)]
#[derive(Copy, Clone)]
pub enum CRole {
    /// Unknown or unspecified role.
    Unknown = 0,
    /// System message role.
    System = 1,
    /// User message role.
    User = 2,
    /// Chatbot message role.
    Chatbot = 3,
    /// Tool message role.
    Tool = 4,
}

/// C-compatible enum for content types.
///
/// Represents the type of content in a message.
#[repr(C)]
#[derive(Copy, Clone)]
pub enum CContentType {
    /// Unknown or unspecified content type.
    Unknown = 0,
    /// Text content.
    Text = 1,
    /// Thinking/reasoning content.
    Thinking = 2,
    /// Image content.
    Image = 3,
    /// Document content.
    Document = 4,
}

/// C-compatible enum for citation quality.
///
/// Indicates the quality or presence of citations.
#[repr(C)]
#[derive(Copy, Clone)]
pub enum CCitationQuality {
    /// Unknown or unspecified citation quality.
    Unknown = 0,
    /// Citations are off.
    Off = 1,
    /// Citations are on.
    On = 2,
}

/// C-compatible enum for grounding options.
///
/// Specifies whether grounding is enabled or disabled.
#[repr(C)]
#[derive(Copy, Clone)]
pub enum CGrounding {
    /// Unknown or unspecified grounding.
    Unknown = 0,
    /// Grounding is enabled.
    Enabled = 1,
    /// Grounding is disabled.
    Disabled = 2,
}

/// C-compatible enum for safety modes.
///
/// Represents the safety mode for rendering.
#[repr(C)]
#[derive(Copy, Clone)]
pub enum CSafetyMode {
    /// Unknown or unspecified safety mode.
    Unknown = 0,
    /// No safety mode.
    None = 1,
    /// Strict safety mode.
    Strict = 2,
    /// Contextual safety mode.
    Contextual = 3,
}

/// C-compatible enum for reasoning types.
///
/// Indicates whether reasoning is enabled or disabled.
#[repr(C)]
#[derive(Copy, Clone)]
pub enum CReasoningType {
    /// Unknown or unspecified reasoning type.
    Unknown = 0,
    /// Reasoning is enabled.
    Enabled = 1,
    /// Reasoning is disabled.
    Disabled = 2,
}

/// C-compatible struct for tool definitions.
#[repr(C)]
pub struct CTool {
    /// Tool name as a null-terminated C string
    pub name: *const c_char,
    /// Tool description as a null-terminated C string
    pub description: *const c_char,
    /// JSON string representing parameters (Map<String, Value>)
    pub parameters_json: *const c_char,
}

/// C-compatible struct for image placeholders.
#[repr(C)]
pub struct CImage {
    /// Image template placeholder as a null-terminated C string
    pub template_placeholder: *const c_char,
}

/// C-compatible struct for content.
#[repr(C)]
pub struct CContent {
    /// Content type enum
    pub content_type: CContentType,
    /// Text content as a null-terminated C string
    pub text: *const c_char,
    /// Thinking content as a null-terminated C string
    pub thinking: *const c_char,
    /// Pointer to image struct (null if None)
    pub image: *const CImage,
    /// Document as a JSON string (null if None)
    pub document_json: *const c_char,
}

/// C-compatible struct for tool calls.
#[repr(C)]
pub struct CToolCall {
    /// Tool call ID as a null-terminated C string
    pub id: *const c_char,
    /// Tool call name as a null-terminated C string
    pub name: *const c_char,
    /// Parameters as a JSON string
    pub parameters_json: *const c_char,
}

/// C-compatible struct for messages.
#[repr(C)]
pub struct CMessage {
    /// Message role enum
    pub role: CRole,
    /// Pointer to array of content structs
    pub content: *const CContent,
    /// Number of content items
    pub content_len: usize,
    /// Pointer to array of tool calls
    pub tool_calls: *const CToolCall,
    /// Number of tool calls
    pub tool_calls_len: usize,
    /// Tool call ID as a null-terminated C string (null if None)
    pub tool_call_id: *const c_char,
}

/// C-compatible struct for CMD3 render options.
#[repr(C)]
pub struct CRenderCmd3Options {
    /// Pointer to array of messages
    pub messages: *const CMessage,
    /// Number of messages
    pub messages_len: usize,
    /// Template as a null-terminated C string
    pub template: *const c_char,
    /// Developer instruction as a null-terminated C string
    pub dev_instruction: *const c_char,
    /// Pointer to array of document JSON strings
    pub documents_json: *const *const c_char,
    /// Number of documents
    pub documents_len: usize,
    /// Pointer to array of available tools
    pub available_tools: *const CTool,
    /// Number of available tools
    pub available_tools_len: usize,
    /// Safety mode enum
    pub safety_mode: CSafetyMode,
    /// Whether safety mode is set
    pub has_safety_mode: bool,
    /// Citation quality enum
    pub citation_quality: CCitationQuality,
    /// Whether citation quality is set
    pub has_citation_quality: bool,
    /// Reasoning type enum
    pub reasoning_type: CReasoningType,
    /// Whether reasoning type is set
    pub has_reasoning_type: bool,
    /// Whether to skip preamble
    pub skip_preamble: bool,
    /// Response prefix as a null-terminated C string
    pub response_prefix: *const c_char,
    /// JSON schema as a null-terminated C string
    pub json_schema: *const c_char,
    /// Whether JSON mode is enabled
    pub json_mode: bool,
    /// Additional template fields as a JSON string
    pub additional_template_fields_json: *const c_char,
    /// Escaped special tokens as a JSON string
    pub escaped_special_tokens_json: *const c_char,
}

/// C-compatible struct for CMD4 render options.
#[repr(C)]
pub struct CRenderCmd4Options {
    /// Pointer to array of messages
    pub messages: *const CMessage,
    /// Number of messages
    pub messages_len: usize,
    /// Template as a null-terminated C string
    pub template: *const c_char,
    /// Developer instruction as a null-terminated C string
    pub dev_instruction: *const c_char,
    /// Platform instruction as a null-terminated C string
    pub platform_instruction: *const c_char,
    /// Pointer to array of document JSON strings
    pub documents_json: *const *const c_char,
    /// Number of documents
    pub documents_len: usize,
    /// Pointer to array of available tools
    pub available_tools: *const CTool,
    /// Number of available tools
    pub available_tools_len: usize,
    /// Grounding enum
    pub grounding: CGrounding,
    /// Whether grounding is set
    pub has_grounding: bool,
    /// Response prefix as a null-terminated C string
    pub response_prefix: *const c_char,
    /// JSON schema as a null-terminated C string
    pub json_schema: *const c_char,
    /// Whether JSON mode is enabled
    pub json_mode: bool,
    /// Additional template fields as a JSON string
    pub additional_template_fields_json: *const c_char,
    /// Escaped special tokens as a JSON string
    pub escaped_special_tokens_json: *const c_char,
}

// ============================================================================
// Templating FFI conversion helpers
// ============================================================================

/// Maps a CRole to a Rust Role.
fn map_role(r: CRole) -> Role {
    match r {
        CRole::Unknown => Role::Unknown,
        CRole::System => Role::System,
        CRole::User => Role::User,
        CRole::Chatbot => Role::Chatbot,
        CRole::Tool => Role::Tool,
    }
}

/// Maps a CContentType to a Rust ContentType.
fn map_content_type(t: CContentType) -> ContentType {
    match t {
        CContentType::Unknown => ContentType::Unknown,
        CContentType::Text => ContentType::Text,
        CContentType::Thinking => ContentType::Thinking,
        CContentType::Image => ContentType::Image,
        CContentType::Document => ContentType::Document,
    }
}

/// Maps a CCitationQuality to a Rust CitationQuality.
fn map_citation_quality(c: CCitationQuality) -> CitationQuality {
    match c {
        CCitationQuality::Unknown => CitationQuality::Unknown,
        CCitationQuality::Off => CitationQuality::Off,
        CCitationQuality::On => CitationQuality::On,
    }
}

/// Maps a CGrounding to a Rust Grounding.
fn map_grounding(g: CGrounding) -> Grounding {
    match g {
        CGrounding::Unknown => Grounding::Unknown,
        CGrounding::Enabled => Grounding::Enabled,
        CGrounding::Disabled => Grounding::Disabled,
    }
}

/// Maps a CSafetyMode to a Rust SafetyMode.
fn map_safety_mode(s: CSafetyMode) -> SafetyMode {
    match s {
        CSafetyMode::Unknown => SafetyMode::Unknown,
        CSafetyMode::None => SafetyMode::None,
        CSafetyMode::Strict => SafetyMode::Strict,
        CSafetyMode::Contextual => SafetyMode::Contextual,
    }
}

/// Maps a CReasoningType to a Rust ReasoningType.
fn map_reasoning_type(r: CReasoningType) -> ReasoningType {
    match r {
        CReasoningType::Unknown => ReasoningType::Unknown,
        CReasoningType::Enabled => ReasoningType::Enabled,
        CReasoningType::Disabled => ReasoningType::Disabled,
    }
}

/// Converts a nullable C string pointer to an Option<String>.
unsafe fn cstr_opt(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        unsafe { Some(CStr::from_ptr(ptr).to_string_lossy().into_owned()) }
    }
}

/// Parses a C string pointer as a JSON object.
unsafe fn parse_json_object(ptr: *const c_char) -> Map<String, Value> {
    if ptr.is_null() {
        Map::new()
    } else {
        unsafe {
            let s = CStr::from_ptr(ptr).to_string_lossy();
            serde_json::from_str::<Map<String, Value>>(&s).unwrap_or_else(|_| Map::new())
        }
    }
}
unsafe fn parse_json_value(ptr: *const c_char) -> Value {
    if ptr.is_null() {
        Value::Null
    } else {
        unsafe {
            let s = CStr::from_ptr(ptr).to_string_lossy();
            serde_json::from_str::<Value>(&s).unwrap_or(Value::Null)
        }
    }
}

unsafe fn convert_ctool(tool: &CTool) -> Tool {
    Tool {
        name: unsafe { CStr::from_ptr(tool.name).to_string_lossy().into_owned() },
        description: unsafe {
            CStr::from_ptr(tool.description)
                .to_string_lossy()
                .into_owned()
        },
        parameters: unsafe { parse_json_object(tool.parameters_json) },
    }
}

unsafe fn convert_cimage(image: &CImage) -> Image {
    Image {
        template_placeholder: unsafe {
            CStr::from_ptr(image.template_placeholder)
                .to_string_lossy()
                .into_owned()
        },
    }
}

unsafe fn convert_ccontent(content: &CContent) -> Content {
    let image = if content.image.is_null() {
        None
    } else {
        Some(unsafe { convert_cimage(&*content.image) })
    };
    let document = if content.document_json.is_null() {
        None
    } else {
        match unsafe { parse_json_value(content.document_json) } {
            Value::Object(m) => Some(m),
            _ => None,
        }
    };
    Content {
        content_type: map_content_type(content.content_type),
        text: unsafe { cstr_opt(content.text) },
        thinking: unsafe { cstr_opt(content.thinking) },
        image,
        document,
    }
}

unsafe fn convert_ctool_call(tc: &CToolCall) -> ToolCall {
    ToolCall {
        id: unsafe { CStr::from_ptr(tc.id).to_string_lossy().into_owned() },
        name: unsafe { CStr::from_ptr(tc.name).to_string_lossy().into_owned() },
        parameters: unsafe {
            CStr::from_ptr(tc.parameters_json)
                .to_string_lossy()
                .into_owned()
        },
    }
}

unsafe fn convert_cmessage(msg: &CMessage) -> Message {
    let contents = if !msg.content.is_null() && msg.content_len > 0 {
        unsafe { slice::from_raw_parts(msg.content, msg.content_len) }
            .iter()
            .map(|c| unsafe { convert_ccontent(c) })
            .collect()
    } else {
        Vec::new()
    };

    let tool_calls = if !msg.tool_calls.is_null() && msg.tool_calls_len > 0 {
        unsafe { slice::from_raw_parts(msg.tool_calls, msg.tool_calls_len) }
            .iter()
            .map(|c| unsafe { convert_ctool_call(c) })
            .collect()
    } else {
        Vec::new()
    };

    Message {
        role: map_role(msg.role),
        content: contents,
        tool_calls,
        tool_call_id: unsafe { cstr_opt(msg.tool_call_id) },
    }
}

unsafe fn convert_cmd3_options<'a>(opts: &CRenderCmd3Options) -> RenderCmd3Options<'a> {
    let messages = if !opts.messages.is_null() && opts.messages_len > 0 {
        unsafe { slice::from_raw_parts(opts.messages, opts.messages_len) }
            .iter()
            .map(|m| unsafe { convert_cmessage(m) })
            .collect()
    } else {
        Vec::new()
    };

    let documents: Vec<Document> = if !opts.documents_json.is_null() && opts.documents_len > 0 {
        unsafe { slice::from_raw_parts(opts.documents_json, opts.documents_len) }
            .iter()
            .map(|&ptr| {
                if ptr.is_null() {
                    Map::new()
                } else {
                    match unsafe { parse_json_value(ptr) } {
                        Value::Object(m) => m,
                        _ => Map::new(),
                    }
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let tools = if !opts.available_tools.is_null() && opts.available_tools_len > 0 {
        unsafe { slice::from_raw_parts(opts.available_tools, opts.available_tools_len) }
            .iter()
            .map(|t| unsafe { convert_ctool(t) })
            .collect()
    } else {
        Vec::new()
    };

    let additional_template_fields =
        unsafe { parse_json_object(opts.additional_template_fields_json) };
    let escaped_special_tokens_raw = unsafe { parse_json_object(opts.escaped_special_tokens_json) };
    let escaped_special_tokens = escaped_special_tokens_raw
        .into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
        .collect();

    let rs_opts = RenderCmd3Options {
        messages,
        dev_instruction: unsafe { cstr_opt(opts.dev_instruction) },
        documents,
        available_tools: tools,
        safety_mode: if opts.has_safety_mode {
            Some(map_safety_mode(opts.safety_mode))
        } else {
            None
        },
        citation_quality: if opts.has_citation_quality {
            Some(map_citation_quality(opts.citation_quality))
        } else {
            None
        },
        reasoning_type: if opts.has_reasoning_type {
            Some(map_reasoning_type(opts.reasoning_type))
        } else {
            None
        },
        skip_preamble: opts.skip_preamble,
        response_prefix: unsafe { cstr_opt(opts.response_prefix) },
        json_schema: unsafe { cstr_opt(opts.json_schema) },
        json_mode: opts.json_mode,
        additional_template_fields,
        escaped_special_tokens,
        ..Default::default()
    };

    let template = unsafe { CStr::from_ptr(opts.template).to_str().unwrap() };
    if template == "" {
        rs_opts
    } else {
        RenderCmd3Options {
            template,
            ..rs_opts
        }
    }
}

unsafe fn convert_cmd4_options<'a>(opts: &CRenderCmd4Options) -> RenderCmd4Options<'a> {
    let messages = if !opts.messages.is_null() && opts.messages_len > 0 {
        unsafe { slice::from_raw_parts(opts.messages, opts.messages_len) }
            .iter()
            .map(|m| unsafe { convert_cmessage(m) })
            .collect()
    } else {
        Vec::new()
    };

    let documents: Vec<Document> = if !opts.documents_json.is_null() && opts.documents_len > 0 {
        unsafe { slice::from_raw_parts(opts.documents_json, opts.documents_len) }
            .iter()
            .map(|&ptr| {
                if ptr.is_null() {
                    Map::new()
                } else {
                    match unsafe { parse_json_value(ptr) } {
                        Value::Object(m) => m,
                        _ => Map::new(),
                    }
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let tools = if !opts.available_tools.is_null() && opts.available_tools_len > 0 {
        unsafe { slice::from_raw_parts(opts.available_tools, opts.available_tools_len) }
            .iter()
            .map(|t| unsafe { convert_ctool(t) })
            .collect()
    } else {
        Vec::new()
    };

    let additional_template_fields =
        unsafe { parse_json_object(opts.additional_template_fields_json) };
    let escaped_special_tokens_raw = unsafe { parse_json_object(opts.escaped_special_tokens_json) };
    let escaped_special_tokens = escaped_special_tokens_raw
        .into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
        .collect();

    let rs_opts = RenderCmd4Options {
        messages,
        template: unsafe { CStr::from_ptr(opts.template).to_str().unwrap() },
        dev_instruction: unsafe { cstr_opt(opts.dev_instruction) },
        platform_instruction: unsafe { cstr_opt(opts.platform_instruction) },
        documents,
        available_tools: tools,
        grounding: if opts.has_grounding {
            Some(map_grounding(opts.grounding))
        } else {
            None
        },
        response_prefix: unsafe { cstr_opt(opts.response_prefix) },
        json_schema: unsafe { cstr_opt(opts.json_schema) },
        json_mode: opts.json_mode,
        additional_template_fields,
        escaped_special_tokens,
    };
    let template = unsafe { CStr::from_ptr(opts.template).to_str().unwrap() };
    if template == "" {
        rs_opts
    } else {
        RenderCmd4Options {
            template,
            ..rs_opts
        }
    }
}

// ============================================================================
// Templating FFI functions
// ============================================================================

/// Renders CMD3 template and returns a newly allocated C string.
/// Caller must free with `melody_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_render_cmd3(opts: *const CRenderCmd3Options) -> *mut c_char {
    if opts.is_null() {
        return std::ptr::null_mut();
    }
    let rust_opts = unsafe { convert_cmd3_options(&*opts) };
    match render_cmd3(&rust_opts) {
        Ok(s) => CString::new(s).unwrap().into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Renders CMD4 template and returns a newly allocated C string.
/// Caller must free with `melody_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_render_cmd4(opts: *const CRenderCmd4Options) -> *mut c_char {
    if opts.is_null() {
        return std::ptr::null_mut();
    }
    let rust_opts = unsafe { convert_cmd4_options(&*opts) };
    match render_cmd4(&rust_opts) {
        Ok(s) => CString::new(s).unwrap().into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a C string returned by render functions.
///
/// # Safety
/// `s` must be a valid pointer returned from a melody render function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn melody_string_free(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}
