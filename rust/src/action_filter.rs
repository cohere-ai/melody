use crate::filter::FilterImpl;
use crate::param_filter::ParamState;
use crate::types::*;
use regex::Regex;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ActionMode {
    NotStarted,
    ToolCallID,
    ToolCallIDEnd,
    ToolName,
    ToolNameEnd,
    ParamName,
    ParamValue,
    ToolEnd,
    ParamNameEnd,
    ParamValueEnd,
    RawParam,
}

#[derive(Debug, Clone)]
pub(crate) struct FilterAction {
    pub mode: ActionMode,
    pub cur_tool_call_index: usize,
    pub trim_left: bool,
    pub cur_param_name: String,
    pub cur_param_state: ParamState,
    pub param_value_buffer: String,
}

impl FilterAction {
    pub fn new() -> Self {
        Self {
            mode: ActionMode::NotStarted,
            cur_tool_call_index: 0,
            trim_left: false,
            cur_param_name: String::new(),
            cur_param_state: ParamState::Beginning,
            param_value_buffer: String::new(),
        }
    }
}

impl FilterImpl {
    pub(crate) fn parse_actions(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        if s.is_empty() || s.ends_with('\\') {
            return (Vec::new(), 0);
        }

        match self.action_metadata.mode {
            ActionMode::NotStarted | ActionMode::ToolEnd => {
                self.handle_before_tool(s, self.has_tool_call_id)
            }
            ActionMode::ToolCallID => self.handle_in_tool_call_id(s),
            ActionMode::ToolCallIDEnd => self.handle_tool_call_id_end(s),
            ActionMode::ToolName => self.handle_in_tool_name(s),
            ActionMode::ToolNameEnd => self.handle_tool_name_end(s),
            ActionMode::RawParam => self.handle_raw_param(s),
            ActionMode::ParamName => self.handle_param_name(s),
            ActionMode::ParamNameEnd => self.handle_end_of_param_name(s),
            ActionMode::ParamValue => self.handle_param_value(s),
            ActionMode::ParamValueEnd => self.handle_param_value_end(s),
        }
    }

    fn handle_before_tool(&mut self, s: &str, check_call_id: bool) -> (Vec<FilterOutput>, usize) {
        let (regex, mode) = if self.llama_tool_parsing {
            (Regex::new(r#""name":\s*""#).unwrap(), ActionMode::ToolName)
        } else if check_call_id {
            (
                Regex::new(r#""tool_call_id":\s*""#).unwrap(),
                ActionMode::ToolCallID,
            )
        } else {
            (
                Regex::new(r#""tool_name":\s*""#).unwrap(),
                ActionMode::ToolName,
            )
        };

        if let Some(mat) = regex.find(s) {
            self.action_metadata.mode = mode;
            self.action_metadata.trim_left = true;
            let (out, rem) = self.parse_actions(&s[mat.end()..]);
            (out, rem + mat.end())
        } else {
            (Vec::new(), 0)
        }
    }

    fn handle_in_tool_call_id(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        if let Some(idx) = find_non_escaped_char(s, '"') {
            let out = self.send_tool_call_id_chunk(&s[..idx]);
            self.action_metadata.mode = ActionMode::ToolCallIDEnd;
            let (o, r) = self.parse_actions(&s[idx..]);
            let mut result = out;
            result.extend(o);
            (result, r + idx + 1)
        } else {
            (Vec::new(), 0)
        }
    }

    fn handle_tool_call_id_end(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        self.handle_before_tool(s, false)
    }

    fn handle_in_tool_name(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        if let Some(idx) = find_non_escaped_char(s, '"') {
            let out = self.send_tool_name_chunk(&s[..idx]);
            self.action_metadata.mode = ActionMode::ToolNameEnd;
            let (o, r) = self.parse_actions(&s[idx..]);
            let mut result = out;
            result.extend(o);
            (result, r + idx + 1)
        } else {
            (Vec::new(), 0)
        }
    }

    fn handle_tool_name_end(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let param_regex = Regex::new(r#""parameters":\s*\{\s*""#).unwrap();

        if let Some(mat) = param_regex.find(s) {
            if self.stream_processed_params {
                self.action_metadata.mode = ActionMode::ParamName;
                let (out, rem) = self.parse_actions(&s[mat.end()..]);
                return (out, rem + mat.end());
            }
            self.action_metadata.mode = ActionMode::RawParam;
            let raw_param_regex = Regex::new(r#""parameters":\s*"#).unwrap();
            if let Some(mat) = raw_param_regex.find(s) {
                let (out, rem) = self.parse_actions(&s[mat.end()..]);
                return (out, rem + mat.end());
            }
        }

        if let Some(idx) = s.find('}') {
            self.action_metadata.mode = ActionMode::ToolEnd;
            self.action_metadata.cur_tool_call_index += 1;
            self.action_metadata.cur_param_name.clear();
            let (out, rem) = self.parse_actions(&s[idx..]);
            (out, rem + idx)
        } else {
            (Vec::new(), 0)
        }
    }

    fn handle_raw_param(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        use crate::param_filter::find_valid_json_value;

        let idx = find_valid_json_value(&self.action_metadata.param_value_buffer, s);

        if idx == usize::MAX {
            let out = self.send_raw_param_chunk_without_indentation(s);
            self.action_metadata.param_value_buffer.push_str(s);
            (out, s.len())
        } else {
            let out = self.send_raw_param_chunk_without_indentation(&s[..idx]);
            self.action_metadata.param_value_buffer.clear();
            self.action_metadata.cur_tool_call_index += 1;
            self.action_metadata.mode = ActionMode::ToolEnd;
            let (o, r) = self.parse_actions(&s[idx..]);
            let mut result = out;
            result.extend(o);
            (result, r + idx)
        }
    }

    const NUM_SPACE_TO_REMOVE_PER_LINE: usize = 8;

    fn send_raw_param_chunk_without_indentation(&mut self, s: &str) -> Vec<FilterOutput> {
        let mut trimmed_str = String::new();

        for c in s.chars() {
            match c {
                '\n' => {
                    self.raw_param_indent_length_removed = 0;
                    self.saw_non_whitespace_in_current_line = false;
                    trimmed_str.push(c);
                }
                c if c.is_whitespace() => {
                    if self.raw_param_indent_length_removed < Self::NUM_SPACE_TO_REMOVE_PER_LINE
                        && !self.saw_non_whitespace_in_current_line
                    {
                        self.raw_param_indent_length_removed += 1;
                        continue;
                    }
                    trimmed_str.push(c);
                }
                _ => {
                    self.saw_non_whitespace_in_current_line = true;
                    trimmed_str.push(c);
                }
            }
        }

        self.send_raw_param_chunk(&trimmed_str)
    }

    fn handle_param_name(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        if let Some(idx) = find_non_escaped_char(s, '"') {
            let out = self.send_param_name_chunk(&s[..idx]);
            self.action_metadata.mode = ActionMode::ParamNameEnd;
            let (o, r) = self.parse_actions(&s[idx..]);
            let mut result = out;
            result.extend(o);
            (result, r + idx + 1)
        } else {
            (Vec::new(), 0)
        }
    }

    fn handle_end_of_param_name(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let param_name_regex = Regex::new(r#"\s*:\s*"#).unwrap();

        if let Some(mat) = param_name_regex.find(s) {
            self.action_metadata.mode = ActionMode::ParamValue;
            let (out, rem) = self.parse_actions(&s[mat.end()..]);
            (out, rem + mat.end())
        } else {
            (Vec::new(), 0)
        }
    }

    fn handle_param_value_end(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        if let Some(idx) = s.find('"') {
            self.action_metadata.mode = ActionMode::ParamName;
            let (out, rem) = self.parse_actions(&s[idx + 1..]);
            (out, rem + idx + 1)
        } else {
            (Vec::new(), 0)
        }
    }

    fn send_tool_call_id_chunk(&self, s: &str) -> Vec<FilterOutput> {
        if s.is_empty() || !self.stream_tool_actions {
            return Vec::new();
        }

        vec![FilterOutput {
            tool_calls: Some(FilterToolCallDelta {
                index: self.action_metadata.cur_tool_call_index,
                id: s.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }]
    }

    fn send_tool_name_chunk(&self, s: &str) -> Vec<FilterOutput> {
        if s.is_empty() || !self.stream_tool_actions {
            return Vec::new();
        }

        vec![FilterOutput {
            tool_calls: Some(FilterToolCallDelta {
                index: self.action_metadata.cur_tool_call_index,
                name: s.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }]
    }

    fn send_param_name_chunk(&mut self, s: &str) -> Vec<FilterOutput> {
        if s.is_empty() || !self.stream_tool_actions {
            return Vec::new();
        }

        self.action_metadata.cur_param_name = s.to_string();

        vec![FilterOutput {
            tool_calls: Some(FilterToolCallDelta {
                index: self.action_metadata.cur_tool_call_index,
                param_delta: Some(FilterToolParameter {
                    name: s.to_string(),
                    value_delta: String::new(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        }]
    }

    fn send_raw_param_chunk(&self, s: &str) -> Vec<FilterOutput> {
        if s.is_empty() || !self.stream_tool_actions {
            return Vec::new();
        }

        vec![FilterOutput {
            tool_calls: Some(FilterToolCallDelta {
                index: self.action_metadata.cur_tool_call_index,
                raw_param_delta: s.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }]
    }

    pub(crate) fn send_param_value_chunk(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let trimmed_str = s.trim_end();
        let trimmed_str = if self.action_metadata.trim_left {
            trimmed_str.trim_start()
        } else {
            trimmed_str
        };

        if trimmed_str.is_empty() || !self.stream_tool_actions {
            return (Vec::new(), 0);
        }

        self.action_metadata.trim_left = false;

        (
            vec![FilterOutput {
                tool_calls: Some(FilterToolCallDelta {
                    index: self.action_metadata.cur_tool_call_index,
                    param_delta: Some(FilterToolParameter {
                        name: self.action_metadata.cur_param_name.clone(),
                        value_delta: trimmed_str.to_string(),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            s.len(),
        )
    }
}

fn find_non_escaped_char(s: &str, ch: char) -> Option<usize> {
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == ch as u8 {
            let mut escaped = false;
            for j in (0..i).rev() {
                if bytes[j] != b'\\' {
                    break;
                }
                escaped = !escaped;
            }
            if !escaped {
                return Some(i);
            }
        }
    }
    None
}
