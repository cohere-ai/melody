#[cfg(test)]
mod tests {
    use crate::filter::{Filter, FilterImpl, find_partial};
    use crate::options::FilterOptions;
    use crate::types::*;

    use tokenizers::Tokenizer;

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

    #[test]
    fn test_citations_standard_case() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let mut filter = FilterImpl::new(tokenizer);
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = -1;

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
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let mut filter = FilterImpl::new(tokenizer);
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = -1;

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
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let mut filter = FilterImpl::new(tokenizer);
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = -1;

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
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let mut filter = FilterImpl::new(tokenizer);
        filter.left_trimmed = true;

        let (result, rem) = filter.trim_space("   hello");
        assert_eq!(result, "hello");
        assert_eq!(rem, 0);
        assert!(!filter.left_trimmed); // Should be reset after trimming
    }

    #[test]
    fn test_trim_space_right() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let mut filter = FilterImpl::new(tokenizer);
        filter.right_trimmed = true;

        let (result, rem) = filter.trim_space("hello   ");
        assert_eq!(result, "hello");
        assert_eq!(rem, 3);
    }

    #[test]
    fn test_trim_space_both() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let mut filter = FilterImpl::new(tokenizer);
        filter.left_trimmed = true;
        filter.right_trimmed = true;

        let (result, rem) = filter.trim_space("   hello   ");
        assert_eq!(result, "hello");
        assert_eq!(rem, 3);
    }

    #[test]
    fn test_trim_prefix() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let mut filter = FilterImpl::new(tokenizer);
        filter.trim_prefix = "prefix:".to_string();

        let (result, rem) = filter.trim_space("prefix:hello");
        assert_eq!(result, "hello");
        assert_eq!(rem, 0);
        assert!(filter.trim_prefix.is_empty());
    }

    #[test]
    fn test_trim_prefix_partial() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let mut filter = FilterImpl::new(tokenizer);
        filter.trim_prefix = "prefix:".to_string();

        let (result, rem) = filter.trim_space("pre");
        assert_eq!(result, "");
        assert_eq!(rem, 3);
        assert_eq!(filter.trim_prefix, "prefix:"); // Should not be cleared
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
    fn test_filter_options_handle_multi_hop_cmd3() {
        let options = FilterOptions::new()
            .handle_multi_hop_cmd3()
            .stream_tool_actions();

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
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let mut filter = FilterImpl::new(tokenizer);

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
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let mut filter = FilterImpl::new(tokenizer);
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
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let filter = FilterImpl::new(tokenizer);

        let outputs = filter.handle_inclusive_stop("hello<|END|>", 5, "<|END|>");
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].text, "hello<|END|>");
    }

    #[test]
    fn test_handle_exclusive_stop() {
        let tokenizer = Tokenizer::from_file(format!(
            "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let mut filter = FilterImpl::new(tokenizer);

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

    #[test]
    fn test_repetition_detection() {
        use crate::filter::hash_tokens_for_repetition_check;

        // Test hash function produces consistent results
        let seq1 = vec![1, 2, 3];
        let seq2 = vec![1, 2, 3];
        let seq3 = vec![1, 2, 4];

        assert_eq!(
            hash_tokens_for_repetition_check(&seq1),
            hash_tokens_for_repetition_check(&seq2)
        );
        assert_ne!(
            hash_tokens_for_repetition_check(&seq1),
            hash_tokens_for_repetition_check(&seq3)
        );
    }

    #[test]
    fn test_filter_command3() {
        // for simplicity's sake lets just generate likelihoods in intervals of thousandths
        let mut test_likelihoods: Vec<f32> = Vec::new();
        for i in 0..999 {
            test_likelihoods.push(i as f32 / 1000.0);
        }

        struct TestCase {
            name: &'static str,
            input: &'static str,
            options: FilterOptions,
            want: Vec<FilterOutput>,
        }

        let test_cases = vec![
            TestCase {
                name: "basic test",
                input: "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
                options: FilterOptions::new(),
                want: vec![
                    FilterOutput {
                        text: "<|START_THINKING|>".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![255019],
                            logprobs: vec![0.0],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "This".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![4184],
                            logprobs: vec![0.001],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: " is".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![1801],
                            logprobs: vec![0.002],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: " a".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![1671],
                            logprobs: vec![0.003],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: " rainbow".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![84470],
                            logprobs: vec![0.004],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: " <".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![2154],
                            logprobs: vec![0.005],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "co".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![2567],
                            logprobs: vec![0.006],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: ">".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![37],
                            logprobs: vec![0.007],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "emoji".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![104150],
                            logprobs: vec![0.008],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: ":".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![33],
                            logprobs: vec![0.009],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: " 🌈".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![11254, 242, 238],
                            logprobs: vec![0.01, 0.011, 0.012],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "</".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![1965],
                            logprobs: vec![0.013],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "co".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![2567],
                            logprobs: vec![0.014],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: ":".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![33],
                            logprobs: vec![0.015],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: " ".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![228],
                            logprobs: vec![0.016],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "0".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![23],
                            logprobs: vec![0.017],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: ":[".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![50706],
                            logprobs: vec![0.018],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "1".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![24],
                            logprobs: vec![0.019],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "]>".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![70118],
                            logprobs: vec![0.020],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "<|END_THINKING|>".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![255020],
                            logprobs: vec![0.021],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "\n".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![206],
                            logprobs: vec![0.022],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "<|START_RESPONSE|>".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![255021],
                            logprobs: vec![0.023],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "foo".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![15579],
                            logprobs: vec![0.024],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: " <".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![2154],
                            logprobs: vec![0.025],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "co".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![2567],
                            logprobs: vec![0.026],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: ">".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![37],
                            logprobs: vec![0.027],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "bar".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![4962],
                            logprobs: vec![0.028],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "</".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![1965],
                            logprobs: vec![0.029],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "co".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![2567],
                            logprobs: vec![0.030],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: ":".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![33],
                            logprobs: vec![0.031],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: " ".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![228],
                            logprobs: vec![0.032],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "0".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![23],
                            logprobs: vec![0.033],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: ":[".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![50706],
                            logprobs: vec![0.034],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "1".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![24],
                            logprobs: vec![0.035],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: ",".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![19],
                            logprobs: vec![0.036],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "2".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![25],
                            logprobs: vec![0.037],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "],".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![4085],
                            logprobs: vec![0.038],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "1".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![24],
                            logprobs: vec![0.039],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: ":[".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![50706],
                            logprobs: vec![0.040],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "3".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![26],
                            logprobs: vec![0.041],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: ",".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![19],
                            logprobs: vec![0.042],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "4".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![27],
                            logprobs: vec![0.043],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "]>".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![70118],
                            logprobs: vec![0.044],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "<|END_RESPONSE|>".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![255022],
                            logprobs: vec![0.045],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                ],
            },
            TestCase {
                name: "With command 3 parsing",
                input: "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
                options: FilterOptions::new()
                    .handle_multi_hop_cmd3()
                    .stream_tool_actions(),
                want: vec![
                    FilterOutput {
                        text: "This".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![4184],
                            logprobs: vec![0.001],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: true,
                    },
                    FilterOutput {
                        text: " is".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![1801],
                            logprobs: vec![0.002],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: true,
                    },
                    FilterOutput {
                        text: " a".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![1671],
                            logprobs: vec![0.003],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: true,
                    },
                    FilterOutput {
                        text: " rainbow".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![84470],
                            logprobs: vec![0.004],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: true,
                    },
                    FilterOutput {
                        text: " ".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![37],
                            logprobs: vec![0.007],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: true,
                    },
                    FilterOutput {
                        text: "emoji".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![104150],
                            logprobs: vec![0.008],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: true,
                    },
                    FilterOutput {
                        text: ":".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![33],
                            logprobs: vec![0.009],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: true,
                    },
                    FilterOutput {
                        text: " 🌈".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![11254, 242, 238],
                            logprobs: vec![0.010, 0.011, 0.012],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: true,
                    },
                    FilterOutput {
                        text: String::new(),
                        logprobs: TokenIDsWithLogProb::new(),
                        search_query: None,
                        citations: vec![FilterCitation {
                            start_index: 18,
                            end_index: 26,
                            text: "emoji: 🌈".to_string(),
                            sources: vec![Source {
                                tool_call_index: 0,
                                tool_result_indices: vec![1],
                            }],
                            is_thinking: true,
                        }],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "foo".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![15579],
                            logprobs: vec![0.024],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: " ".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![37],
                            logprobs: vec![0.027],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: "bar".to_string(),
                        logprobs: TokenIDsWithLogProb {
                            token_ids: vec![4962],
                            logprobs: vec![0.028],
                        },
                        search_query: None,
                        citations: vec![],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                    FilterOutput {
                        text: String::new(),
                        logprobs: TokenIDsWithLogProb::new(),
                        search_query: None,
                        citations: vec![FilterCitation {
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
                        }],
                        tool_calls: None,
                        is_post_answer: false,
                        is_tools_reason: false,
                    },
                ],
            },
        ];

        for tt in test_cases {
            let tokenizer = Tokenizer::from_file(format!(
                "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap();

            let mut filter = crate::options::new_filter(tokenizer, tt.options.clone());

            let tokenizer_for_encode = Tokenizer::from_file(format!(
                "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap();

            let encoding = tokenizer_for_encode.encode(tt.input, false).unwrap();
            let tokens = encoding.get_ids();

            let mut out: Vec<FilterOutput> = Vec::new();
            for (i, &token) in tokens.iter().enumerate() {
                let o = filter.write(token, Some(test_likelihoods[i])).unwrap();
                out.extend(o);
            }

            assert_eq!(out, tt.want, "Test case '{}' failed", tt.name);

            // Duplicate the test by writing the raw strings instead
            let mut text_chunks: Vec<String> = Vec::new();
            let mut buffer: Vec<u32> = Vec::new();
            let mut likelihoods_chunks: Vec<TokenIDsWithLogProb> = Vec::new();
            let mut likelihood_buffer: Vec<f32> = Vec::new();

            for (i, &token) in tokens.iter().enumerate() {
                buffer.push(token);
                likelihood_buffer.push(test_likelihoods[i]);

                let decoded = tokenizer_for_encode.decode(&buffer, false).unwrap();

                if decoded.ends_with('\u{fffd}') {
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

            let tokenizer_for_decode = Tokenizer::from_file(format!(
                "{}/tokenizers/data/multilingual+255k+bos+eos+sptok+fim+agents3.json",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap();

            let mut filter = crate::options::new_filter(tokenizer_for_decode, tt.options);
            let mut out: Vec<FilterOutput> = Vec::new();
            for (i, chunk) in text_chunks.iter().enumerate() {
                out.extend(filter.write_decoded(chunk, likelihoods_chunks[i].clone()));
            }

            assert_eq!(
                out, tt.want,
                "Test case '{}' (WriteDecoded) failed",
                tt.name
            );
        }
    }
}
