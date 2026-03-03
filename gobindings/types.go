package gobindings

// AggregatedResult is the result of a write_decoded or flush_partials call
type AggregatedResult struct {
	Content       *string
	Reasoning     *string
	ToolCalls     []AccumulatedToolCall
	Citations     []FilterCitation
	SearchQueries []SearchQueryDelta
}

// AccumulatedToolCall represents a tool call (possibly partial in streaming)
type AccumulatedToolCall struct {
	Index     uint
	ID        string
	Name      string
	Arguments string
}

// SearchQueryDelta represents a search query update
type SearchQueryDelta struct {
	Index uint
	Text  string
}

// FilterCitation represents a citation parsed from a model generation
type FilterCitation struct {
	// The beginning index of the citation in the larger generation.
	// E.g. "Hello world" where the citation is "world" would have a StartIndex of 6.
	StartIndex uint `json:"start_index"`
	// The end index of the citation in the larger generation.
	// E.g. "Hello world" where the citation is "world" would have an EndIndex of 10.
	EndIndex   uint     `json:"end_index"`
	Text       string   `json:"text"`
	Sources    []Source `json:"sources"`
	IsThinking bool     `json:"is_thinking"`
}

// Source indicates which tool call and which tool results from that tool are being cited
type Source struct {
	ToolCallIndex     uint   `json:"tool_call_index"`
	ToolResultIndices []uint `json:"tool_result_indices"`
}
