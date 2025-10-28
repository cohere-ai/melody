use crate::action_filter::ActionMode;
use crate::filter::{FilterImpl, find_partial};
use crate::types::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ParamState {
    Beginning,
    ComplexType,
    BasicType,
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
        let (idx, _) = find_partial(s, &["}".to_string(), ",".to_string()]);

        if idx == usize::MAX {
            return self.send_param_value_chunk(s);
        }

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
        let (out, rem) = self.send_param_value_chunk(trim_send);

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

        let (o, r) = self.parse_actions(&s[rem + 1..]);
        let mut result = out;
        result.extend(o);
        (result, r + rem + 1)
    }
}

/// Find the index of the first valid json prefix
pub(crate) fn find_valid_json_value(buffer: &str, s: &str) -> usize {
    let mut whole_str = buffer.to_string();

    for (i, c) in s.chars().enumerate() {
        whole_str.push(c);
        if serde_json::from_str::<serde_json::Value>(&whole_str).is_ok() {
            return i + 1;
        }
    }

    usize::MAX
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_filter::FilterAction;
    use crate::filter::FilterImpl;
    use tokenizers::Tokenizer;

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
    fn test_handle_param_value_empty() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 0);
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn test_handle_param_value_basic_with_next_parameter() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "30   ,";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 6);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_calls {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "30");
    }

    #[test]
    fn test_handle_param_value_basic_with_end_of_tool() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "1.2   \n}";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 8);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_calls {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "1.2");
    }

    #[test]
    fn test_handle_param_value_null_with_end_of_tool() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "null   \n}";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 9);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_calls {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "null");
    }

    #[test]
    fn test_handle_param_value_boolean_with_end_of_tool() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "true   \n}";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 9);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_calls {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "true");
    }

    #[test]
    fn test_handle_param_value_partial_string() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "\"testing";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 8);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_calls {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "\"testing");
    }

    #[test]
    fn test_handle_param_value_whole_string() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "\"testing string\"   \n}";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 17);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_calls {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "\"testing string\"");
    }

    #[test]
    fn test_handle_param_value_whole_object() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "{\"tes t\": [\"}\"]}   \n,";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 17);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_calls {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "{\"tes t\": [\"}\"]}");
    }

    #[test]
    fn test_handle_param_value_partial_object() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "{\"tes t\": [\"}    ,";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 18);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_calls {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "{\"tes t\": [\"}    ,");
    }

    #[test]
    fn test_handle_param_value_whole_array() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "[{\"test\",[\"}\",\"]\"]}]   }";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 24);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_calls {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "[{\"test\",[\"}\",\"]\"]}]   }");
    }

    #[test]
    fn test_handle_param_value_partial_array() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.action_metadata = starting_metadata();
        filter.stream_tool_actions = true;

        let input = "[{\"test\",[\"}\",\"]    ,";
        let (out, actual_remove) = filter.handle_param_value(input);

        assert_eq!(actual_remove, 21);
        let mut result = String::new();
        for o in out {
            if let Some(tool_calls) = o.tool_calls {
                if let Some(param_delta) = tool_calls.param_delta {
                    result.push_str(&param_delta.value_delta);
                }
            }
        }
        assert_eq!(result, "[{\"test\",[\"}\",\"]    ,");
    }
}
