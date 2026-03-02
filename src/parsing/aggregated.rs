//! Aggregated output types for efficient interop.
//!
//! These types pre-separate content, reasoning, tool call deltas, and citations
//! on the Rust side, so callers don't need to loop over raw `FilterOutput` vectors.

use std::collections::HashMap;

use crate::parsing::types::{FilterCitation, FilterOutput};

#[cfg(feature = "python_ffi")]
use pyo3::prelude::*;

/// Aggregated result of a streaming `write_decoded` call.
/// Pre-separates content, reasoning, and tool call deltas.
#[cfg_attr(feature = "python_ffi", pyclass(get_all))]
#[derive(Debug, Clone, Default)]
pub struct AggregatedResult {
    /// Non-reasoning text content (`None` if no text was produced).
    pub content: Option<String>,
    /// Reasoning/thinking text content (`None` if no reasoning was produced).
    pub reasoning: Option<String>,
    /// Tool call deltas produced in this chunk.
    pub tool_calls: Vec<AccumulatedToolCall>,
    /// Citations produced in this chunk.
    pub citations: Vec<FilterCitation>,
}

/// A fully accumulated tool call
#[cfg_attr(feature = "python_ffi", pyclass(get_all))]
#[derive(Debug, Clone, Default)]
pub struct AccumulatedToolCall {
    /// Tool call index (0-based).
    pub index: usize,
    /// Tool call identifier.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Complete JSON arguments string.
    pub arguments: String,
}

fn push_text(target: &mut Option<String>, text: &mut String) {
    match target {
        Some(s) => s.push_str(text),
        None => {
            *target = Some(std::mem::take(text));
        }
    }
}

/// Aggregate a list of `FilterOutput`s into a single streaming result.
///
/// Consumes the outputs vec to avoid cloning strings — moves text
/// and tool call data directly into the aggregated result.
#[must_use]
pub fn aggregate_stream(outputs: Vec<FilterOutput>) -> AggregatedResult {
    let mut content: Option<String> = None;
    let mut reasoning: Option<String> = None;
    let mut tool_calls = Vec::new();
    let mut citations = Vec::new();

    for mut o in outputs {
        if !o.text.is_empty() {
            let target = if o.is_reasoning {
                &mut reasoning
            } else {
                &mut content
            };
            push_text(target, &mut o.text);
        }
        if !o.citations.is_empty() {
            citations.append(&mut o.citations);
        }
        if let Some(tc) = o.tool_call_delta {
            tool_calls.push(AccumulatedToolCall {
                index: tc.index,
                id: tc.id,
                name: tc.name,
                arguments: tc.raw_param_delta,
            });
        }
    }

    AggregatedResult {
        content,
        reasoning,
        tool_calls,
        citations,
    }
}

/// Aggregate a sequence of `FilterOutput`s into a full result with complete tool calls.
///
/// Consumes the outputs vec. Tool call deltas are accumulated into complete calls
/// using a `HashMap` keyed by index, so deltas may arrive in any order.
#[must_use]
pub fn aggregate_unary(all_outputs: Vec<FilterOutput>) -> AggregatedResult {
    let mut content: Option<String> = None;
    let mut reasoning: Option<String> = None;
    let mut tool_call_map: HashMap<usize, AccumulatedToolCall> = HashMap::new();
    let mut citations = Vec::new();

    for mut o in all_outputs {
        if !o.text.is_empty() {
            let target = if o.is_reasoning {
                &mut reasoning
            } else {
                &mut content
            };
            push_text(target, &mut o.text);
        }
        if !o.citations.is_empty() {
            citations.append(&mut o.citations);
        }
        if let Some(tc) = o.tool_call_delta {
            let call = tool_call_map
                .entry(tc.index)
                .or_insert_with(|| AccumulatedToolCall {
                    index: tc.index,
                    ..Default::default()
                });
            if !tc.id.is_empty() {
                call.id = tc.id;
            }
            if !tc.name.is_empty() {
                call.name = tc.name;
            }
            call.arguments.push_str(&tc.raw_param_delta);
        }
    }

    let mut tool_calls: Vec<AccumulatedToolCall> = tool_call_map.into_values().collect();
    tool_calls.sort_by_key(|tc| tc.index);

    AggregatedResult {
        content,
        reasoning,
        tool_calls,
        citations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::types::FilterToolCallDelta;

    #[test]
    fn test_aggregate_stream_empty() {
        let result = aggregate_stream(vec![]);
        assert!(result.content.is_none());
        assert!(result.reasoning.is_none());
        assert!(result.tool_calls.is_empty());
    }

    #[test]
    fn test_aggregate_stream_content_only() {
        let outputs = vec![
            FilterOutput {
                text: "hello ".into(),
                ..Default::default()
            },
            FilterOutput {
                text: "world".into(),
                ..Default::default()
            },
        ];
        let result = aggregate_stream(outputs);
        assert_eq!(result.content, Some("hello world".into()));
        assert!(result.reasoning.is_none());
        assert!(result.tool_calls.is_empty());
    }

    #[test]
    fn test_aggregate_stream_reasoning_only() {
        let outputs = vec![
            FilterOutput {
                text: "step 1".into(),
                is_reasoning: true,
                ..Default::default()
            },
            FilterOutput {
                text: " step 2".into(),
                is_reasoning: true,
                ..Default::default()
            },
        ];
        let result = aggregate_stream(outputs);
        assert!(result.content.is_none());
        assert_eq!(result.reasoning, Some("step 1 step 2".into()));
    }

    #[test]
    fn test_aggregate_stream_mixed() {
        let outputs = vec![
            FilterOutput {
                text: "thinking...".into(),
                is_reasoning: true,
                ..Default::default()
            },
            FilterOutput {
                text: "hello".into(),
                is_reasoning: false,
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    id: "call_0".into(),
                    name: "search".into(),
                    raw_param_delta: r#"{"q":"test"}"#.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ];

        let result = aggregate_stream(outputs);
        assert_eq!(result.reasoning, Some("thinking...".into()));
        assert_eq!(result.content, Some("hello".into()));
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call_0");
        assert_eq!(result.tool_calls[0].arguments, r#"{"q":"test"}"#);
    }

    #[test]
    fn test_aggregate_stream_with_citations() {
        let outputs = vec![FilterOutput {
            text: "cited text".into(),
            citations: vec![FilterCitation {
                start_index: 0,
                end_index: 10,
                text: "cited text".into(),
                sources: vec![],
                is_thinking: false,
            }],
            ..Default::default()
        }];
        let result = aggregate_stream(outputs);
        assert_eq!(result.citations.len(), 1);
        assert_eq!(result.citations[0].start_index, 0);
    }

    #[test]
    fn test_aggregate_stream_no_text_fields() {
        let outputs = vec![FilterOutput {
            text: String::new(),
            ..Default::default()
        }];
        let result = aggregate_stream(outputs);
        assert!(result.content.is_none());
        assert!(result.reasoning.is_none());
    }

    #[test]
    fn test_aggregate_unary_empty() {
        let result = aggregate_unary(vec![]);
        assert!(result.content.is_none());
        assert!(result.reasoning.is_none());
        assert!(result.tool_calls.is_empty());
        assert!(result.citations.is_empty());
    }

    #[test]
    fn test_aggregate_unary_tool_calls() {
        let outputs = vec![
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    id: "0".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    name: "search".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    raw_param_delta: r#"{"q": "#.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    raw_param_delta: r#""hello"}"#.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                text: "Response text".into(),
                ..Default::default()
            },
        ];

        let result = aggregate_unary(outputs);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "0");
        assert_eq!(result.tool_calls[0].name, "search");
        assert_eq!(result.tool_calls[0].arguments, r#"{"q": "hello"}"#);
        assert_eq!(result.content, Some("Response text".into()));
    }

    #[test]
    fn test_aggregate_unary_multiple_tool_calls() {
        let outputs = vec![
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    id: "call_0".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    name: "search".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 1,
                    id: "call_1".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 1,
                    name: "read".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    raw_param_delta: r#"{"q":"a"}"#.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 1,
                    raw_param_delta: r#"{"file":"b"}"#.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ];

        let result = aggregate_unary(outputs);
        assert_eq!(result.tool_calls.len(), 2);
        assert_eq!(result.tool_calls[0].id, "call_0");
        assert_eq!(result.tool_calls[0].name, "search");
        assert_eq!(result.tool_calls[0].arguments, r#"{"q":"a"}"#);
        assert_eq!(result.tool_calls[1].id, "call_1");
        assert_eq!(result.tool_calls[1].name, "read");
        assert_eq!(result.tool_calls[1].arguments, r#"{"file":"b"}"#);
    }

    #[test]
    fn test_aggregate_unary_reasoning_and_content() {
        let outputs = vec![
            FilterOutput {
                text: "thinking".into(),
                is_reasoning: true,
                ..Default::default()
            },
            FilterOutput {
                text: "answer".into(),
                is_reasoning: false,
                ..Default::default()
            },
        ];
        let result = aggregate_unary(outputs);
        assert_eq!(result.reasoning, Some("thinking".into()));
        assert_eq!(result.content, Some("answer".into()));
        assert!(result.tool_calls.is_empty());
    }
}
