use cohere_melody::parsing::Filter;
use cohere_melody::*;

fn main() {
    // Initialize logging
    env_logger::init();

    println!("=== Melody Parsing - Basic Usage Example ===\n");

    // Example 1: Basic filter with plain text
    println!("Example 1: Basic Filter");
    {
        let options = parsing::FilterOptions::new()
            .with_left_trimmed()
            .with_right_trimmed();

        let mut filter = parsing::new_filter(options);

        // Simulate citation text
        let citation_text = "Hello World!";
        let logprobs = parsing::types::TokenIDsWithLogProb {
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
        let options = parsing::FilterOptions::new().cmd3();

        let mut filter = parsing::new_filter(options);

        // Simulate citation text
        let citation_text = "Hello <co: 1>world</co: 1>!";
        let logprobs = parsing::types::TokenIDsWithLogProb {
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
        let options = parsing::FilterOptions::new().handle_search_query();

        let mut filter = parsing::new_filter(options);

        let search_text = "Search: machine learning";
        let logprobs = parsing::types::TokenIDsWithLogProb {
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
        let options = parsing::FilterOptions::new()
            .with_inclusive_stops(vec!["<|END|>".to_string()])
            .with_exclusive_stops(vec!["</s>".to_string()]);

        let mut filter = parsing::new_filter(options);

        let text_with_stop = "Hello world<|END|>";
        let logprobs = parsing::types::TokenIDsWithLogProb::new();

        let outputs = filter.write_decoded(text_with_stop, logprobs);
        for output in outputs {
            println!("  Output: {}", output.text);
        }
    }

    println!("=== Melody Prompt Rendering - Basic Usage Example ===\n");
    {
        let options = templating::RenderCmd4Options {
            messages: vec![
                templating::types::Message {
                    role: templating::types::Role::System,
                    content: vec![templating::types::Content {
                        content_type: templating::types::ContentType::Text,
                        text: Some("You are a helpful assistant.".to_string()),
                        thinking: None,
                        image: None,
                        document: None,
                    }],
                    tool_calls: vec![],
                    tool_call_id: None,
                    citations: vec![],
                },
                templating::types::Message {
                    role: templating::types::Role::User,
                    content: vec![templating::types::Content {
                        content_type: templating::types::ContentType::Text,
                        text: Some("Hello Command!.".to_string()),
                        thinking: None,
                        image: None,
                        document: None,
                    }],
                    tool_calls: vec![],
                    tool_call_id: None,
                    citations: vec![],
                },
            ],
            ..Default::default()
        };

        let prompt = templating::render_cmd4(&options).unwrap();
        println!("Rendered CMD4 Prompt:\n{}", prompt);
    }

    println!("\n=== Examples Complete ===");
}
