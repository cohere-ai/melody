package parsing

// Decoder represents a basic tokenizer interface required for the filter to accept tokens.
type Decoder interface {
	Decode(tokens []int64, skipSpecialTokens bool) (string, error)
}

// TokenIDsWithLogProb is a struct that pairs tokens with their log probabilities.
type TokenIDsWithLogProb struct {
	TokenIDs []int64
	Logprobs []float32
}

func (t *TokenIDsWithLogProb) append(other TokenIDsWithLogProb) {
	t.TokenIDs = append(t.TokenIDs, other.TokenIDs...)
	t.Logprobs = append(t.Logprobs, other.Logprobs...)
}

type fulltextwithlogprobs struct {
	Text     []byte
	Logprobs TokenIDsWithLogProb
}

// FilterOutput represents a partial parsed output from a model generation.
type FilterOutput struct {
	Text          string
	Logprobs      TokenIDsWithLogProb
	SearchQuery   *FilterSearchQueryDelta
	Citations     []FilterCitation
	ToolCalls     *FilterToolCallDelta
	IsPostAnswer  bool
	IsToolsReason bool
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

// FilterCitation represents a citation parsed from a model generation.
type FilterCitation struct {
	// The beginning index of the citation in the larger generation
	// E.g. "Hello world" where the citation is "world" would have an StartIndex of 6
	StartIndex int
	// The end index of the citation in the larger generation.
	// E.g. "Hello world" where the citation is "world" would have an EndIndex of 10
	EndIndex int
	Text     string
	Sources  []Source

	IsThinking bool
}

// Source indicates which tool call and which tool results from that tool are being cited
type Source struct {
	ToolCallIndex     int
	ToolResultIndices []int
}

type filterMode struct{ e uint }

var (
	plainText       = filterMode{0}
	ignore          = filterMode{1}
	toolAction      = filterMode{2}
	toolReason      = filterMode{3}
	answer          = filterMode{4}
	groundedAnswer  = filterMode{5}
	inclusiveStop   = filterMode{6}
	exclusiveStop   = filterMode{7}
	searchQuery     = filterMode{8}
	nextSearchQuery = filterMode{9}
)
