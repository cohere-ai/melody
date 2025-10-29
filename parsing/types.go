package parsing

import (
	"fmt"
	"strings"
)

func reverse[A comparable, B comparable](m map[A]B) map[B]A {
	reversed := make(map[B]A)
	for k, v := range m {
		reversed[v] = k
	}
	return reversed
}

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
	Citations     []FilterCitation
	ToolCalls     *FilterToolCallDelta
	IsToolsReason bool // also used to mark thinking

	SearchQuery  *FilterSearchQueryDelta `python:",omit"` // deprecated
	IsPostAnswer bool                    // deprecated
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

type FilterMode struct{ e uint }

var (
	unknownFilterMode = FilterMode{e: 0}
	plainText         = FilterMode{1}
	ignore            = FilterMode{2}
	toolAction        = FilterMode{3}
	toolReason        = FilterMode{4}
	answer            = FilterMode{5}
	groundedAnswer    = FilterMode{6}
	inclusiveStop     = FilterMode{7}
	exclusiveStop     = FilterMode{8}
	searchQuery       = FilterMode{9}
	nextSearchQuery   = FilterMode{10}
)

var filterModeToString = map[FilterMode]string{
	plainText:       "plaintext",
	ignore:          "ignore",
	toolAction:      "toolaction",
	toolReason:      "toolreason",
	answer:          "answer",
	groundedAnswer:  "groundedanswer",
	inclusiveStop:   "inclusivestop",
	exclusiveStop:   "exclusivestop",
	searchQuery:     "searchquery",
	nextSearchQuery: "nextsearchquery",
}

var stringToFilterMode = reverse(filterModeToString)

func FilterModeFromString(s string) (FilterMode, error) {
	if fm, ok := stringToFilterMode[strings.ToLower(s)]; ok {
		return fm, nil
	}
	return unknownFilterMode, fmt.Errorf("unknown filter mode: %s", s)
}

func (f FilterMode) String() string {
	return filterModeToString[f]
}
