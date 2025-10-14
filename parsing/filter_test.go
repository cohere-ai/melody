package parsing

import (
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/cohere-ai/melody/_internal/tokenizers"
)

func TestFilter_Command3(t *testing.T) {
	tests := []struct {
		name    string
		input   string
		options []FilterOption
		want    []FilterOutput
	}{
		{
			name:  "basic test",
			input: "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
			want: []FilterOutput{
				{Text: "<BOS_TOKEN>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{5}}},
				{Text: "<|START_THINKING|>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{255019}}},
				{Text: "This", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4184}}},
				{Text: " is", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1801}}},
				{Text: " a", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1671}}},
				{Text: " rainbow", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{84470}}},
				{Text: " <", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2154}}},
				{Text: "co", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2567}}},
				{Text: ">", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}}},
				{Text: "emoji", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{104150}}},
				{Text: ":", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{33}}},
				{Text: " 🌈", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{11254, 242, 238}}},
				{Text: "</", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1965}}},
				{Text: "co", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2567}}},
				{Text: ":", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{33}}},
				{Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{228}}},
				{Text: "0", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{23}}},
				{Text: ":[", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{50706}}},
				{Text: "1", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{24}}},
				{Text: "]>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{70118}}},
				{Text: "<|END_THINKING|>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{255020}}},
				{Text: "\n", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{206}}},
				{Text: "<|START_RESPONSE|>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{255021}}},
				{Text: "foo", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{15579}}},
				{Text: " <", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2154}}},
				{Text: "co", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2567}}},
				{Text: ">", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}}},
				{Text: "bar", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4962}}},
				{Text: "</", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1965}}},
				{Text: "co", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2567}}},
				{Text: ":", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{33}}},
				{Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{228}}},
				{Text: "0", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{23}}},
				{Text: ":[", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{50706}}},
				{Text: "1", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{24}}},
				{Text: ",", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{19}}},
				{Text: "2", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{25}}},
				{Text: "],", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4085}}},
				{Text: "1", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{24}}},
				{Text: ":[", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{50706}}},
				{Text: "3", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{26}}},
				{Text: ",", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{19}}},
				{Text: "4", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{27}}},
				{Text: "]>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{70118}}},
				{Text: "<|END_RESPONSE|>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{255022}}}},
		},
		{
			name: "With command 3 parsing",
			options: []FilterOption{
				HandleMultiHopCmd3(),
				StreamToolActions(),
			},
			input: "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
			want: []FilterOutput{
				{IsToolsReason: true, Text: "This", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4184}}},
				{IsToolsReason: true, Text: " is", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1801}}},
				{IsToolsReason: true, Text: " a", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1671}}},
				{IsToolsReason: true, Text: " rainbow", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{84470}}},
				{IsToolsReason: true, Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}}},
				{IsToolsReason: true, Text: "emoji", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{104150}}},
				{IsToolsReason: true, Text: ":", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{33}}},
				{IsToolsReason: true, Text: " 🌈", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{11254, 242, 238}}},
				{IsToolsReason: true, Citations: []FilterCitation{{
					StartIndex: 18,
					EndIndex:   26,
					Text:       "emoji: 🌈",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
					IsThinking: true,
				}}},
				{Text: "foo", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{15579}}},
				{Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}}},
				{Text: "bar", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4962}}},
				{Citations: []FilterCitation{{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1, 2}}, {ToolIndex: 1, ResultIndices: []int{3, 4}}},
					IsThinking: false,
				}}},
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			tkzr, err := tokenizers.GetTokenizer("multilingual+255k+bos+eos+sptok+fim+agents3")
			f := NewFilter(nil, tkzr, tt.options...)
			require.NoError(t, err)
			tokens, err := tkzr.Encode(tt.input, tokenizers.NoSpecialTokens())
			require.NoError(t, err)
			out := []FilterOutput{}
			for _, token := range tokens {
				o, e := f.Write(token, nil)
				require.NoError(t, e)
				out = append(out, o...)
			}
			require.Equal(t, tt.want, out)
		})
	}
}
