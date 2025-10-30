package tokenizers

import (
	_ "embed"
	"fmt"
	"sync"

	hftokenizers "github.com/cohere-ai/tokenizers"
)

var (
	mux          sync.RWMutex
	hfTokenizers map[string]Tokenizer
)

func init() {
	mux = sync.RWMutex{}
	hfTokenizers = map[string]Tokenizer{}
}

type Tokenizer interface {
	EncodeUint32(text string, opts ...EncodeOption) ([]uint32, error)
	DecodeUint32(tokens []uint32, skipSpecialTokens bool) (string, error)
	EncodeBatchUint32(texts []string, opts ...EncodeOption) ([][]uint32, error)
	DecodeBatchUint32(tokens [][]uint32, skipSpecialTokens bool) ([]string, error)
	VocabSize() (int64, error)
	ID() string
}

type encodeParams struct {
	truncationLength *int
	truncationMode   *hftokenizers.TruncationDirection
	addSpecialTokens bool
}

type EncodeOption func(*encodeParams)

func WithTruncationLength(length int) EncodeOption {
	return func(params *encodeParams) {
		params.truncationLength = &length
	}
}

func WithLeftTruncation() EncodeOption {
	return func(params *encodeParams) {
		t := hftokenizers.TruncationDirectionLeft
		params.truncationMode = &t
	}
}

func WithRightTruncation() EncodeOption {
	return func(params *encodeParams) {
		t := hftokenizers.TruncationDirectionRight
		params.truncationMode = &t
	}
}

func NoSpecialTokens() EncodeOption {
	return func(params *encodeParams) {
		params.addSpecialTokens = false
	}
}

type TokenizerProvider interface {
	GetTokenizer(tokenizerID string) (Tokenizer, error)
}

type tokenizerProvider struct{}

type tokenizerCfg struct {
	data     []byte
	filename string
}

// Tokenizer paths relative to the file
var (
	//go:embed data/75k+bos+eos+eop.json
	tokenizer75kBosEosEop []byte
	//go:embed data/co.json
	tokenizerCo []byte
	//go:embed data/multilingual+255k+bos+eos+sptok+fim+agents3.json
	tokenizerCommand3 []byte

	configs = map[string]tokenizerCfg{
		"50k":             {data: tokenizerCo, filename: "co.json"},
		"75k+bos+eos+eop": {data: tokenizer75kBosEosEop, filename: "75k+bos+eos+eop.json"},
		"multilingual+255k+bos+eos+sptok+fim+agents3": {data: tokenizerCommand3, filename: "multilingual+255k+bos+eos+sptok+fim+agents3.json"},
	}
)

func (t *tokenizerProvider) GetTokenizer(tokenizerID string) (Tokenizer, error) {
	if tokenizerID == "" || tokenizerID == "cohere" {
		tokenizerID = "50k"
	}
	return t.huggingFaceTokenizer(tokenizerID)
}

// since this could be invoked concurrently, we need to protect the map and
// ensure that exactly one goroutine is creating the tokenizer.
func (t *tokenizerProvider) huggingFaceTokenizer(tokenizerID string) (Tokenizer, error) {
	mux.RLock()
	tokenizer, ok := hfTokenizers[tokenizerID]
	mux.RUnlock()
	if ok {
		return tokenizer, nil
	}
	mux.Lock()
	defer mux.Unlock()
	// handle 2nd goroutine to get the lock
	tokenizer, ok = hfTokenizers[tokenizerID]
	if ok {
		return tokenizer, nil
	}
	// actually create instance of tokenizer
	tkzrCfg, ok := configs[tokenizerID]
	if !ok {
		return nil, fmt.Errorf("unknown tokenizer: %s", tokenizerID)
	}
	tokenizer, err := newHuggingFaceTokenizer(tokenizerID, tkzrCfg.data)
	if err != nil {
		return nil, err
	}
	hfTokenizers[tokenizerID] = tokenizer
	return tokenizer, nil
}

// TODO this is a useless abstraction, ideally router would init it once and pass all configs
func NewTokenizerProvider() TokenizerProvider {
	return &tokenizerProvider{}
}

// TODO shouldn't use useless TokenizerProvider, but some tests are using it
func GetTokenizer(tokenizerID string) (Tokenizer, error) {
	return NewTokenizerProvider().GetTokenizer(tokenizerID)
}

func GetTokenizerConfig(tokenizerID string) []byte {
	out := make([]byte, len(configs[tokenizerID].data))
	copy(out, configs[tokenizerID].data)
	return out
}

func GetTokenizerFilename(tokenizerID string) string {
	return configs[tokenizerID].filename
}
