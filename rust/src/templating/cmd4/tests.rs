use super::*;
use crate::templating::melody_types::*;

#[test]
fn test_cmd4_1_message() {
    let messages = vec![Message {
        role: Role::User,
        content: vec![Content {
            content_type: ContentType::Text,
            text: "Hello.".to_string(),
            thinking: String::new(),
            image: None,
        }],
        tool_calls: Vec::new(),
        tool_call_id: String::new(),
        additional_fields: std::collections::HashMap::new(),
    }];

    let options = OptionsBuilder::new().build();

    let result = render(messages, options).expect("Failed to render");

    let expected = include_str!("test_data/1_message.txt");
    assert_eq!(expected, result);
}
