package gobindings_test

import (
	_ "embed"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"

	melody "github.com/cohere-ai/melody/gobindings"
	"github.com/cohere-ai/melody/gobindings/tokenizers"
)

//go:embed data/multilingual+255k+bos+eos+sptok+fim+agents3.json
var tokenizerCommand3 []byte

func strPtr(s string) *string { return &s }

func TestFilter_Command3(t *testing.T) {
	t.Parallel()

	tkzr, err := tokenizers.FromBytes(tokenizerCommand3)
	require.NoError(t, err)

	tests := []struct {
		name          string
		input         string
		options       []melody.FilterOption
		wantContent   *string
		wantReasoning *string
		wantCitations []melody.FilterCitation
		wantToolCalls []melody.AccumulatedToolCall
	}{
		{
			name:        "basic test (no special parsing)",
			input:       "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
			wantContent: strPtr("<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>"),
		},
		{
			name: "With command 3 parsing",
			options: []melody.FilterOption{
				melody.HandleMultiHopCmd3(),
				melody.StreamToolActions(),
			},
			input:         "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
			wantContent:   strPtr("foo bar"),
			wantReasoning: strPtr("This is a rainbow emoji: 🌈"),
			wantCitations: []melody.FilterCitation{
				{
					StartIndex: 18, EndIndex: 26, Text: "emoji: 🌈",
					Sources:    []melody.Source{{ToolCallIndex: 0, ToolResultIndices: []uint{1}}},
					IsThinking: true,
				},
				{
					StartIndex: 4, EndIndex: 7, Text: "bar",
					Sources: []melody.Source{
						{ToolCallIndex: 0, ToolResultIndices: []uint{1, 2}},
						{ToolCallIndex: 1, ToolResultIndices: []uint{3, 4}},
					},
					IsThinking: false,
				},
			},
		}, {
			name: "processed params tool call",
			options: []melody.FilterOption{
				melody.HandleMultiHopCmd3(),
				melody.StreamToolActions(),
				melody.StreamProcessedParams(),
			},
			input:         "<|START_THINKING|>Some plan<|END_THINKING|>\n<|START_ACTION|>[{\"tool_call_id\": \"0\", \"tool_name\": \"add\", \"parameters\": {\"a\": 6, \"b\": 7}}]<|END_ACTION|>",
			wantReasoning: strPtr("Some plan"),
			wantToolCalls: []melody.AccumulatedToolCall{
				{
					ID: "0",
				}, {
					Name: "add",
				}, {
					ProcessedParams: []melody.FilterToolParameter{
						{Name: "a"},
					},
				}, {
					ProcessedParams: []melody.FilterToolParameter{
						{Name: "a", ValueDelta: "6"},
					},
				}, {
					ProcessedParams: []melody.FilterToolParameter{
						{Name: "b"},
					},
				}, {
					ProcessedParams: []melody.FilterToolParameter{
						{Name: "b", ValueDelta: "7"},
					},
				},
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()

			tokens, _ := tkzr.Encode(tt.input, false)

			var textChunks []string
			var buffer []uint32
			for _, token := range tokens {
				buffer = append(buffer, token)
				decoded := tkzr.Decode(buffer, false)
				if strings.HasSuffix(decoded, "\ufffd") {
					continue
				}
				textChunks = append(textChunks, decoded)
				buffer = []uint32{}
			}
			f := melody.NewFilter(tt.options...)
			require.NotNil(t, f)

			var fullContent, fullReasoning string
			var allCitations []melody.FilterCitation
			var allToolCalls []melody.AccumulatedToolCall

			for _, chunk := range textChunks {
				result, err := f.WriteDecoded(chunk)
				require.NoError(t, err)
				if result == nil {
					continue
				}
				if result.Content != nil {
					fullContent += *result.Content
				}
				if result.Reasoning != nil {
					fullReasoning += *result.Reasoning
				}
				allCitations = append(allCitations, result.Citations...)
				allToolCalls = append(allToolCalls, result.ToolCalls...)
			}

			flushResult, err := f.FlushPartials()
			require.NoError(t, err)
			if flushResult != nil {
				if flushResult.Content != nil {
					fullContent += *flushResult.Content
				}
				if flushResult.Reasoning != nil {
					fullReasoning += *flushResult.Reasoning
				}
				allCitations = append(allCitations, flushResult.Citations...)
				allToolCalls = append(allToolCalls, flushResult.ToolCalls...)
			}

			if tt.wantContent != nil {
				require.Equal(t, *tt.wantContent, fullContent, "content mismatch")
			} else {
				require.Empty(t, fullContent, "expected no content")
			}

			if tt.wantReasoning != nil {
				require.Equal(t, *tt.wantReasoning, fullReasoning, "reasoning mismatch")
			} else {
				require.Empty(t, fullReasoning, "expected no reasoning")
			}

			if tt.wantCitations != nil {
				require.Equal(t, tt.wantCitations, allCitations, "citations mismatch")
			}

			if tt.wantToolCalls != nil {
				require.Equal(t, tt.wantToolCalls, allToolCalls, "tool calls mismatch")
			}
		})
	}
}
