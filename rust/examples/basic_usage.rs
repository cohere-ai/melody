use melody_parsing::{Filter, FilterImpl, FilterOptions, TokenIDsWithLogProb};

fn main() {
    // Initialize logging
    env_logger::init();

    println!("=== Melody Parsing - Basic Usage Example ===\n");

    // Example 1: Basic filter with plain text
    println!("Example 1: Basic Filter");
    {
        let mut filter = FilterImpl::new();
        let options = FilterOptions::new()
            .with_left_trimmed()
            .with_right_trimmed();

        options.apply_to_filter(&mut filter);

        // Simulate citation text
        let citation_text = "Hello World!";
        let logprobs = TokenIDsWithLogProb {
            token_ids: vec![1, 2, 3],
            logprobs: vec![0.1, 0.2, 0.3],
        };

        let outputs = filter.write_decoded(citation_text, logprobs);
        for output in outputs {
            println!("  Text: {}", output.text);
            for citation in output.citations {
                println!(
                    "    Citation: {} (indices {}-{})",
                    citation.text, citation.start_index, citation.end_index
                );
            }
        }
    }

    println!();

    // Example 2: Filter with citations
    println!("Example 2: Citation Parsing");
    {
        let options = FilterOptions::new()
            .handle_multi_hop_cmd3()
            .stream_tool_actions();

        let mut filter = FilterImpl::new();
        options.apply_to_filter(&mut filter);

        // Simulate citation text
        let citation_text = "Hello <co: 1>world</co: 1>!";
        let logprobs = TokenIDsWithLogProb {
            token_ids: vec![1, 2, 3],
            logprobs: vec![0.1, 0.2, 0.3],
        };

        let outputs = filter.write_decoded(citation_text, logprobs);
        for output in outputs {
            println!("  Text: {}", output.text);
            for citation in output.citations {
                println!(
                    "    Citation: {} (indices {}-{})",
                    citation.text, citation.start_index, citation.end_index
                );
            }
        }
    }

    println!();

    // Example 3: Search query handling
    println!("Example 3: Search Query");
    {
        let options = FilterOptions::new().handle_search_query();

        let mut filter = FilterImpl::new();
        options.apply_to_filter(&mut filter);

        let search_text = "Search: machine learning";
        let logprobs = TokenIDsWithLogProb {
            token_ids: vec![5, 6],
            logprobs: vec![0.5, 0.6],
        };

        let outputs = filter.write_decoded(search_text, logprobs);
        for output in outputs {
            if let Some(ref query) = output.search_query {
                println!("  Search Query {}: {}", query.index, query.text);
            }
        }
    }

    println!();

    // Example 4: Stop tokens
    println!("Example 4: Stop Tokens");
    {
        let options = FilterOptions::new()
            .with_inclusive_stops(vec!["<|END|>".to_string()])
            .with_exclusive_stops(vec!["</s>".to_string()]);

        let mut filter = FilterImpl::new();
        options.apply_to_filter(&mut filter);

        let text_with_stop = "Hello world<|END|>";
        let logprobs = TokenIDsWithLogProb::new();

        let outputs = filter.write_decoded(text_with_stop, logprobs);
        for output in outputs {
            println!("  Output: {}", output.text);
        }
    }

    println!("\n=== Examples Complete ===");
}
