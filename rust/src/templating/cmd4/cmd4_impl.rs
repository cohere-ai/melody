use crate::templating::melody_types::*;
use crate::templating::prompts;
use crate::templating::util::{escape_special_tokens, json_to_liquid, messages_to_template, safe_liquid_substitutions, tools_to_template};
use liquid::ParserBuilder;
use regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// Options for cmd4 rendering
#[derive(Debug, Clone)]
pub struct Options {
    pub template: String,
    pub dev_instruction: Option<String>,
    pub platform_instruction: String,
    pub documents: Vec<Document>,
    pub available_tools: Vec<Tool>,
    pub grounding: Grounding,
    pub response_prefix: String,
    pub escaped_special_tokens: HashMap<String, String>,
    pub json_schema: Option<String>,
    pub json_mode: bool,
    pub additional_template_fields: HashMap<String, Value>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            template: prompts::COMMAND4.to_string(),
            dev_instruction: None,
            platform_instruction: String::new(),
            documents: Vec::new(),
            available_tools: Vec::new(),
            grounding: Grounding::default(),
            response_prefix: String::new(),
            escaped_special_tokens: HashMap::new(),
            json_schema: None,
            json_mode: false,
            additional_template_fields: HashMap::new(),
        }
    }
}

/// Builder for cmd4 options
pub struct OptionsBuilder {
    options: Options,
}

impl OptionsBuilder {
    pub fn new() -> Self {
        OptionsBuilder {
            options: Options::default(),
        }
    }

    pub fn template(mut self, template: String) -> Self {
        self.options.template = template;
        self
    }

    pub fn developer_instruction(mut self, dev_instruction: Option<String>) -> Self {
        self.options.dev_instruction = dev_instruction;
        self
    }

    pub fn platform_instruction(mut self, platform_instruction: String) -> Self {
        self.options.platform_instruction = platform_instruction;
        self
    }

    pub fn documents(mut self, documents: Vec<Document>) -> Self {
        self.options.documents = documents;
        self
    }

    pub fn available_tools(mut self, available_tools: Vec<Tool>) -> Self {
        self.options.available_tools = available_tools;
        self
    }

    pub fn grounding(mut self, grounding: Grounding) -> Self {
        self.options.grounding = grounding;
        self
    }

    pub fn response_prefix(mut self, response_prefix: String) -> Self {
        self.options.response_prefix = response_prefix;
        self
    }

    pub fn json_schema(mut self, json_schema: Option<String>) -> Self {
        self.options.json_schema = json_schema;
        self
    }

    pub fn json_mode(mut self, json_mode: bool) -> Self {
        self.options.json_mode = json_mode;
        self
    }

    pub fn escaped_special_tokens(mut self, special_tokens: Vec<String>) -> Self {
        let re = Regex::new(r"([<>|])").unwrap();
        for token in special_tokens {
            let replacement = re.replace_all(&token, r"\$1").to_string();
            self.options
                .escaped_special_tokens
                .insert(token, replacement);
        }
        self
    }

    pub fn additional_template_fields(mut self, fields: HashMap<String, Value>) -> Self {
        self.options.additional_template_fields = fields;
        self
    }

    pub fn build(self) -> Options {
        self.options
    }
}

impl Default for OptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Render messages using Command 4 template
pub fn render(
    messages: Vec<Message>,
    options: Options,
) -> Result<String, Box<dyn std::error::Error>> {
    let template_tools = tools_to_template(&options.available_tools)?;

    let messages_template = messages_to_template(
        messages,
        !options.documents.is_empty(),
        &options.escaped_special_tokens,
    )?;

    let docs: Vec<String> = options
        .documents
        .iter()
        .map(|d| escape_special_tokens(d, &options.escaped_special_tokens))
        .collect();

    let mut substitutions = Map::new();

    // Add additional template fields first
    for (k, v) in &options.additional_template_fields {
        substitutions.insert(k.clone(), v.clone());
    }

    // Pre-compute grounding value with default
    let grounding_str = if matches!(options.grounding, Grounding::Unknown) {
        "DISABLED".to_string()
    } else {
        options.grounding.to_string()
    };

    // Pre-compute boolean flags for the template (Rust liquid doesn't support boolean expressions in assigns)
    let tools_exist = !options.available_tools.is_empty();
    let documents_exist = !options.documents.is_empty();
    let grounding_enabled = grounding_str == "ENABLED";
    let tools_or_documents_exist = tools_exist || documents_exist;
    let render_grounding = grounding_enabled && tools_or_documents_exist;
    let render_platform_instruction_override = !options.platform_instruction.is_empty();
    let render_developer_instruction = options.dev_instruction.is_some();

    // Add standard substitutions
    substitutions.insert(
        "developer_instruction".to_string(),
        json!(options.dev_instruction),
    );
    substitutions.insert(
        "platform_instruction_override".to_string(),
        json!(options.platform_instruction),
    );
    substitutions.insert("messages".to_string(), json!(messages_template));
    substitutions.insert("documents".to_string(), json!(docs));
    substitutions.insert("available_tools".to_string(), json!(template_tools));
    substitutions.insert("grounding".to_string(), json!(grounding_str));
    substitutions.insert(
        "response_prefix".to_string(),
        json!(options.response_prefix),
    );
    substitutions.insert("json_schema".to_string(), json!(options.json_schema));
    substitutions.insert("json_mode".to_string(), json!(options.json_mode));

    // Add pre-computed boolean flags
    substitutions.insert("tools_exist".to_string(), json!(tools_exist));
    substitutions.insert("documents_exist".to_string(), json!(documents_exist));
    substitutions.insert("render_docs".to_string(), json!(documents_exist));
    substitutions.insert("grounding_enabled".to_string(), json!(grounding_enabled));
    substitutions.insert(
        "tools_or_documents_exist".to_string(),
        json!(tools_or_documents_exist),
    );
    substitutions.insert("render_grounding".to_string(), json!(render_grounding));
    substitutions.insert(
        "render_platform_instruction_override".to_string(),
        json!(render_platform_instruction_override),
    );
    substitutions.insert(
        "render_developer_instruction".to_string(),
        json!(render_developer_instruction),
    );

    let safe_substitutions = safe_liquid_substitutions(substitutions);

    let parser = ParserBuilder::with_stdlib().build()?;
    let template = parser.parse(&options.template)?;

    // Convert serde_json::Map to liquid::Object
    let mut globals = liquid::Object::new();
    for (k, v) in safe_substitutions {
        globals.insert(k.into(), json_to_liquid(v));
    }

    let output = template.render(&globals)?;

    Ok(output)
}
