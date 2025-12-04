//! Core filtering logic and state machine implementation
//!
//! This module contains the main filter implementation that processes streaming tokens
//! and extracts structured information.

use crate::FilterOptions;
use crate::action_filter::FilterAction;
use crate::types::{FilterMode, FilterOutput, FilterSearchQueryDelta, TokenIDsWithLogProb};
use std::collections::HashMap;

/// Core trait for streaming token parsers.
///
/// This trait defines the interface for processing decoded tokens from the model
/// and extracting structured outputs. Implementations maintain internal state to handle
/// partial tokens and mode transitions.
///
/// # Examples
///
/// ```rust
/// use cohere_melody::{Filter, FilterOptions, new_filter, TokenIDsWithLogProb};
///
/// let options = FilterOptions::new();
/// let mut filter = new_filter(options);
///
/// // Process tokens one at a time
/// let outputs = filter.write_decoded("Hello", TokenIDsWithLogProb::new());
/// let outputs = filter.write_decoded(" world", TokenIDsWithLogProb::new());
///
/// // Flush any buffered content at the end
/// let final_outputs = filter.flush_partials();
/// ```
pub trait Filter {
    /// Process a decoded token and return any completed outputs.
    ///
    /// This method is called for each token string as it's decoded from the model. It may
    /// return zero or more `FilterOutput` instances depending on what structured
    /// content is found.
    ///
    /// # Arguments
    ///
    /// * `decoded_token` - The decoded text for this token
    /// * `prob` - Token IDs and log probabilities for this token
    ///
    /// # Returns
    ///
    /// A vector of parsed outputs (may be empty if content is still buffered)
    fn write_decoded(
        &mut self,
        decoded_token: &str,
        prob: TokenIDsWithLogProb,
    ) -> Vec<FilterOutput>;

    /// Flush any buffered partial outputs.
    ///
    /// This should be called at the end of generation to output any content that
    /// was buffered waiting for special tokens or complete structures.
    ///
    /// # Returns
    ///
    /// Any remaining buffered outputs
    fn flush_partials(&mut self) -> Vec<FilterOutput>;
}

/// Main implementation of the streaming filter state machine.
///
/// This struct maintains all the state needed to incrementally parse token streams,
/// including:
/// - Current parsing mode and mode transitions
/// - Buffered content waiting for complete structures
/// - Position tracking for citations
/// - Configuration options
///
/// # Implementation Notes
///
/// The filter operates as a state machine that:
/// 1. Buffers incoming tokens until they form complete UTF-8 sequences
/// 2. Checks for special tokens that trigger mode transitions
/// 3. Processes content based on the current mode
/// 4. Outputs structured results when complete chunks are available
///
/// # Internal State
///
/// This struct contains many fields to track various aspects of parsing. Users should
/// not create instances directly; use `new_filter()` instead.
#[allow(clippy::struct_excessive_bools)]
pub struct FilterImpl {
    // Trimming configuration
    pub(crate) left_trimmed: bool,
    pub(crate) right_trimmed: bool,

    // Mode and special token configuration
    pub(crate) default_mode: FilterMode,
    pub(crate) special_token_map: HashMap<String, FilterMode>,
    pub(crate) stream_non_grounded_answer: bool,
    pub(crate) stream_tool_actions: bool,
    pub(crate) stream_processed_params: bool,

    // Raw parameter parsing state
    pub(crate) raw_param_indent_length_removed: usize,
    pub(crate) saw_non_whitespace_in_current_line: bool,

    // Citation tracking
    pub(crate) cur_text_index: usize,
    pub(crate) cur_text_byte_index: usize,
    pub(crate) cur_citation_byte_index: Option<usize>,
    pub(crate) action_metadata: FilterAction,

    // Search query tracking
    pub(crate) curr_search_query_idx: usize,
    pub(crate) sent_curr_index: bool,

    // Format flags
    pub(crate) has_tool_call_id: bool,
    pub(crate) cmd3_citations: bool,

    // Chunking configuration
    pub(crate) chunk_size: usize,
    pub(crate) num_tokens_in_chunk: usize,
    pub(crate) chunk_log_probs: TokenIDsWithLogProb,

    // Buffering state
    pub(crate) buf: Vec<u8>,
    pub(crate) partial_special_token_log_prob: TokenIDsWithLogProb,
    pub(crate) mode: FilterMode,
    pub(crate) done: bool,
}

impl FilterImpl {
    pub(crate) fn new() -> Self {
        Self {
            left_trimmed: false,
            right_trimmed: false,
            default_mode: FilterMode::PlainText,
            special_token_map: HashMap::new(),
            stream_non_grounded_answer: false,
            stream_tool_actions: false,
            stream_processed_params: false,
            raw_param_indent_length_removed: 0,
            saw_non_whitespace_in_current_line: false,
            cur_text_index: 0,
            cur_text_byte_index: 0,
            cur_citation_byte_index: None,
            action_metadata: FilterAction::new(),
            curr_search_query_idx: 0,
            sent_curr_index: false,
            has_tool_call_id: false,
            cmd3_citations: false,
            chunk_size: 1,
            num_tokens_in_chunk: 0,
            chunk_log_probs: TokenIDsWithLogProb::new(),
            buf: Vec::new(),
            partial_special_token_log_prob: TokenIDsWithLogProb::new(),
            mode: FilterMode::PlainText,
            done: false,
        }
    }

    pub(crate) fn apply_options(mut self, options: FilterOptions) -> Self {
        self.left_trimmed = options.left_trimmed;
        self.right_trimmed = options.right_trimmed;
        self.chunk_size = options.chunk_size;
        self.stream_non_grounded_answer = options.stream_non_grounded_answer;
        self.stream_tool_actions = options.stream_tool_actions;
        self.stream_processed_params = options.stream_processed_params;
        self.has_tool_call_id = options.has_tool_call_id;
        self.cmd3_citations = options.cmd3_citations;
        self.default_mode = options.default_mode;
        self.mode = options.default_mode;

        // Merge special token maps
        for (token, mode) in &options.special_token_map {
            self.special_token_map.insert(token.clone(), *mode);
        }

        // Add inclusive stops
        for stop in options.inclusive_stops {
            self.special_token_map
                .insert(stop, FilterMode::InclusiveStop);
        }

        // Add exclusive stops
        for stop in options.exclusive_stops {
            self.special_token_map
                .insert(stop, FilterMode::ExclusiveStop);
        }

        self
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
        let (special_token_idx, found_seq) = find_partial(&str, &mut self.special_token_map.keys());
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
                    // Take ownership temporarily to avoid clone
                    let partial_log_prob = std::mem::take(&mut self.partial_special_token_log_prob);
                    let (o, _) = self.handle_token(
                        self.mode,
                        pre_special_token.as_bytes(),
                        false,
                        &partial_log_prob,
                    );
                    // restore
                    self.partial_special_token_log_prob = partial_log_prob;
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

    fn handle_token(
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
            FilterMode::Ignore | FilterMode::NextSearchQuery => (Vec::new(), 0),
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
            FilterMode::Answer | FilterMode::SearchQuery => {
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
            let text = if let Some(start_idx) = self.cur_citation_byte_index {
                s[start_idx..idx + token.len()].to_string()
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
            let text = if let Some(start_idx) = self.cur_citation_byte_index {
                let (trimmed, _) = self.trim_space(&s[start_idx..idx]);
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

    pub(crate) fn utf8_valid_or_limit(bstr: &[u8]) -> bool {
        let limit = 4; // utf-8 is up to 4 bytes
        let valid = std::str::from_utf8(bstr).is_ok();
        if bstr.len() >= limit && !valid {
            log::warn!("emitting invalid utf8: {bstr:?}");
        }
        valid || bstr.len() >= limit
    }

    pub(crate) fn process_search_query(&mut self, bstr: &[u8]) -> (Vec<FilterOutput>, usize) {
        if !Self::utf8_valid_or_limit(bstr) {
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
        if !Self::utf8_valid_or_limit(bstr) {
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

    // TODO: this can be refactored to avoid all the string allocations
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
            // Use take to avoid cloning
            let buf_copy = std::mem::take(&mut self.buf);
            let log_prob_copy = std::mem::take(&mut self.partial_special_token_log_prob);
            let (o, _remove) = self.handle_token(self.mode, &buf_copy, true, &log_prob_copy);
            return o;
        }
        Vec::new()
    }
}

pub(crate) fn find_partial<'a>(
    s: &str,
    stops: impl Iterator<Item = &'a String>,
) -> (usize, String) {
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
        let (idx, found) = find_partial("hello <co: ", stops.iter());
        assert_eq!(idx, 6);
        assert_eq!(found, "<co: ");

        // Test partial match
        let (idx, found) = find_partial("hello <c", stops.iter());
        assert_eq!(idx, 6);
        assert_eq!(found, "");

        // Test no match
        let (idx, _) = find_partial("hello world", stops.iter());
        assert_eq!(idx, usize::MAX);
    }
}
