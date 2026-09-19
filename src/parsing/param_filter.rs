//! Parameter value parsing for tool calls
//!
//! This module handles parsing of parameter values from tool action JSON.
//! It supports both basic types (numbers, booleans, null) and complex types
//! (strings, objects, arrays) with proper JSON validation.

use crate::parsing::action_filter::ActionMode;
use crate::parsing::filter::{FilterImpl, PartialMatchResult, find_partial};
use crate::parsing::types::FilterOutput;
use serde::de::IgnoredAny;

/// State machine for parsing parameter values.
///
/// Parameter values can be simple (numbers, booleans) or complex (strings, objects, arrays).
/// This state machine tracks which type is being parsed to know when the value is complete.
///
/// # Examples
///
/// - `Beginning` → sees `"` → transitions to `ComplexType` (string)
/// - `Beginning` → sees `{` → transitions to `ComplexType` (object)
/// - `Beginning` → sees digit → transitions to `BasicType` (number)
/// - `BasicType` → sees `,` or `}` → transitions to `End`
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ParamState {
    /// Initial state, haven't determined value type yet
    Beginning,
    /// Parsing a complex value (string, object, or array)
    ComplexType,
    /// Parsing a basic value (number, boolean, null)
    BasicType,
    /// Value parsing complete
    End,
}

impl FilterImpl {
    pub(crate) fn handle_param_value(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        if s.is_empty() {
            return (Vec::new(), 0);
        }

        match self.action_metadata.cur_param_state {
            ParamState::Beginning => self.handle_param_value_beginning(s),
            ParamState::ComplexType => self.handle_param_value_complex_type(s),
            ParamState::BasicType => self.handle_param_value_basic_type(s),
            ParamState::End => self.handle_param_value_end_type(s),
        }
    }

    fn handle_param_value_beginning(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let trim = s.trim_start();

        if trim.is_empty() {
            return (Vec::new(), 0);
        }

        let first_char = trim.chars().next().unwrap();

        match first_char {
            '"' | '{' | '[' => {
                self.action_metadata.cur_param_state = ParamState::ComplexType;
                self.handle_param_value(s)
            }
            '}' | ',' => {
                self.action_metadata.cur_param_state = ParamState::End;
                self.handle_param_value(s)
            }
            _ => {
                self.action_metadata.cur_param_state = ParamState::BasicType;
                self.handle_param_value(s)
            }
        }
    }

    fn handle_param_value_basic_type(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let idx = match find_partial(s, ["}".to_string(), ",".to_string()].iter()) {
            PartialMatchResult::NoMatch => return self.send_param_value_chunk(s),
            PartialMatchResult::Partial { idx } | PartialMatchResult::Full { idx, .. } => idx,
        };

        let (out, _) = self.send_param_value_chunk(&s[..idx]);
        self.action_metadata.cur_param_state = ParamState::End;
        let (o, r) = self.handle_param_value(&s[idx..]);
        let mut result = out;
        result.extend(o);
        (result, r + idx)
    }

    fn handle_param_value_complex_type(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let idx = find_valid_json_value(&self.action_metadata.param_value_buffer, s);

        if idx == usize::MAX {
            let (out, rem) = self.send_param_value_chunk(s);
            self.action_metadata.param_value_buffer.push_str(s);
            (out, rem)
        } else {
            self.action_metadata.param_value_buffer.clear();
            self.action_metadata.cur_param_state = ParamState::End;
            let (out, _) = self.send_param_value_chunk(&s[..idx]);
            let (o, r) = self.handle_param_value(&s[idx..]);
            let mut result = out;
            result.extend(o);
            (result, r + idx)
        }
    }

    fn handle_param_value_end_type(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let trim = s.trim_start();

        if trim.is_empty() {
            return (Vec::new(), 0);
        }

        let first_char = trim.chars().next().unwrap();
        let idx = s.find(first_char).unwrap();
        let trim_send = s[..idx].trim_end();
        let (out, _) = self.send_param_value_chunk(trim_send);

        // Reset all the metadata
        self.action_metadata.trim_left = true;
        self.action_metadata.param_value_buffer.clear();
        self.action_metadata.cur_param_state = ParamState::Beginning;
        self.action_metadata.cur_param_name.clear();

        if first_char == '}' {
            self.action_metadata.mode = ActionMode::ToolEnd;
            self.action_metadata.cur_tool_call_index += 1;
        } else {
            self.action_metadata.mode = ActionMode::ParamValueEnd;
        }

        let (o, r) = self.parse_actions(&s[idx + 1..]);
        let mut result = out;
        result.extend(o);
        (result, r + idx + 1)
    }
}

/// Find the (byte) index of the first valid json prefix (returns number of bytes)
///
/// This uses serde's streaming deserializer to find the end of the first complete
/// JSON value without constructing a JSON tree or reparsing every prefix.
///
/// # Arguments
///
/// * `buffer` - Previously buffered content
/// * `s` - New content to process
///
/// # Returns
///
/// The index in `s` where a valid JSON value completes, or `usize::MAX` if no
/// complete value is found yet.
///
pub(crate) fn find_valid_json_value(buffer: &str, s: &str) -> usize {
    fn first_value_end(input: &str) -> Option<usize> {
        let mut values = serde_json::Deserializer::from_str(input).into_iter::<IgnoredAny>();
        match values.next() {
            Some(Ok(_)) => Some(values.byte_offset()),
            Some(Err(_)) | None => None,
        }
    }

    if buffer.is_empty() {
        return first_value_end(s).unwrap_or(usize::MAX);
    }

    let mut input = String::with_capacity(buffer.len() + s.len());
    input.push_str(buffer);
    input.push_str(s);

    first_value_end(&input)
        .and_then(|end| end.checked_sub(buffer.len()))
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::action_filter::FilterAction;
    use crate::parsing::filter::FilterImpl;

    fn starting_metadata() -> FilterAction {
        FilterAction {
            mode: ActionMode::NotStarted,
            cur_tool_call_index: 0,
            trim_left: false,
            cur_param_name: String::new(),
            cur_param_state: ParamState::Beginning,
            param_value_buffer: String::new(),
        }
    }

    #[test]
    fn test_find_valid_json_value_complete_prefix() {
        assert_eq!(find_valid_json_value("", r#"{"a":[1,"}"]} trailing"#), 13);
    }

    #[test]
    fn test_find_valid_json_value_across_chunks() {
        assert_eq!(
            find_valid_json_value(r#"{"message":"hé"#, r#"llo"} rest"#),
            5
        );
    }

    #[test]
    fn test_find_valid_json_value_incomplete_or_invalid() {
        assert_eq!(find_valid_json_value("", r#"{"a": [1, 2]"#), usize::MAX);
        assert_eq!(find_valid_json_value("", r#"{"a", 1}"#), usize::MAX);
    }

    #[test]
    fn test_handle_param_value_empty() {
        let mut filter = FilterImpl::new();
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 0);
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn test_handle_param_value_basic_with_next_parameter() {
        let mut filter = FilterImpl::new();
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "30   ,";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 6);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_call_delta {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "30");
    }

    #[test]
    fn test_handle_param_value_basic_with_end_of_tool() {
        let mut filter = FilterImpl::new();
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "1.2   \n}";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 8);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_call_delta {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "1.2");
    }

    #[test]
    fn test_handle_param_value_null_with_end_of_tool() {
        let mut filter = FilterImpl::new();
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "null   \n}";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 9);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_call_delta {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "null");
    }

    #[test]
    fn test_handle_param_value_boolean_with_end_of_tool() {
        let mut filter = FilterImpl::new();
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "true   \n}";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 9);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_call_delta {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "true");
    }

    #[test]
    fn test_handle_param_value_partial_string() {
        let mut filter = FilterImpl::new();
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "\"testing";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 8);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_call_delta {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "\"testing");
    }

    #[test]
    fn test_handle_param_value_whole_string() {
        let mut filter = FilterImpl::new();
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "\"testing string\"   \n}";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 21);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_call_delta {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "\"testing string\"");
    }

    #[test]
    fn test_handle_param_value_whole_object() {
        let mut filter = FilterImpl::new();
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "{\"tes t\": [\"}\"]}   \n,";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 21);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_call_delta {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "{\"tes t\": [\"}\"]}");
    }

    #[test]
    fn test_handle_param_value_partial_object() {
        let mut filter = FilterImpl::new();
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "{\"tes t\": [\"}    ,";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 18);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_call_delta {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "{\"tes t\": [\"}    ,");
    }

    #[test]
    fn test_handle_param_value_whole_array() {
        let mut filter = FilterImpl::new();
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "[{\"test\",[\"}\",\"]\"]}]   }";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 24);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_call_delta {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "[{\"test\",[\"}\",\"]\"]}]   }");
    }

    #[test]
    fn test_handle_param_value_partial_array() {
        let mut filter = FilterImpl::new();
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "[{\"test\",[\"}\",\"]    ,";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 21);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_call_delta {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "[{\"test\",[\"}\",\"]    ,");
    }
}
