use crate::action_filter::FilterAction;
use crate::types::*;
use std::collections::HashMap;

/// Filter is the interface used to parse the output of a cohere model.
pub trait Filter {
    fn write_decoded(
        &mut self,
        decoded_token: &str,
        prob: TokenIDsWithLogProb,
    ) -> Vec<FilterOutput>;
    fn flush_partials(&mut self) -> Vec<FilterOutput>;
}

impl Default for FilterImpl {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FilterImpl {
    pub(crate) left_trimmed: bool,
    pub(crate) right_trimmed: bool,
    pub(crate) trim_prefix: String,
    pub(crate) max_repetition_limit: usize,
    pub(crate) max_repetition_sequence_length: usize,

    pub(crate) default_mode: FilterMode,
    pub(crate) special_token_map: HashMap<String, FilterMode>,
    pub(crate) stream_non_grounded_answer: bool,
    pub(crate) stream_tool_actions: bool,
    pub(crate) stream_processed_params: bool,

    pub(crate) raw_param_indent_length_removed: usize,
    pub(crate) saw_non_whitespace_in_current_line: bool,

    pub(crate) cur_text_index: usize,
    pub(crate) cur_text_byte_index: usize,
    pub(crate) cur_citation_byte_index: isize,
    pub(crate) action_metadata: FilterAction,

    pub(crate) curr_search_query_idx: usize,
    pub(crate) sent_curr_index: bool,

    pub(crate) has_tool_call_id: bool,
    pub(crate) cmd3_citations: bool,
    pub(crate) llama_tool_parsing: bool,

    pub(crate) chunk_size: usize,
    pub(crate) num_tokens_in_chunk: usize,
    pub(crate) chunk_log_probs: TokenIDsWithLogProb,

    pub(crate) buf: Vec<u8>,
    pub(crate) partial_special_token_log_prob: TokenIDsWithLogProb,
    pub(crate) mode: FilterMode,
    pub(crate) special_token_keys: Vec<String>,
    pub(crate) done: bool,
}

impl FilterImpl {
    pub fn new() -> Self {
        Self {
            left_trimmed: false,
            right_trimmed: false,
            trim_prefix: String::new(),
            max_repetition_limit: 0,
            max_repetition_sequence_length: 0,
            default_mode: FilterMode::PlainText,
            special_token_map: HashMap::new(),
            stream_non_grounded_answer: false,
            stream_tool_actions: false,
            stream_processed_params: false,
            raw_param_indent_length_removed: 0,
            saw_non_whitespace_in_current_line: false,
            cur_text_index: 0,
            cur_text_byte_index: 0,
            cur_citation_byte_index: -1,
            action_metadata: FilterAction::new(),
            curr_search_query_idx: 0,
            sent_curr_index: false,
            has_tool_call_id: false,
            cmd3_citations: false,
            llama_tool_parsing: false,
            chunk_size: 1,
            num_tokens_in_chunk: 0,
            chunk_log_probs: TokenIDsWithLogProb::new(),
            buf: Vec::new(),
            partial_special_token_log_prob: TokenIDsWithLogProb::new(),
            mode: FilterMode::PlainText,
            special_token_keys: Vec::new(),
            done: false,
        }
    }

    pub(crate) fn write_text(
        &mut self,
        text: &[u8],
        logprobs: TokenIDsWithLogProb,
    ) -> Vec<FilterOutput> {
        if self.done {
            return Vec::new();
        }

        self.buf.extend_from_slice(text);
        let str = String::from_utf8_lossy(&self.buf).to_string();

        // If is a partial special token, we need to wait for the next token.
        let (special_token_idx, found_seq) = find_partial(&str, &self.special_token_keys);
        if special_token_idx != usize::MAX && found_seq.is_empty() {
            self.partial_special_token_log_prob = logprobs;
            return Vec::new();
        }

        let mut out = Vec::new();

        // If it is a whole special token, change the mode, remove the tokens and continue
        if special_token_idx != usize::MAX && !found_seq.is_empty() {
            let (o, new_mode, stop, valid_special) =
                self.handle_special_token(&str, special_token_idx, &found_seq, self.mode);
            out.extend(o);

            if valid_special {
                if stop {
                    self.buf.clear();
                    self.done = true;
                    return out;
                }

                // Before the special token, process the buffer with the old mode
                let pre_special_token = &str[..special_token_idx];
                if !pre_special_token.is_empty() {
                    let partial_log_prob = self.partial_special_token_log_prob.clone();
                    let (o, _) = self.handle_token(
                        self.mode,
                        pre_special_token.as_bytes(),
                        false,
                        &partial_log_prob,
                    );
                    out.extend(o);
                }

                // Remove the special token and the text before
                let remove_len = pre_special_token.len() + found_seq.len();
                self.buf.drain(..remove_len);

                // Change mode
                self.mode = new_mode;
            }
        }

        // Process buffer by mode
        if !self.buf.is_empty() {
            self.num_tokens_in_chunk += 1;
            self.chunk_log_probs.append(logprobs);

            if self.chunk_size > 1 && self.num_tokens_in_chunk < self.chunk_size {
                return out;
            }

            let (o, remove) = self.handle_token(
                self.mode,
                &self.buf.clone(),
                false,
                &self.chunk_log_probs.clone(),
            );
            out.extend(o);
            self.buf.drain(..remove);
            self.num_tokens_in_chunk = 0;
            self.chunk_log_probs = TokenIDsWithLogProb::new();
        }

        out
    }

    pub(crate) fn handle_token(
        &mut self,
        mode: FilterMode,
        bstr: &[u8],
        after_last_token: bool,
        token_log_probs: &TokenIDsWithLogProb,
    ) -> (Vec<FilterOutput>, usize) {
        match mode {
            FilterMode::InclusiveStop | FilterMode::ExclusiveStop => {
                log::error!("in stop mode but we should have already stopped");
                (Vec::new(), 0)
            }
            FilterMode::Ignore => (Vec::new(), 0),
            FilterMode::ToolAction => {
                let s = String::from_utf8_lossy(bstr);
                self.parse_actions(&s)
            }
            FilterMode::GroundedAnswer | FilterMode::ToolReason => {
                self.process_grounded_text(bstr, after_last_token, mode, Some(token_log_probs))
            }
            FilterMode::SearchQuery => self.process_search_query(bstr),
            FilterMode::Answer => {
                if self.stream_non_grounded_answer {
                    self.process_text(bstr, Some(token_log_probs))
                } else {
                    (Vec::new(), bstr.len())
                }
            }
            FilterMode::PlainText => self.process_text(bstr, Some(token_log_probs)),
            FilterMode::NextSearchQuery => (Vec::new(), 0),
        }
    }

    fn handle_special_token(
        &mut self,
        s: &str,
        idx: usize,
        token: &str,
        cur_mode: FilterMode,
    ) -> (Vec<FilterOutput>, FilterMode, bool, bool) {
        let new_mode = self
            .special_token_map
            .get(token)
            .copied()
            .unwrap_or(FilterMode::PlainText);

        // Disable mode change if in grounded answer or answer mode and see "Answer:" in the text
        let not_special = (cur_mode == FilterMode::GroundedAnswer
            || cur_mode == FilterMode::Answer)
            && new_mode == FilterMode::Answer;

        if not_special {
            return (Vec::new(), cur_mode, false, false);
        }

        match new_mode {
            FilterMode::InclusiveStop => {
                let out = self.handle_inclusive_stop(s, idx, token);
                (out, new_mode, true, true)
            }
            FilterMode::ExclusiveStop => {
                let out = self.handle_exclusive_stop(s, idx);
                (out, new_mode, true, true)
            }
            FilterMode::GroundedAnswer => {
                self.cur_text_index = 0;
                if self.stream_non_grounded_answer {
                    self.left_trimmed = true;
                }
                (Vec::new(), new_mode, false, true)
            }
            FilterMode::ToolReason => {
                self.left_trimmed = true;
                self.right_trimmed = true;
                (Vec::new(), new_mode, false, true)
            }
            FilterMode::Answer => {
                self.left_trimmed = true;
                (Vec::new(), new_mode, false, true)
            }
            FilterMode::SearchQuery => {
                self.left_trimmed = true;
                (Vec::new(), new_mode, false, true)
            }
            FilterMode::NextSearchQuery => {
                self.left_trimmed = true;
                if self.sent_curr_index {
                    self.curr_search_query_idx += 1;
                    self.sent_curr_index = false;
                }
                (Vec::new(), FilterMode::SearchQuery, false, true)
            }
            _ => (Vec::new(), new_mode, false, true),
        }
    }

    pub(crate) fn handle_inclusive_stop(
        &self,
        s: &str,
        idx: usize,
        token: &str,
    ) -> Vec<FilterOutput> {
        if idx != usize::MAX && !s[..idx + token.len()].is_empty() {
            let text = if self.cur_citation_byte_index != -1 {
                s[self.cur_citation_byte_index as usize..idx + token.len()].to_string()
            } else {
                s[..idx + token.len()].to_string()
            };
            return vec![FilterOutput {
                text,
                ..Default::default()
            }];
        }
        Vec::new()
    }

    pub(crate) fn handle_exclusive_stop(&mut self, s: &str, idx: usize) -> Vec<FilterOutput> {
        if idx != usize::MAX && !s[..idx].is_empty() {
            let text = if self.cur_citation_byte_index != -1 {
                let (trimmed, _) = self.trim_space(&s[self.cur_citation_byte_index as usize..idx]);
                trimmed
            } else {
                let (trimmed, _) = self.trim_space(&s[..idx]);
                trimmed
            };
            return vec![FilterOutput {
                text,
                ..Default::default()
            }];
        }
        Vec::new()
    }

    pub(crate) fn utf8_valid_or_limit(&self, bstr: &[u8]) -> bool {
        let limit = 4; // utf-8 is up to 4 bytes
        let valid = std::str::from_utf8(bstr).is_ok();
        if bstr.len() >= limit && !valid {
            log::warn!("emitting invalid utf8: {:?}", bstr);
        }
        valid || bstr.len() >= limit
    }

    pub(crate) fn process_search_query(&mut self, bstr: &[u8]) -> (Vec<FilterOutput>, usize) {
        if !self.utf8_valid_or_limit(bstr) {
            return (Vec::new(), 0);
        }

        let s = String::from_utf8_lossy(bstr);
        let (send, rem_right) = self.trim_space(&s);
        let mut out = Vec::new();

        if !send.is_empty() {
            out.push(FilterOutput {
                search_query: Some(FilterSearchQueryDelta {
                    index: self.curr_search_query_idx,
                    text: send,
                }),
                ..Default::default()
            });
            self.sent_curr_index = true;
        }

        (out, bstr.len() - rem_right)
    }

    pub(crate) fn process_text(
        &mut self,
        bstr: &[u8],
        token_log_probs: Option<&TokenIDsWithLogProb>,
    ) -> (Vec<FilterOutput>, usize) {
        if !self.utf8_valid_or_limit(bstr) {
            return (Vec::new(), 0);
        }

        let s = String::from_utf8_lossy(bstr);
        let (send, rem_right) = self.trim_space(&s);
        let mut out = Vec::new();

        if !send.is_empty() {
            let mut output = FilterOutput {
                text: send,
                ..Default::default()
            };
            if let Some(probs) = token_log_probs {
                output.logprobs = probs.clone();
            }
            out.push(output);
        }

        (out, bstr.len() - rem_right)
    }

    pub(crate) fn trim_space(&mut self, s: &str) -> (String, usize) {
        let mut result = s.to_string();
        let mut rem = 0;

        if self.right_trimmed {
            rem = result.len();
            result = result.trim_end().to_string();
            rem -= result.len();
        }

        if self.left_trimmed {
            result = result.trim_start().to_string();
            if !result.is_empty() {
                self.left_trimmed = false;
            }
        }

        if !self.trim_prefix.is_empty() {
            let prefix_len = self.trim_prefix.len().min(result.len());
            let full_prefix_len = self.trim_prefix.len();

            let prefix = &self.trim_prefix[..prefix_len];

            if result.starts_with(prefix) {
                if prefix_len == full_prefix_len {
                    self.trim_prefix.clear();
                    return (result[prefix_len..].to_string(), rem);
                }
                return (String::new(), result.len() + rem);
            }
            self.trim_prefix.clear();
        }

        (result, rem)
    }
}

impl Filter for FilterImpl {
    fn write_decoded(&mut self, decoded_token: &str, l: TokenIDsWithLogProb) -> Vec<FilterOutput> {
        self.write_text(decoded_token.as_bytes(), l)
    }

    fn flush_partials(&mut self) -> Vec<FilterOutput> {
        self.done = true;
        if !self.buf.is_empty()
            && self.mode != FilterMode::InclusiveStop
            && self.mode != FilterMode::ExclusiveStop
        {
            let buf_copy = self.buf.clone();
            let log_prob_copy = self.partial_special_token_log_prob.clone();
            let (o, remove) = self.handle_token(self.mode, &buf_copy, true, &log_prob_copy);
            self.buf.drain(..remove);
            return o;
        }
        Vec::new()
    }
}

/// Find partial returns first index in str that might match one of stop sequences.
pub(crate) fn find_partial(s: &str, stops: &[String]) -> (usize, String) {
    let mut min_idx = usize::MAX;

    for stop in stops {
        // If we find the stop sequence, return the index and the stop sequence
        if let Some(idx) = s.find(stop) {
            return (idx, stop.clone());
        }

        // Go through the substrings of the stop sequence
        for i in 0..s.len() {
            let suffix = if stop.len() > s.len() - i {
                &stop[..s.len() - i]
            } else {
                stop
            };

            if s.ends_with(suffix) {
                let idx = s.len() - suffix.len();
                if min_idx == usize::MAX || min_idx > idx {
                    min_idx = idx;
                }
                break;
            }
        }
    }

    (
        if min_idx == usize::MAX {
            usize::MAX
        } else {
            min_idx
        },
        String::new(),
    )
}

#[cfg(test)]
mod tests {
    use crate::filter::find_partial;

    #[test]
    fn test_find_partial() {
        let stops = vec!["<co: ".to_string(), "</co: ".to_string()];

        // Test full match
        let (idx, found) = find_partial("hello <co: ", &stops);
        assert_eq!(idx, 6);
        assert_eq!(found, "<co: ");

        // Test partial match
        let (idx, found) = find_partial("hello <c", &stops);
        assert_eq!(idx, 6);
        assert_eq!(found, "");

        // Test no match
        let (idx, _) = find_partial("hello world", &stops);
        assert_eq!(idx, usize::MAX);
    }
}

// --- C FFI for FilterImpl ---

#[repr(C)]
pub struct CTokenIDsWithLogProb {
    token_ids: *const u32,
    logprobs: *const f32,
    len: usize,
}

#[repr(C)]
pub struct CFilterSearchQueryDelta {
    index: usize,
    text: *const std::os::raw::c_char,
}

#[repr(C)]
pub struct CFilterToolParameter {
    name: *const std::os::raw::c_char,
    value_delta: *const std::os::raw::c_char,
}

#[repr(C)]
pub struct CFilterToolCallDelta {
    index: usize,
    id: *const std::os::raw::c_char,
    name: *const std::os::raw::c_char,
    param_delta: *mut CFilterToolParameter,
    raw_param_delta: *const std::os::raw::c_char,
}

#[repr(C)]
pub struct CSource {
    tool_call_index: usize,
    tool_result_indices: *const usize,
    tool_result_indices_len: usize,
}

#[repr(C)]
pub struct CFilterCitation {
    start_index: usize,
    end_index: usize,
    text: *const std::os::raw::c_char,
    sources: *mut CSource,
    sources_len: usize,
    is_thinking: bool,
}

#[repr(C)]
pub struct CFilterOutput {
    text: *const std::os::raw::c_char,
    token_ids: *const u32,
    logprobs: *const f32,
    len: usize,
    search_query: *mut CFilterSearchQueryDelta,
    citations: *mut CFilterCitation,
    citations_len: usize,
    tool_calls: *mut CFilterToolCallDelta,
    is_post_answer: bool,
    is_tools_reason: bool,
}

#[repr(C)]
pub struct CFilterOutputVec {
    outputs: *mut CFilterOutput,
    len: usize,
}

use std::ffi::{CString, CStr};
use std::os::raw::{c_char};

#[unsafe(no_mangle)]
pub extern "C" fn filterimpl_new() -> *mut FilterImpl {
    Box::into_raw(Box::new(FilterImpl::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn filterimpl_free(ptr: *mut FilterImpl) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)); }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn filterimpl_write_decoded(
    ptr: *mut FilterImpl,
    decoded_token: *const c_char,
    token_ids: *const u32,
    logprobs: *const f32,
    len: usize,
) -> CFilterOutputVec {
    let filter = unsafe { ptr.as_mut().unwrap() };
    let decoded_token = unsafe { CStr::from_ptr(decoded_token).to_str().unwrap() };
    let token_ids = unsafe { std::slice::from_raw_parts(token_ids, len) };
    let logprobs = unsafe { std::slice::from_raw_parts(logprobs, len) };
    let probs = crate::types::TokenIDsWithLogProb {
        token_ids: token_ids.to_vec(),
        logprobs: logprobs.to_vec(),
    };
    let outputs = filter.write_decoded(decoded_token, probs);

    let mut c_outputs: Vec<CFilterOutput> = outputs
        .into_iter()
        .map(|out| {
            // Text
            let c_text = CString::new(out.text).unwrap();
            let text_ptr = c_text.into_raw();

            // Token IDs and logprobs
            let ids_ptr = if !out.logprobs.token_ids.is_empty() {
                out.logprobs.token_ids.as_ptr()
            } else {
                std::ptr::null()
            };
            let probs_ptr = if !out.logprobs.logprobs.is_empty() {
                out.logprobs.logprobs.as_ptr()
            } else {
                std::ptr::null()
            };
            let len = out.logprobs.token_ids.len();
            std::mem::forget(out.logprobs.token_ids.clone());
            std::mem::forget(out.logprobs.logprobs.clone());

            // Search Query
            let search_query_ptr = if let Some(sq) = out.search_query {
                let c_sq_text = CString::new(sq.text).unwrap();
                let sq_struct = Box::new(CFilterSearchQueryDelta {
                    index: sq.index,
                    text: c_sq_text.into_raw(),
                });
                Box::into_raw(sq_struct)
            } else {
                std::ptr::null_mut()
            };

            // Citations
            let mut c_citations: Vec<CFilterCitation> = Vec::new();
            for cit in &out.citations {
                let c_cit_text = CString::new(cit.text.clone()).unwrap();
                // Sources
                let mut c_sources: Vec<CSource> = Vec::new();
                for src in &cit.sources {
                    let indices_ptr = if !src.tool_result_indices.is_empty() {
                        src.tool_result_indices.as_ptr()
                    } else {
                        std::ptr::null()
                    };
                    c_sources.push(CSource {
                        tool_call_index: src.tool_call_index,
                        tool_result_indices: indices_ptr,
                        tool_result_indices_len: src.tool_result_indices.len(),
                    });
                }
                let sources_ptr = if !c_sources.is_empty() {
                    let ptr = c_sources.as_mut_ptr();
                    std::mem::forget(c_sources);
                    ptr
                } else {
                    std::ptr::null_mut()
                };
                c_citations.push(CFilterCitation {
                    start_index: cit.start_index,
                    end_index: cit.end_index,
                    text: c_cit_text.into_raw(),
                    sources: sources_ptr,
                    sources_len: cit.sources.len(),
                    is_thinking: cit.is_thinking,
                });
            }
            let citations_ptr = if !c_citations.is_empty() {
                let ptr = c_citations.as_mut_ptr();
                std::mem::forget(c_citations);
                ptr
            } else {
                std::ptr::null_mut()
            };

            // Tool Calls
            let tool_calls_ptr = if let Some(tc) = &out.tool_calls {
                // ToolCallDelta fields
                let c_id = CString::new(tc.id.clone()).unwrap();
                let c_name = CString::new(tc.name.clone()).unwrap();
                let c_raw_param_delta = CString::new(tc.raw_param_delta.clone()).unwrap();
                let param_delta_ptr = if let Some(param) = &tc.param_delta {
                    let c_param_name = CString::new(param.name.clone()).unwrap();
                    let c_param_value = CString::new(param.value_delta.clone()).unwrap();
                    let param_struct = Box::new(CFilterToolParameter {
                        name: c_param_name.into_raw(),
                        value_delta: c_param_value.into_raw(),
                    });
                    Box::into_raw(param_struct)
                } else {
                    std::ptr::null_mut()
                };
                let tc_struct = Box::new(CFilterToolCallDelta {
                    index: tc.index,
                    id: c_id.into_raw(),
                    name: c_name.into_raw(),
                    param_delta: param_delta_ptr,
                    raw_param_delta: c_raw_param_delta.into_raw(),
                });
                Box::into_raw(tc_struct)
            } else {
                std::ptr::null_mut()
            };

            CFilterOutput {
                text: text_ptr,
                token_ids: ids_ptr,
                logprobs: probs_ptr,
                len,
                search_query: search_query_ptr,
                citations: citations_ptr,
                citations_len: out.citations.len(),
                tool_calls: tool_calls_ptr,
                is_post_answer: out.is_post_answer,
                is_tools_reason: out.is_tools_reason,
            }
        })
        .collect();

    let out_ptr = if !c_outputs.is_empty() {
        let ptr = c_outputs.as_mut_ptr();
        std::mem::forget(c_outputs);
        ptr
    } else {
        std::ptr::null_mut()
    };

    CFilterOutputVec {
        outputs: out_ptr,
        len: outputs.len(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn filterimpl_flush_partials(ptr: *mut FilterImpl) -> CFilterOutputVec {
    let filter = unsafe { ptr.as_mut().unwrap() };
    let outputs = filter.flush_partials();

    let mut c_outputs: Vec<CFilterOutput> = outputs
        .into_iter()
        .map(|out| {
            // Text
            let c_text = CString::new(out.text).unwrap();
            let text_ptr = c_text.into_raw();

            // Token IDs and logprobs
            let ids_ptr = if !out.logprobs.token_ids.is_empty() {
                out.logprobs.token_ids.as_ptr()
            } else {
                std::ptr::null()
            };
            let probs_ptr = if !out.logprobs.logprobs.is_empty() {
                out.logprobs.logprobs.as_ptr()
            } else {
                std::ptr::null()
            };
            let len = out.logprobs.token_ids.len();
            std::mem::forget(out.logprobs.token_ids.clone());
            std::mem::forget(out.logprobs.logprobs.clone());

            // Search Query
            let search_query_ptr = if let Some(sq) = out.search_query {
                let c_sq_text = CString::new(sq.text).unwrap();
                let sq_struct = Box::new(CFilterSearchQueryDelta {
                    index: sq.index,
                    text: c_sq_text.into_raw(),
                });
                Box::into_raw(sq_struct)
            } else {
                std::ptr::null_mut()
            };

            // Citations
            let mut c_citations: Vec<CFilterCitation> = Vec::new();
            for cit in &out.citations {
                let c_cit_text = CString::new(cit.text.clone()).unwrap();
                // Sources
                let mut c_sources: Vec<CSource> = Vec::new();
                for src in &cit.sources {
                    let indices_ptr = if !src.tool_result_indices.is_empty() {
                        src.tool_result_indices.as_ptr()
                    } else {
                        std::ptr::null()
                    };
                    c_sources.push(CSource {
                        tool_call_index: src.tool_call_index,
                        tool_result_indices: indices_ptr,
                        tool_result_indices_len: src.tool_result_indices.len(),
                    });
                }
                let sources_ptr = if !c_sources.is_empty() {
                    let ptr = c_sources.as_mut_ptr();
                    std::mem::forget(c_sources);
                    ptr
                } else {
                    std::ptr::null_mut()
                };
                c_citations.push(CFilterCitation {
                    start_index: cit.start_index,
                    end_index: cit.end_index,
                    text: c_cit_text.into_raw(),
                    sources: sources_ptr,
                    sources_len: cit.sources.len(),
                    is_thinking: cit.is_thinking,
                });
            }
            let citations_ptr = if !c_citations.is_empty() {
                let ptr = c_citations.as_mut_ptr();
                std::mem::forget(c_citations);
                ptr
            } else {
                std::ptr::null_mut()
            };

            // Tool Calls
            let tool_calls_ptr = if let Some(tc) = &out.tool_calls {
                // ToolCallDelta fields
                let c_id = CString::new(tc.id.clone()).unwrap();
                let c_name = CString::new(tc.name.clone()).unwrap();
                let c_raw_param_delta = CString::new(tc.raw_param_delta.clone()).unwrap();
                let param_delta_ptr = if let Some(param) = &tc.param_delta {
                    let c_param_name = CString::new(param.name.clone()).unwrap();
                    let c_param_value = CString::new(param.value_delta.clone()).unwrap();
                    let param_struct = Box::new(CFilterToolParameter {
                        name: c_param_name.into_raw(),
                        value_delta: c_param_value.into_raw(),
                    });
                    Box::into_raw(param_struct)
                } else {
                    std::ptr::null_mut()
                };
                let tc_struct = Box::new(CFilterToolCallDelta {
                    index: tc.index,
                    id: c_id.into_raw(),
                    name: c_name.into_raw(),
                    param_delta: param_delta_ptr,
                    raw_param_delta: c_raw_param_delta.into_raw(),
                });
                Box::into_raw(tc_struct)
            } else {
                std::ptr::null_mut()
            };

            CFilterOutput {
                text: text_ptr,
                token_ids: ids_ptr,
                logprobs: probs_ptr,
                len,
                search_query: search_query_ptr,
                citations: citations_ptr,
                citations_len: out.citations.len(),
                tool_calls: tool_calls_ptr,
                is_post_answer: out.is_post_answer,
                is_tools_reason: out.is_tools_reason,
            }
        })
        .collect();

    let out_ptr = if !c_outputs.is_empty() {
        let ptr = c_outputs.as_mut_ptr();
        std::mem::forget(c_outputs);
        ptr
    } else {
        std::ptr::null_mut()
    };

    CFilterOutputVec {
        outputs: out_ptr,
        len: outputs.len(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn filterimpl_free_outputs(vec: CFilterOutputVec) {
    if vec.outputs.is_null() {
        return;
    }
    unsafe {
        let slice = std::slice::from_raw_parts_mut(vec.outputs, vec.len);
        for out in slice {
            if !out.text.is_null() {
                drop(CString::from_raw(out.text as *mut c_char));
            }
            // token_ids/logprobs are leaked Vecs, so we must reconstruct and drop them
            if out.len > 0 && !out.token_ids.is_null() {
                let _ = Vec::from_raw_parts(out.token_ids as *mut u32, out.len, out.len);
            }
            if out.len > 0 && !out.logprobs.is_null() {
                let _ = Vec::from_raw_parts(out.logprobs as *mut f32, out.len, out.len);
            }
            // Free search_query
            if !out.search_query.is_null() {
                let sq = Box::from_raw(out.search_query);
                if !sq.text.is_null() {
                    drop(CString::from_raw(sq.text as *mut c_char));
                }
            }
            // Free citations
            if out.citations_len > 0 && !out.citations.is_null() {
                let citations = std::slice::from_raw_parts_mut(out.citations, out.citations_len);
                for cit in citations {
                    if !cit.text.is_null() {
                        drop(CString::from_raw(cit.text as *mut c_char));
                    }
                    if cit.sources_len > 0 && !cit.sources.is_null() {
                        // sources is a flat array of CSource, tool_result_indices are borrowed
                        Vec::from_raw_parts(cit.sources, cit.sources_len, cit.sources_len);
                    }
                }
                Vec::from_raw_parts(out.citations, out.citations_len, out.citations_len);
            }
            // Free tool_calls
            if !out.tool_calls.is_null() {
                let tc = Box::from_raw(out.tool_calls);
                if !tc.id.is_null() {
                    drop(CString::from_raw(tc.id as *mut c_char));
                }
                if !tc.name.is_null() {
                    drop(CString::from_raw(tc.name as *mut c_char));
                }
                if !tc.raw_param_delta.is_null() {
                    drop(CString::from_raw(tc.raw_param_delta as *mut c_char));
                }
                if !tc.param_delta.is_null() {
                    let param = Box::from_raw(tc.param_delta);
                    if !param.name.is_null() {
                        drop(CString::from_raw(param.name as *mut c_char));
                    }
                    if !param.value_delta.is_null() {
                        drop(CString::from_raw(param.value_delta as *mut c_char));
                    }
                }
            }
        }
        Vec::from_raw_parts(vec.outputs, vec.len, vec.len);
    }
}
