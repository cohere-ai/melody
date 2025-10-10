package parsing

type Decoder interface {
	Decode(tokens []int64, skipSpecialTokens bool) (string, error)
}

type TokenLikelihood struct {
	Token      int64
	Text       string
	Likelihood *float32
}

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

type FilterOutput struct {
	Text          string
	Logprobs      TokenIDsWithLogProb
	SearchQuery   *FilterSearchQueryDelta
	Citations     []FilterCitation
	ToolCalls     *FilterToolCallDelta
	IsPostAnswer  bool
	IsToolsReason bool
}

type FilterSearchQueryDelta struct {
	Index int
	Text  string
}

type FilterToolCallDelta struct {
	Index         int
	ID            string
	Name          string
	ParamDelta    *FilterToolParameter
	RawParamDelta string
}

type FilterToolParameter struct {
	Name       string
	ValueDelta string
}

type FilterCitation struct {
	// The beginning index of the citation in the larger generation
	// E.g. "Hello world" where the citation is "world" would have an StartIndex of 6
	StartIndex int
	// The end index of the citation in the larger generation.
	// E.g. "Hello world" where the citation is "world" would have an EndIndex of 10
	EndIndex   int
	Text       string
	DocIndices []DocIndex

	IsThinking bool
}

type DocIndex struct {
	ToolIndex     int
	ResultIndices []int
}

type FilterMode struct{ e uint }

var (
	PlainText       = FilterMode{0}
	Ignore          = FilterMode{1}
	ToolAction      = FilterMode{2}
	ToolReason      = FilterMode{3}
	Answer          = FilterMode{4}
	GroundedAnswer  = FilterMode{5}
	InclusiveStop   = FilterMode{6}
	ExclusiveStop   = FilterMode{7}
	SearchQuery     = FilterMode{8}
	NextSearchQuery = FilterMode{9}
)
