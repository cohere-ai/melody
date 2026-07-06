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
	Index           uint
	ID              string
	Name            string
	Arguments       string
	ProcessedParams []FilterToolParameter
}

// FilterToolParameter represents a change to a tool parameter
type FilterToolParameter struct {
	Name       string
	ValueDelta string
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
	// DocumentIDs are the original document identifiers that ToolResultIndices
	// resolve back to, populated when the filter is configured with a document
	// ID lookup table (see FilterOptions.WithDocumentIDs). Same length as
	// ToolResultIndices when populated; nil otherwise.
	DocumentIDs []string `json:"document_ids,omitempty"`
}

// RenderOutput is the result of RenderCMD3Detailed / RenderCMD4Detailed.
//
// The two identifier lookup tables describe how the templating engine
// numbered documents and tool calls, so callers can convert back and forth
// between their own string identifiers and the numeric indices the model
// emits inside citations:
//
//   - DocumentIDs[toolCallIndex][toolResultIndex] yields the original `id`
//     field of the document at that prompt position. Feed it straight into
//     FilterOptions.WithDocumentIDs to have the parser populate
//     Source.DocumentIDs.
//   - ToolCallIDs[toolCallIndex] yields the original tool_call_id string.
//     Empty string at index 0 when a top-level Documents array was passed
//     (its "virtual" tool-call bucket).
type RenderOutput struct {
	Prompt      string     `json:"prompt"`
	DocumentIDs [][]string `json:"document_ids"`
	ToolCallIDs []string   `json:"tool_call_ids"`
}
