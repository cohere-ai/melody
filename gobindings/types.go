package gobindings

// TokenIDsWithLogProb pairs tokens with their log probabilities
type TokenIDsWithLogProb struct {
	TokenIDs []uint32
	Logprobs []float32
}

// FilterOutput represents a partial parsed output from a model generation
type FilterOutput struct {
	Text          string
	Logprobs      TokenIDsWithLogProb
	SearchQuery   *FilterSearchQueryDelta
	Citations     []FilterCitation
	ToolCallDelta     *FilterToolCallDelta
	IsPostAnswer  bool
	IsReasoning bool
}

// FilterSearchQueryDelta represents a change to a search query
type FilterSearchQueryDelta struct {
	Index int
	Text  string
}

// FilterToolCallDelta represents a change to a tool call
type FilterToolCallDelta struct {
	Index         int
	ID            string
	Name          string
	ParamDelta    *FilterToolParameter
	RawParamDelta string
}

// FilterToolParameter represents a change to a tool parameter
type FilterToolParameter struct {
	Name       string
	ValueDelta string
}

// FilterCitation represents a citation parsed from a model generation
type FilterCitation struct {
	// The beginning index of the citation in the larger generation.
	// E.g. "Hello world" where the citation is "world" would have a StartIndex of 6.
	StartIndex int
	// The end index of the citation in the larger generation.
	// E.g. "Hello world" where the citation is "world" would have an EndIndex of 10.
	EndIndex   int
	Text       string
	Sources    []Source
	IsThinking bool
}

// Source indicates which tool call and which tool results from that tool are being cited
type Source struct {
	ToolCallIndex     int
	ToolResultIndices []int
}
