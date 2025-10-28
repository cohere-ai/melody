use crate::templating::melody_types::*;
use crate::templating::prompts;
use crate::templating::util::*;
use liquid::ParserBuilder;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// Options for cmd3 rendering
#[derive(Debug, Clone)]
pub struct Options {
    pub template: String,
    pub dev_instruction: Option<String>,
    pub documents: Vec<Document>,
    pub available_tools: Vec<Tool>,
    pub safety_mode: SafetyMode,
    pub citation_quality: CitationQuality,
    pub reasoning_type: ReasoningType,
    pub skip_preamble: bool,
    pub response_prefix: String,
    pub escaped_special_tokens: HashMap<String, String>,
    pub json_schema: Option<String>,
    pub json_mode: bool,
    pub additional_template_fields: HashMap<String, Value>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            template: prompts::COMMAND3.to_string(),
            dev_instruction: None,
            documents: Vec::new(),
            available_tools: Vec::new(),
            safety_mode: SafetyMode::default(),
            citation_quality: CitationQuality::default(),
            reasoning_type: ReasoningType::default(),
            skip_preamble: false,
            response_prefix: String::new(),
            escaped_special_tokens: HashMap::new(),
            json_schema: None,
            json_mode: false,
            additional_template_fields: HashMap::new(),
        }
    }
}

/// Builder for cmd3 options
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

    pub fn documents(mut self, documents: Vec<Document>) -> Self {
        self.options.documents = documents;
        self
    }

    pub fn available_tools(mut self, available_tools: Vec<Tool>) -> Self {
        self.options.available_tools = available_tools;
        self
    }

    pub fn citation_quality(mut self, citation_quality: CitationQuality) -> Self {
        self.options.citation_quality = citation_quality;
        self
    }

    pub fn safety_mode(mut self, safety_mode: SafetyMode) -> Self {
        self.options.safety_mode = safety_mode;
        self
    }

    pub fn reasoning_type(mut self, reasoning_type: ReasoningType) -> Self {
        self.options.reasoning_type = reasoning_type;
        self
    }

    pub fn skip_preamble(mut self, skip_preamble: bool) -> Self {
        self.options.skip_preamble = skip_preamble;
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

/// Render messages using Command 3 template
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

    // Pre-compute boolean flags for the template (Rust liquid doesn't support boolean expressions in assigns)
    let tools_exist = !options.available_tools.is_empty();
    let documents_exist = !options.documents.is_empty();
    let render_docs = documents_exist;
    let reasoning_enabled = options.reasoning_type == ReasoningType::Enabled;
    let preamble_exists = options.dev_instruction.is_some();

    // Add standard substitutions
    substitutions.insert("preamble".to_string(), json!(options.dev_instruction));
    substitutions.insert("messages".to_string(), json!(messages_template));
    substitutions.insert("documents".to_string(), json!(docs));
    substitutions.insert("available_tools".to_string(), json!(template_tools));
    substitutions.insert(
        "citation_mode".to_string(),
        json!(options.citation_quality.to_string()),
    );
    substitutions.insert(
        "safety_mode".to_string(),
        json!(options.safety_mode.to_string()),
    );
    substitutions.insert(
        "reasoning_options".to_string(),
        json!({
            "enabled": options.reasoning_type == ReasoningType::Enabled
        }),
    );
    substitutions.insert("skip_preamble".to_string(), json!(options.skip_preamble));
    substitutions.insert(
        "skip_thinking".to_string(),
        json!(options.reasoning_type == ReasoningType::Disabled),
    );
    substitutions.insert(
        "response_prefix".to_string(),
        json!(options.response_prefix),
    );
    substitutions.insert("json_schema".to_string(), json!(options.json_schema));
    substitutions.insert("json_mode".to_string(), json!(options.json_mode));

    // Add pre-computed boolean flags
    substitutions.insert("tools_exist".to_string(), json!(tools_exist));
    substitutions.insert("documents_exist".to_string(), json!(documents_exist));
    substitutions.insert("render_docs".to_string(), json!(render_docs));
    substitutions.insert("reasoning_enabled".to_string(), json!(reasoning_enabled));
    substitutions.insert("preamble_exists".to_string(), json!(preamble_exists));
    substitutions.insert("rendered_non_system".to_string(), json!(false));

    let parser = ParserBuilder::with_stdlib().build()?;
    let template = parser.parse(&options.template)?;

    // Convert serde_json::Map to liquid::Object
    let mut globals = liquid::Object::new();
    for (k, v) in substitutions {
        globals.insert(k.into(), json_to_liquid(v));
    }

    let output = template.render(&globals)?;

    Ok(output)
}
