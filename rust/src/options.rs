use crate::filter::FilterImpl;
use crate::types::FilterMode;
use std::collections::HashMap;

#[derive(Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct FilterOptions {
    pub(crate) left_trimmed: bool,
    pub(crate) right_trimmed: bool,
    pub(crate) trim_prefix: String,
    pub(crate) inclusive_stops: Vec<String>,
    pub(crate) exclusive_stops: Vec<String>,
    pub(crate) chunk_size: usize,
    pub(crate) special_token_map: HashMap<String, FilterMode>,
    pub(crate) default_mode: FilterMode,
    pub(crate) stream_non_grounded_answer: bool,
    pub(crate) stream_tool_actions: bool,
    pub(crate) stream_processed_params: bool,
    pub(crate) has_tool_call_id: bool,
    pub(crate) cmd3_citations: bool,
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self {
            left_trimmed: false,
            right_trimmed: false,
            trim_prefix: String::new(),
            inclusive_stops: Vec::new(),
            exclusive_stops: Vec::new(),
            chunk_size: 1,
            special_token_map: HashMap::new(),
            default_mode: FilterMode::PlainText,
            stream_non_grounded_answer: false,
            stream_tool_actions: false,
            stream_processed_params: false,
            has_tool_call_id: false,
            cmd3_citations: false,
        }
    }
}

impl FilterOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_left_trimmed(mut self) -> Self {
        self.left_trimmed = true;
        self
    }

    #[must_use]
    pub fn with_right_trimmed(mut self) -> Self {
        self.right_trimmed = true;
        self
    }

    #[must_use]
    pub fn with_prefix_trim(mut self, prefix: String) -> Self {
        self.trim_prefix = prefix;
        self
    }

    #[must_use]
    pub fn with_inclusive_stops(mut self, stops: Vec<String>) -> Self {
        self.inclusive_stops = stops;
        self
    }

    #[must_use]
    pub fn with_exclusive_stops(mut self, stops: Vec<String>) -> Self {
        self.exclusive_stops = stops;
        self
    }

    #[must_use]
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    #[must_use]
    pub fn handle_rag(mut self) -> Self {
        self.default_mode = FilterMode::Ignore;
        self.right_trimmed = true;
        self.special_token_map
            .insert("Grounded answer:".to_string(), FilterMode::GroundedAnswer);
        self.special_token_map
            .insert("Answer:".to_string(), FilterMode::Answer);
        self
    }

    #[must_use]
    pub fn handle_search_query(mut self) -> Self {
        self.default_mode = FilterMode::Ignore;
        self.right_trimmed = true;
        self.special_token_map
            .insert("Search:".to_string(), FilterMode::SearchQuery);
        self.special_token_map
            .insert("|||".to_string(), FilterMode::NextSearchQuery);
        self.special_token_map
            .insert("\n".to_string(), FilterMode::NextSearchQuery);
        self
    }

    #[must_use]
    pub fn handle_multi_hop(mut self) -> Self {
        self.default_mode = FilterMode::Ignore;
        self.right_trimmed = true;
        self.special_token_map
            .insert("Grounded answer:".to_string(), FilterMode::GroundedAnswer);
        self.special_token_map
            .insert("Answer:".to_string(), FilterMode::Answer);
        self.special_token_map
            .insert("Plan:".to_string(), FilterMode::ToolReason);
        self.special_token_map
            .insert("Reflection:".to_string(), FilterMode::ToolReason);
        self.special_token_map
            .insert("Action:".to_string(), FilterMode::ToolAction);
        self.special_token_map
            .insert("Relevant Documents:".to_string(), FilterMode::Ignore);
        self.special_token_map
            .insert("Cited Documents:".to_string(), FilterMode::Ignore);
        self
    }

    #[must_use]
    pub fn handle_multi_hop_cmd3(mut self) -> Self {
        self.default_mode = FilterMode::GroundedAnswer;
        self.right_trimmed = true;
        self.has_tool_call_id = true;
        self.cmd3_citations = true;
        self.special_token_map
            .insert("<|START_RESPONSE|>".to_string(), FilterMode::GroundedAnswer);
        self.special_token_map
            .insert("<|END_RESPONSE|>".to_string(), FilterMode::Ignore);
        self.special_token_map
            .insert("<|START_THINKING|>".to_string(), FilterMode::ToolReason);
        self.special_token_map
            .insert("<|END_THINKING|>".to_string(), FilterMode::Ignore);
        self.special_token_map
            .insert("<|START_ACTION|>".to_string(), FilterMode::ToolAction);
        self.special_token_map
            .insert("<|END_ACTION|>".to_string(), FilterMode::Ignore);
        self
    }

    #[must_use]
    pub fn handle_multi_hop_cmd4(mut self) -> Self {
        self.default_mode = FilterMode::GroundedAnswer;
        self.right_trimmed = true;
        self.has_tool_call_id = true;
        self.cmd3_citations = true;
        self.special_token_map
            .insert("<|START_TEXT|>".to_string(), FilterMode::GroundedAnswer);
        self.special_token_map
            .insert("<|END_TEXT|>".to_string(), FilterMode::Ignore);
        self.special_token_map
            .insert("<|START_THINKING|>".to_string(), FilterMode::ToolReason);
        self.special_token_map
            .insert("<|END_THINKING|>".to_string(), FilterMode::Ignore);
        self.special_token_map
            .insert("<|START_ACTION|>".to_string(), FilterMode::ToolAction);
        self.special_token_map
            .insert("<|END_ACTION|>".to_string(), FilterMode::Ignore);
        self
    }

    #[must_use]
    pub fn stream_non_grounded_answer(mut self) -> Self {
        self.stream_non_grounded_answer = true;
        self
    }

    #[must_use]
    pub fn stream_tool_actions(mut self) -> Self {
        self.stream_tool_actions = true;
        self
    }

    #[must_use]
    pub fn stream_processed_params(mut self) -> Self {
        self.stream_processed_params = true;
        self
    }

    #[must_use]
    pub fn remove_token(mut self, token: &str) -> Self {
        self.special_token_map.remove(token);
        self
    }

    pub(crate) fn apply_to_filter(self, filter: &mut FilterImpl) {
        filter.left_trimmed = self.left_trimmed;
        filter.right_trimmed = self.right_trimmed;
        filter.trim_prefix = self.trim_prefix;
        filter.chunk_size = self.chunk_size;
        filter.stream_non_grounded_answer = self.stream_non_grounded_answer;
        filter.stream_tool_actions = self.stream_tool_actions;
        filter.stream_processed_params = self.stream_processed_params;
        filter.has_tool_call_id = self.has_tool_call_id;
        filter.cmd3_citations = self.cmd3_citations;
        filter.default_mode = self.default_mode;
        filter.mode = self.default_mode;

        // Merge special token maps
        for (token, mode) in self.special_token_map {
            filter.special_token_map.insert(token, mode);
        }

        // Add inclusive stops
        for stop in self.inclusive_stops {
            filter
                .special_token_map
                .insert(stop, FilterMode::InclusiveStop);
        }

        // Add exclusive stops
        for stop in self.exclusive_stops {
            filter
                .special_token_map
                .insert(stop, FilterMode::ExclusiveStop);
        }

        // Update special token keys
        filter.special_token_keys = filter.special_token_map.keys().cloned().collect();
    }
}

#[must_use]
pub fn new_filter(options: FilterOptions) -> FilterImpl {
    let mut filter = FilterImpl::new();
    options.apply_to_filter(&mut filter);
    filter
}
