#[cfg(test)]
mod tests {
    use crate::filter::{Filter, FilterImpl};
    use crate::options::FilterOptions;
    use crate::types::*;

    use tokenizers::Tokenizer;

    #[test]
    fn test_citations_standard_case() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = None;

        let input = "hello <co: 2,1>foo</co: 2,1>";
        let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello foo");
        assert_eq!(output.citations.len(), 1);
        assert_eq!(output.citations[0].start_index, 6);
        assert_eq!(output.citations[0].end_index, 9);
        assert_eq!(output.citations[0].text, "foo");
        assert_eq!(output.citations[0].sources.len(), 1);
        assert_eq!(output.citations[0].sources[0].tool_call_index, 0);
        assert_eq!(
            output.citations[0].sources[0].tool_result_indices,
            vec![2, 1]
        );
        assert_eq!(remove, 28);
    }

    #[test]
    fn test_citations_no_document() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = None;

        let input = "hello <co: >foo</co: >";
        let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello foo");
        assert_eq!(output.citations.len(), 1);
        assert_eq!(output.citations[0].start_index, 6);
        assert_eq!(output.citations[0].end_index, 9);
        assert_eq!(output.citations[0].text, "foo");
        assert_eq!(output.citations[0].sources.len(), 0);
        assert_eq!(remove, 22);
    }

    #[test]
    fn test_citations_multiple() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = None;

        let input = "hello <co: 1>foo</co: 1> world <co: 2>bar</co: 2>";
        let (output, _remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello foo world bar");
        assert_eq!(output.citations.len(), 2);

        assert_eq!(output.citations[0].start_index, 6);
        assert_eq!(output.citations[0].end_index, 9);
        assert_eq!(output.citations[0].text, "foo");

        assert_eq!(output.citations[1].start_index, 16);
        assert_eq!(output.citations[1].end_index, 19);
        assert_eq!(output.citations[1].text, "bar");
    }

    #[test]
    fn test_trim_space_left() {
        let mut filter = FilterImpl::new();
        filter.left_trimmed = true;

        let (result, rem) = filter.trim_space("   hello");
        assert_eq!(result, "hello");
        assert_eq!(rem, 0);
        assert!(!filter.left_trimmed); // Should be reset after trimming
    }

    #[test]
    fn test_trim_space_right() {
        let mut filter = FilterImpl::new();
        filter.right_trimmed = true;

        let (result, rem) = filter.trim_space("hello   ");
        assert_eq!(result, "hello");
        assert_eq!(rem, 3);
    }

    #[test]
    fn test_trim_space_both() {
        let mut filter = FilterImpl::new();
        filter.left_trimmed = true;
        filter.right_trimmed = true;

        let (result, rem) = filter.trim_space("   hello   ");
        assert_eq!(result, "hello");
        assert_eq!(rem, 3);
    }

    #[test]
    fn test_filter_options_builder() {
        let options = FilterOptions::new()
            .with_left_trimmed()
            .with_right_trimmed()
            .with_chunk_size(5);

        assert!(options.left_trimmed);
        assert!(options.right_trimmed);
        assert_eq!(options.chunk_size, 5);
    }

    #[test]
    fn test_filter_options_cmd3() {
        let options = FilterOptions::new().cmd3();

        assert_eq!(options.default_mode, FilterMode::GroundedAnswer);
        assert!(options.right_trimmed);
        assert!(options.has_tool_call_id);
        assert!(options.cmd3_citations);
        assert!(options.stream_tool_actions);
        assert!(options.special_token_map.contains_key("<|START_RESPONSE|>"));
        assert!(options.special_token_map.contains_key("<|START_THINKING|>"));
        assert!(options.special_token_map.contains_key("<|START_ACTION|>"));
    }

    #[test]
    fn test_process_text_with_logprobs() {
        let mut filter = FilterImpl::new();

        let text = "hello world";
        let logprobs = TokenIDsWithLogProb {
            token_ids: vec![1, 2, 3],
            logprobs: vec![0.1, 0.2, 0.3],
        };

        let (outputs, _) = filter.process_text(text.as_bytes(), Some(&logprobs));

        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].text, "hello world");
        assert_eq!(outputs[0].logprobs.token_ids, vec![1, 2, 3]);
        assert_eq!(outputs[0].logprobs.logprobs, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_process_search_query() {
        let mut filter = FilterImpl::new();
        filter.curr_search_query_idx = 0;

        let (outputs, remove) = filter.process_search_query(b"test query");

        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].search_query.is_some());
        assert_eq!(outputs[0].search_query.as_ref().unwrap().index, 0);
        assert_eq!(outputs[0].search_query.as_ref().unwrap().text, "test query");
        assert!(filter.sent_curr_index);
        assert_eq!(remove, 10);
    }

    #[test]
    fn test_handle_inclusive_stop() {
        let filter = FilterImpl::new();

        let outputs = filter.handle_inclusive_stop("hello<|END|>", 5, "<|END|>");
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].text, "hello<|END|>");
    }

    #[test]
    fn test_handle_exclusive_stop() {
        let mut filter = FilterImpl::new();

        let outputs = filter.handle_exclusive_stop("hello<|END|>", 5);
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].text, "hello");
    }

    #[test]
    fn test_token_ids_with_log_prob_append() {
        let mut logprobs1 = TokenIDsWithLogProb {
            token_ids: vec![1, 2],
            logprobs: vec![0.1, 0.2],
        };

        let logprobs2 = TokenIDsWithLogProb {
            token_ids: vec![3, 4],
            logprobs: vec![0.3, 0.4],
        };

        logprobs1.append(logprobs2);

        assert_eq!(logprobs1.token_ids, vec![1, 2, 3, 4]);
        assert_eq!(logprobs1.logprobs, vec![0.1, 0.2, 0.3, 0.4]);
    }

    static TOKENIZER: std::sync::LazyLock<Tokenizer> = std::sync::LazyLock::new(|| {
        Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    });

    struct FilterTestCase {
        name: &'static str,
        input: &'static str,
        options: FilterOptions,
        // Aggregated result so test cases can be simpler
        want_text: &'static str,
        want_thinking: &'static str,
        want_tool_calls: Vec<FilterToolCallDelta>,
        want_likelihoods: Vec<f32>,
        want_citations: Vec<FilterCitation>,
    }

    fn run_filter_test(tt: FilterTestCase) {
        // for simplicity's sake lets just generate likelihoods in intervals of thousandths
        let mut test_likelihoods: Vec<f32> = Vec::new();
        for i in 0..999 {
            test_likelihoods.push(i as f32 / 1000.0);
        }

        let encoding = TOKENIZER.encode(tt.input, false).unwrap();
        let tokens = encoding.get_ids();

        // Duplicate the test by writing the raw strings instead
        let mut text_chunks: Vec<String> = Vec::new();
        let mut buffer: Vec<u32> = Vec::new();
        let mut likelihoods_chunks: Vec<TokenIDsWithLogProb> = Vec::new();
        let mut likelihood_buffer: Vec<f32> = Vec::new();

        let mut out_text = String::new();
        let mut out_thinking = String::new();
        let mut out_likelihoods = Vec::new();
        let mut out_tool_calls: Vec<FilterToolCallDelta> = Vec::new();
        let mut out_citations: Vec<FilterCitation> = Vec::new();

        for (i, &token) in tokens.iter().enumerate() {
            buffer.push(token);
            likelihood_buffer.push(test_likelihoods[i]);

            let decoded = TOKENIZER.decode(&buffer, false).unwrap();

            if decoded.ends_with(char::REPLACEMENT_CHARACTER) {
                continue;
            }

            text_chunks.push(decoded);
            likelihoods_chunks.push(TokenIDsWithLogProb {
                token_ids: buffer.clone(),
                logprobs: likelihood_buffer.clone(),
            });
            buffer.clear();
            likelihood_buffer.clear();
        }

        let mut filter = crate::options::new_filter(tt.options);
        for (i, chunk) in text_chunks.iter().enumerate() {
            let out = filter.write_decoded(chunk, likelihoods_chunks[i].clone());
            for o in out.iter() {
                if o.is_reasoning {
                    out_thinking.push_str(&o.text);
                } else {
                    out_text.push_str(&o.text);
                }
                for f in o.logprobs.logprobs.iter() {
                    out_likelihoods.push(*f)
                }
                if let Some(c) = &o.tool_call_delta {
                    if c.index >= out_tool_calls.len() {
                        let mut ftcd = FilterToolCallDelta::default();
                        ftcd.index = c.index;
                        out_tool_calls.push(ftcd);
                    }
                    out_tool_calls[c.index].id.push_str(&c.id);
                    out_tool_calls[c.index].name.push_str(&c.name);
                    out_tool_calls[c.index]
                        .raw_param_delta
                        .push_str(&c.raw_param_delta);
                }
                for c in o.citations.iter() {
                    out_citations.push(c.clone());
                }
            }
        }

        assert_eq!(
            out_text, tt.want_text,
            "Test case '{}' (WriteDecoded) failed - text not equal",
            tt.name
        );
        assert_eq!(
            out_thinking, tt.want_thinking,
            "Test case '{}' (WriteDecoded) failed - thinking not equal",
            tt.name
        );
        assert_eq!(
            out_likelihoods, tt.want_likelihoods,
            "Test case '{}' (WriteDecoded) failed - likelihoods not equal",
            tt.name
        );
        assert_eq!(
            out_tool_calls, tt.want_tool_calls,
            "Test case '{}' (WriteDecoded) failed - tool_calls not equal",
            tt.name
        );
        assert_eq!(
            out_citations, tt.want_citations,
            "Test case '{}' (WriteDecoded) failed - citations not equal",
            tt.name
        )
    }

    #[test]
    fn test_filter_inclusive_stop() {
        run_filter_test(FilterTestCase {
            name: "inclusive stop test",
            input: "The tallest penguin is the emperor penguin.",
            options: FilterOptions::new().with_inclusive_stops(vec!["emperor penguin".to_string()]),
            want_text: "The tallest penguin is the emperor penguin",
            want_thinking: "",
            want_tool_calls: vec![],
            want_citations: vec![],
            want_likelihoods: vec![0.001, 0.002, 0.003],
        })
    }

    #[test]
    fn test_filter_exclusive_stop() {
        run_filter_test(FilterTestCase {
            name: "exclusive stop test",
            input: "The tallest penguin is the emperor penguin.",
            options: FilterOptions::new().with_exclusive_stops(vec!["emperor penguin".to_string()]),
            want_text: "The tallest penguin is the ",
            want_thinking: "",
            want_tool_calls: vec![],
            want_citations: vec![],
            want_likelihoods: vec![0.001, 0.002, 0.003],
        })
    }

    #[test]
    fn test_filter_likelihoods() {
        run_filter_test(FilterTestCase {
            name: "basic test",
            input: "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
            options: FilterOptions::new(),
            want_text: "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
            want_thinking: "",
            want_tool_calls: vec![],
            want_citations: vec![],
            want_likelihoods: vec![
                0.0, 0.001, 0.002, 0.003, 0.004, 0.005, 0.006, 0.007, 0.008, 0.009, 0.01, 0.011,
                0.012, 0.013, 0.014, 0.015, 0.016, 0.017, 0.018, 0.019, 0.02, 0.021, 0.022, 0.023,
                0.024, 0.025, 0.026, 0.027, 0.028, 0.029, 0.03, 0.031, 0.032, 0.033, 0.034, 0.035,
                0.036, 0.037, 0.038, 0.039, 0.04, 0.041, 0.042, 0.043, 0.044, 0.045,
            ],
        });
    }

    #[test]
    fn test_filter_command3_simple() {
        run_filter_test(FilterTestCase {
            name: "basic test",
            input: "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
            options: FilterOptions::new(),
            want_text: "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
            want_thinking: "",
            want_tool_calls: vec![],
            want_citations: vec![],
            want_likelihoods: vec![
                0.0, 0.001, 0.002, 0.003, 0.004, 0.005, 0.006, 0.007, 0.008, 0.009, 0.01, 0.011,
                0.012, 0.013, 0.014, 0.015, 0.016, 0.017, 0.018, 0.019, 0.02, 0.021, 0.022, 0.023,
                0.024, 0.025, 0.026, 0.027, 0.028, 0.029, 0.03, 0.031, 0.032, 0.033, 0.034, 0.035,
                0.036, 0.037, 0.038, 0.039, 0.04, 0.041, 0.042, 0.043, 0.044, 0.045,
            ],
        })
    }

    #[test]
    fn test_filter_command3_left_trim() {
        run_filter_test(FilterTestCase {
            name: "filter left trim",
            input: "\n \tfoo bar baz\t\n ",
            options: FilterOptions::new().with_left_trimmed(),
            want_text: "foo bar baz\t\n ",
            want_thinking: "",
            want_tool_calls: vec![],
            want_citations: vec![],
            want_likelihoods: vec![0.002, 0.003, 0.004, 0.005],
        })
    }
    #[test]
    fn test_filter_command3_right_trim() {
        run_filter_test(FilterTestCase {
            name: "filter right trim",
            input: "\n \tfoo bar baz\t\n ",
            options: FilterOptions::new().with_right_trimmed(),
            want_text: "\n \tfoo bar baz",
            want_thinking: "",
            want_tool_calls: vec![],
            want_citations: vec![],
            want_likelihoods: vec![0.002, 0.003, 0.004],
        })
    }

    #[test]
    fn test_filter_command3_reasoning_and_citations() {
        run_filter_test(FilterTestCase {
            name: "reasoning and citation parsing",
            input: "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
            options: FilterOptions::new().cmd3(),
            want_text: "foo bar",
            want_thinking: "This is a rainbow emoji: 🌈",
            want_tool_calls: vec![],
            want_citations: vec![
                FilterCitation {
                    start_index: 18,
                    end_index: 26,
                    text: "emoji: 🌈".to_string(),
                    sources: vec![Source {
                        tool_call_index: 0,
                        tool_result_indices: vec![1],
                    }],
                    is_thinking: true,
                },
                FilterCitation {
                    start_index: 4,
                    end_index: 7,
                    text: "bar".to_string(),
                    sources: vec![
                        Source {
                            tool_call_index: 0,
                            tool_result_indices: vec![1, 2],
                        },
                        Source {
                            tool_call_index: 1,
                            tool_result_indices: vec![3, 4],
                        },
                    ],
                    is_thinking: false,
                },
            ],
            want_likelihoods: vec![
                0.001, 0.002, 0.003, 0.004, 0.007, 0.008, 0.009, 0.01, 0.011, 0.012, 0.024, 0.027,
                0.028,
            ],
        })
    }

    #[test]
    fn test_filter_command3_overlapping_citations() {
        run_filter_test(FilterTestCase {
            name: "overlapping citations", // This scenario is ambiguous - for now, we define the behavior but we should figure out a nice way to handle this
            input: "<|START_RESPONSE|>foo <co>bar <co>baz</co: 1:[1]> boo</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
            options: FilterOptions::new().cmd3(),
            want_text: "foo bar <co>baz boo</co: 0:[1,2],1:[3,4]>",
            want_thinking: "",
            want_tool_calls: vec![],
            want_citations: vec![FilterCitation {
                start_index: 4,
                end_index: 15,
                text: "bar <co>baz".to_string(),
                sources: vec![Source {
                    tool_call_index: 1,
                    tool_result_indices: vec![1],
                }],
                is_thinking: false,
            }],
            want_likelihoods: vec![
                0.001, 0.004, 0.005, 0.007, 0.008, 0.009, 0.018, 0.019, 0.02, 0.021, 0.022, 0.024,
                0.025, 0.026, 0.027, 0.028, 0.029, 0.03, 0.031, 0.032, 0.033, 0.034, 0.035,
            ],
        })
    }

    #[test]
    fn test_filter_command3_tool_simple() {
        run_filter_test(FilterTestCase {
            name: "tool use simple",
            input: r#"<|START_THINKING|>I will use the add tool to calculate the sum of 6 and 7.<|END_THINKING|><|START_ACTION|>[{"tool_call_id": "0", "tool_name": "add", "parameters": {"a": 6, "b": 7}}]<|END_ACTION|>"#,
            options: FilterOptions::new().cmd3(),
            want_text: "",
            want_thinking: "I will use the add tool to calculate the sum of 6 and 7.",
            want_citations: vec![],
            want_tool_calls: vec![FilterToolCallDelta {
                index: 0,
                id: "0".to_string(),
                name: "add".to_string(),
                param_delta: None,
                raw_param_delta: "{\"a\": 6, \"b\": 7}".to_string(),
            }],
            want_likelihoods: vec![
                0.001, 0.002, 0.003, 0.004, 0.005, 0.006, 0.007, 0.008, 0.009, 0.01, 0.011, 0.013,
                0.014, 0.016, 0.017,
            ],
        })
    }

    #[test]
    fn test_filter_command3_tool_multiple_calls() {
        run_filter_test(FilterTestCase {
            name: "tool use multiple calls",
            input: r#"<|START_THINKING|>I will search for United States and Canada in separate tool calls.<|END_THINKING|><|START_ACTION|>[{"tool_call_id": "0", "tool_name": "web_search", "parameters": {"query": "United States"}},{"tool_call_id": "1", "tool_name": "web_search", "parameters": {"query": "Canada"}}]<|END_ACTION|>"#,
            options: FilterOptions::new().cmd3(),
            want_text: "",
            want_thinking: "I will search for United States and Canada in separate tool calls.",
            want_citations: vec![],
            want_tool_calls: vec![
                FilterToolCallDelta {
                    index: 0,
                    id: "0".to_string(),
                    name: "web_search".to_string(),
                    param_delta: None,
                    raw_param_delta: "{\"query\": \"United States\"}".to_string(),
                },
                FilterToolCallDelta {
                    index: 1,
                    id: "1".to_string(),
                    name: "web_search".to_string(),
                    param_delta: None,
                    raw_param_delta: "{\"query\": \"Canada\"}".to_string(),
                },
            ],
            want_likelihoods: vec![
                0.001, 0.002, 0.003, 0.004, 0.005, 0.006, 0.007, 0.008, 0.009, 0.01, 0.011, 0.012,
                0.013,
            ],
        })
    }

    #[test]
    fn test_filter_command3_tool_multiple_calls_chunk_size() {
        run_filter_test(FilterTestCase {
            name: "tool use multiple calls with chunk size",
            input: r#"<|START_THINKING|>I will search for United States and Canada in separate tool calls.<|END_THINKING|><|START_ACTION|>[{"tool_call_id": "0", "tool_name": "web_search", "parameters": {"query": "United States"}},{"tool_call_id": "1", "tool_name": "web_search", "parameters": {"query": "Canada"}}]<|END_ACTION|>"#,
            options: FilterOptions::new().cmd3().with_chunk_size(10),
            want_text: "",
            want_thinking: "I will search for United States and Canada in separate tool calls.",
            want_citations: vec![],
            want_tool_calls: vec![
                FilterToolCallDelta {
                    index: 0,
                    id: "0".to_string(),
                    name: "web_search".to_string(),
                    param_delta: None,
                    raw_param_delta: "{\"query\": \"United States\"}".to_string(),
                },
                FilterToolCallDelta {
                    index: 1,
                    id: "1".to_string(),
                    name: "web_search".to_string(),
                    param_delta: None,
                    raw_param_delta: "{\"query\": \"Canada\"}".to_string(),
                },
            ],
            want_likelihoods: vec![
                0.001, 0.002, 0.003, 0.004, 0.005, 0.006, 0.007, 0.008, 0.009, 0.01,
            ],
        })
    }

    #[test]
    fn test_filter_command3_skip_tool_parsing() {
        run_filter_test(FilterTestCase {
            name: "skip tool parsing",
            input: r#"<|START_THINKING|>I will use the add tool to calculate the sum of 6 and 7.<|END_THINKING|><|START_ACTION|>[{"tool_call_id": "0", "tool_name": "add", "parameters": {"a": 6, "b": 7}}]<|END_ACTION|>"#,
            options: FilterOptions::new()
                .cmd3()
                .remove_token("<|START_ACTION|>")
                .remove_token("<|END_ACTION|>"),
            want_text: "<|START_ACTION|>[{\"tool_call_id\": \"0\", \"tool_name\": \"add\", \"parameters\": {\"a\": 6, \"b\": 7}}]<|END_ACTION|>",
            want_thinking: "I will use the add tool to calculate the sum of 6 and 7.",
            want_citations: vec![],
            want_tool_calls: vec![],
            want_likelihoods: vec![
                0.001, 0.002, 0.003, 0.004, 0.005, 0.006, 0.007, 0.008, 0.009, 0.01, 0.011, 0.013,
                0.014, 0.016, 0.017, 0.019, 0.02, 0.021, 0.022, 0.023, 0.024, 0.025, 0.026, 0.027,
                0.028, 0.029, 0.03, 0.031, 0.032, 0.033, 0.034, 0.035, 0.036, 0.037, 0.038, 0.039,
                0.04, 0.041, 0.042, 0.043, 0.044, 0.046, 0.047, 0.048, 0.049, 0.05, 0.052, 0.053,
                0.054, 0.055,
            ],
        })
    }

    #[test] // For VLLM, <|START_RESPONSE|> gets omitted because it's a special token (and thinking <|*_THINKING|> is not)
    fn test_filter_command3_handles_missing_start_response() {
        run_filter_test(FilterTestCase {
            name: "skipping <|START_RESPONSE|>",
            input: "<|START_THINKING|>Plan<|END_THINKING|>Response",
            options: FilterOptions::new().cmd3(),
            want_text: "Response",
            want_thinking: "Plan",
            want_citations: vec![],
            want_tool_calls: vec![],
            want_likelihoods: vec![0.001, 0.003],
        });
    }
}
