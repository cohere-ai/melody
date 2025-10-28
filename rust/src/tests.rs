#[cfg(test)]
mod tests {
    use crate::filter::{FilterImpl, find_partial};
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
}
