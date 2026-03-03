use cohere_melody::parsing::Filter;
use cohere_melody::{parsing, templating};

fn main() {
    env_logger::init();

    println!("=== Melody Parsing - Basic Usage Example ===\n");

    println!("Example 1: Basic Filter");
    {
        let options = parsing::FilterOptions::new()
            .with_left_trimmed()
            .with_right_trimmed();

        let mut filter = parsing::new_filter(options);

        let citation_text = "Hello World!";
        let result = filter.write_decoded(citation_text);
        if let Some(ref text) = result.content {
            println!("  Content: {text}");
        }
        for citation in &result.citations {
            println!(
                "    Citation: {} (indices {}-{})",
                citation.text, citation.start_index, citation.end_index
            );
        }
    }

    println!();

    println!("Example 2: Citation Parsing");
    {
        let options = parsing::FilterOptions::new().cmd3();

        let mut filter = parsing::new_filter(options);

        let citation_text = "Hello <co: 1>world</co: 1>!";
        let result = filter.write_decoded(citation_text);
        if let Some(ref text) = result.content {
            println!("  Content: {text}");
        }
        for citation in &result.citations {
            println!(
                "    Citation: {} (indices {}-{})",
                citation.text, citation.start_index, citation.end_index
            );
        }
    }

    println!();

    println!("Example 3: Search Query");
    {
        let options = parsing::FilterOptions::new().handle_search_query();

        let mut filter = parsing::new_filter(options);

        let search_text = "Search: machine learning";
        let result = filter.write_decoded(search_text);
        for sq in &result.search_queries {
            println!("  Search Query {}: {}", sq.index, sq.text);
        }
    }

    println!();

    println!("Example 4: Stop Tokens");
    {
        let options = parsing::FilterOptions::new()
            .with_inclusive_stops(vec!["<|END|>".to_string()])
            .with_exclusive_stops(vec!["</s>".to_string()]);

        let mut filter = parsing::new_filter(options);

        let text_with_stop = "Hello world<|END|>";
        let result = filter.write_decoded(text_with_stop);
        if let Some(ref text) = result.content {
            println!("  Output: {text}");
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
        println!("Rendered CMD4 Prompt:\n{prompt}");
    }

    println!("\n=== Examples Complete ===");
}
