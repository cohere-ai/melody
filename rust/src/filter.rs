use crate::action_filter::FilterAction;
use crate::types::*;
use std::collections::HashMap;
use std::error::Error;

/// Filter is the interface used to parse the output of a cohere model.
pub trait Filter {
    fn write(
        &mut self,
        token: u32,
        likelihood: Option<f32>,
    ) -> Result<Vec<FilterOutput>, Box<dyn Error>>;
    fn write_decoded(
        &mut self,
        decoded_token: &str,
        prob: TokenIDsWithLogProb,
    ) -> Vec<FilterOutput>;
    fn flush_partials(&mut self) -> Vec<FilterOutput>;
    fn get_raw_tokens(&self) -> &[u32];
}

pub struct FilterImpl {
    pub(crate) tokenizer: Option<tokenizers::Tokenizer>,
    pub(crate) token_buf: Vec<u32>,
    pub(crate) log_prob_buf: Vec<f32>,
    pub(crate) raw_tokens: Vec<u32>,

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
    pub fn new(tokenizer: Option<tokenizers::Tokenizer>) -> Self {
        Self {
            tokenizer,
            token_buf: Vec::new(),
            log_prob_buf: Vec::new(),
            raw_tokens: Vec::new(),
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

    fn decode_token(
        &mut self,
        token: u32,
        token_log_prob: Option<f32>,
    ) -> Result<String, Box<dyn Error>> {
        self.token_buf.push(token);

        let text = if let Some(ref tokenizer) = self.tokenizer {
            tokenizer.decode(&self.token_buf, false).unwrap()
        } else {
            String::new()
        };

        if let Some(prob) = token_log_prob {
            self.log_prob_buf.push(prob);
        }

        Ok(text)
    }

    fn get_full_text_with_log_probs(
        &mut self,
        token: u32,
        token_log_prob: Option<f32>,
    ) -> Result<Option<FullTextWithLogprobs>, Box<dyn Error>> {
        self.raw_tokens.push(token);

        // Check if the token is repeated too many times
        let has_repetition_limits =
            self.max_repetition_limit > 0 && self.max_repetition_sequence_length > 0;
        if has_repetition_limits
            && has_hit_token_repetition_limit(
                &self.raw_tokens,
                self.max_repetition_limit,
                self.max_repetition_sequence_length,
            )
        {
            return Err("saw too many repeated tokens".into());
        }

        let text = self.decode_token(token, token_log_prob)?;

        // Multi-token characters will decode into this string
        if text.ends_with('\u{fffd}') {
            return Ok(None);
        }

        let token_buf_copy = self.token_buf.clone();
        self.token_buf.clear();

        let log_probs_copy = if !self.log_prob_buf.is_empty() {
            let copy = self.log_prob_buf.clone();
            self.log_prob_buf.clear();
            copy
        } else {
            Vec::new()
        };

        Ok(Some(FullTextWithLogprobs {
            text: text.into_bytes(),
            logprobs: TokenIDsWithLogProb {
                token_ids: token_buf_copy,
                logprobs: log_probs_copy,
            },
        }))
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
    fn write(
        &mut self,
        token: u32,
        likelihood: Option<f32>,
    ) -> Result<Vec<FilterOutput>, Box<dyn Error>> {
        let t = self.get_full_text_with_log_probs(token, likelihood)?;
        if let Some(t) = t {
            Ok(self.write_text(&t.text, t.logprobs))
        } else {
            Ok(Vec::new())
        }
    }

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

    fn get_raw_tokens(&self) -> &[u32] {
        &self.raw_tokens
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

/// hashTokensForRepetitionCheck is essentially a DJB2 hash function
pub(crate) fn hash_tokens_for_repetition_check(seq: &[u32]) -> u64 {
    let mut hash: u64 = 5381;
    for &v in seq {
        hash = hash.wrapping_mul(33).wrapping_add(v as u64);
    }
    hash
}

fn has_hit_token_repetition_limit(
    seen_tokens: &[u32],
    repetition_limit: usize,
    max_sequence_length: usize,
) -> bool {
    if seen_tokens.len() <= repetition_limit {
        return false;
    }

    let max_possible_seq_len = seen_tokens.len() / repetition_limit;
    let max_sequence_length = max_sequence_length.min(max_possible_seq_len);

    for seq_len in 1..=max_sequence_length {
        let start = seen_tokens.len() - repetition_limit * seq_len;
        let tokens = &seen_tokens[start..];

        let mut first_hash = 0u64;
        let mut mismatch = false;

        for i in 0..repetition_limit {
            let offset = i * seq_len;
            let h = hash_tokens_for_repetition_check(&tokens[offset..offset + seq_len]);
            if i == 0 {
                first_hash = h;
            } else if h != first_hash {
                mismatch = true;
                break;
            }
        }

        if !mismatch {
            return true;
        }
    }

    false
}
