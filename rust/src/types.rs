/// TokenIDsWithLogProb pairs tokens with their log probabilities.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenIDsWithLogProb {
    pub token_ids: Vec<u32>,
    pub logprobs: Vec<f32>,
}

impl TokenIDsWithLogProb {
    pub fn new() -> Self {
        Self {
            token_ids: Vec::new(),
            logprobs: Vec::new(),
        }
    }

    pub fn append(&mut self, other: TokenIDsWithLogProb) {
        self.token_ids.extend(other.token_ids);
        self.logprobs.extend(other.logprobs);
    }
}

impl Default for TokenIDsWithLogProb {
    fn default() -> Self {
        Self::new()
    }
}

/// FilterOutput represents a partial parsed output from a model generation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilterOutput {
    pub text: String,
    pub logprobs: TokenIDsWithLogProb,
    pub search_query: Option<FilterSearchQueryDelta>,
    pub citations: Vec<FilterCitation>,
    pub tool_calls: Option<FilterToolCallDelta>,
    pub is_post_answer: bool,
    pub is_tools_reason: bool,
}

/// FilterSearchQueryDelta represents a change to a search query.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterSearchQueryDelta {
    pub index: usize,
    pub text: String,
}

/// FilterToolCallDelta represents a change to a tool call.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilterToolCallDelta {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub param_delta: Option<FilterToolParameter>,
    pub raw_param_delta: String,
}

/// FilterToolParameter represents a change to a tool parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterToolParameter {
    pub name: String,
    pub value_delta: String,
}

/// FilterCitation represents a citation parsed from a model generation.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterCitation {
    /// The beginning index of the citation in the larger generation.
    /// E.g. "Hello world" where the citation is "world" would have a start_index of 6.
    pub start_index: usize,
    /// The end index of the citation in the larger generation.
    /// E.g. "Hello world" where the citation is "world" would have an end_index of 10.
    pub end_index: usize,
    pub text: String,
    pub sources: Vec<Source>,
    pub is_thinking: bool,
}

/// Source indicates which tool call and which tool results from that tool are being cited.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub tool_call_index: usize,
    pub tool_result_indices: Vec<usize>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum FilterMode {
    PlainText,
    Ignore,
    ToolAction,
    ToolReason,
    Answer,
    GroundedAnswer,
    InclusiveStop,
    ExclusiveStop,
    SearchQuery,
    NextSearchQuery,
}
