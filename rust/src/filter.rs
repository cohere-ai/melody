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
