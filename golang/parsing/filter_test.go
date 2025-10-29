package parsing

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/cohere-ai/melody/_internal/tokenizers"
)

func TestFilter_Command3(t *testing.T) {
	t.Parallel()

	// for simplicity's sake lets just generate likelihoods in intervals of thousandths
	testLikelihoods := make([]float32, 999)
	for i := range testLikelihoods {
		testLikelihoods[i] = float32(i) / 1000
	}
	tests := []struct {
		name        string
		input       string
		likelihoods []float32
		options     []FilterOption
		want        []FilterOutput
	}{
		{
			name:        "basic test",
			input:       "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
			likelihoods: testLikelihoods,
			want: []FilterOutput{
				{Text: "<|START_THINKING|>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{255019}, Logprobs: []float32{0}}},
				{Text: "This", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4184}, Logprobs: []float32{0.001}}},
				{Text: " is", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1801}, Logprobs: []float32{0.002}}},
				{Text: " a", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1671}, Logprobs: []float32{0.003}}},
				{Text: " rainbow", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{84470}, Logprobs: []float32{0.004}}},
				{Text: " <", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2154}, Logprobs: []float32{0.005}}},
				{Text: "co", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2567}, Logprobs: []float32{0.006}}},
				{Text: ">", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}, Logprobs: []float32{0.007}}},
				{Text: "emoji", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{104150}, Logprobs: []float32{0.008}}},
				{Text: ":", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{33}, Logprobs: []float32{0.009}}},
				{Text: " 🌈", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{11254, 242, 238}, Logprobs: []float32{0.01, 0.011, 0.012}}},
				{Text: "</", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1965}, Logprobs: []float32{0.013}}},
				{Text: "co", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2567}, Logprobs: []float32{0.014}}},
				{Text: ":", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{33}, Logprobs: []float32{0.015}}},
				{Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{228}, Logprobs: []float32{0.016}}},
				{Text: "0", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{23}, Logprobs: []float32{0.017}}},
				{Text: ":[", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{50706}, Logprobs: []float32{0.018}}},
				{Text: "1", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{24}, Logprobs: []float32{0.019}}},
				{Text: "]>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{70118}, Logprobs: []float32{0.02}}},
				{Text: "<|END_THINKING|>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{255020}, Logprobs: []float32{0.021}}},
				{Text: "\n", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{206}, Logprobs: []float32{0.022}}},
				{Text: "<|START_RESPONSE|>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{255021}, Logprobs: []float32{0.023}}},
				{Text: "foo", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{15579}, Logprobs: []float32{0.024}}},
				{Text: " <", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2154}, Logprobs: []float32{0.025}}},
				{Text: "co", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2567}, Logprobs: []float32{0.026}}},
				{Text: ">", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}, Logprobs: []float32{0.027}}},
				{Text: "bar", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4962}, Logprobs: []float32{0.028}}},
				{Text: "</", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1965}, Logprobs: []float32{0.029}}},
				{Text: "co", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{2567}, Logprobs: []float32{0.03}}},
				{Text: ":", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{33}, Logprobs: []float32{0.031}}},
				{Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{228}, Logprobs: []float32{0.032}}},
				{Text: "0", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{23}, Logprobs: []float32{0.033}}},
				{Text: ":[", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{50706}, Logprobs: []float32{0.034}}},
				{Text: "1", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{24}, Logprobs: []float32{0.035}}},
				{Text: ",", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{19}, Logprobs: []float32{0.036}}},
				{Text: "2", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{25}, Logprobs: []float32{0.037}}},
				{Text: "],", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4085}, Logprobs: []float32{0.038}}},
				{Text: "1", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{24}, Logprobs: []float32{0.039}}},
				{Text: ":[", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{50706}, Logprobs: []float32{0.04}}},
				{Text: "3", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{26}, Logprobs: []float32{0.041}}},
				{Text: ",", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{19}, Logprobs: []float32{0.042}}},
				{Text: "4", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{27}, Logprobs: []float32{0.043}}},
				{Text: "]>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{70118}, Logprobs: []float32{0.044}}},
				{Text: "<|END_RESPONSE|>", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{255022}, Logprobs: []float32{0.045}}},
			},
		},
		{
			name: "With command 3 parsing",
			options: []FilterOption{
				HandleMultiHopCmd3(),
				StreamToolActions(),
			},
			likelihoods: testLikelihoods,
			input:       "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
			want: []FilterOutput{
				{IsToolsReason: true, Text: "This", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4184}, Logprobs: []float32{0.001}}},
				{IsToolsReason: true, Text: " is", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1801}, Logprobs: []float32{0.002}}},
				{IsToolsReason: true, Text: " a", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1671}, Logprobs: []float32{0.003}}},
				{IsToolsReason: true, Text: " rainbow", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{84470}, Logprobs: []float32{0.004}}},
				{IsToolsReason: true, Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}, Logprobs: []float32{0.007}}},
				{IsToolsReason: true, Text: "emoji", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{104150}, Logprobs: []float32{0.008}}},
				{IsToolsReason: true, Text: ":", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{33}, Logprobs: []float32{0.009}}},
				{IsToolsReason: true, Text: " 🌈", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{11254, 242, 238}, Logprobs: []float32{0.01, 0.011, 0.012}}},
				{IsToolsReason: true, Citations: []FilterCitation{{
					StartIndex: 18,
					EndIndex:   26,
					Text:       "emoji: 🌈",
					Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1}}},
					IsThinking: true,
				}}},
				{Text: "foo", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{15579}, Logprobs: []float32{0.024}}},
				{Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}, Logprobs: []float32{0.027}}},
				{Text: "bar", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4962}, Logprobs: []float32{0.028}}},
				{Citations: []FilterCitation{{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1, 2}}, {ToolCallIndex: 1, ToolResultIndices: []int{3, 4}}},
					IsThinking: false,
				}}},
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()

			tkzr, err := tokenizers.GetTokenizer("multilingual+255k+bos+eos+sptok+fim+agents3")
			require.NoError(t, err)
			f := NewFilter(nil, tkzr, tt.options...)
			tokens, err := tkzr.Encode(tt.input, tokenizers.NoSpecialTokens())
			require.NoError(t, err)
			out := []FilterOutput{}
			for i, token := range tokens {
				o, e := f.Write(token, &tt.likelihoods[i])
				require.NoError(t, e)
				out = append(out, o...)
			}
			require.Equal(t, tt.want, out)

			// Duplicate the test by writing the raw strings instead
			var textChunks []string
			var buffer []int64
			var likelihoodsChunks []TokenIDsWithLogProb
			var likelihoodBuffer []float32
			for i, token := range tokens {
				buffer = append(buffer, token)
				decoded, err := tkzr.Decode(buffer, false)
				require.NoError(t, err)
				likelihoodBuffer = append(likelihoodBuffer, tt.likelihoods[i])
				if strings.HasSuffix(decoded, "\ufffd") {
					continue
				}
				textChunks = append(textChunks, decoded)
				likelihoodsChunks = append(likelihoodsChunks, TokenIDsWithLogProb{
					TokenIDs: buffer,
					Logprobs: likelihoodBuffer,
				})
				buffer = []int64{}
				likelihoodBuffer = []float32{}
			}
			f = NewFilter(nil, nil, tt.options...)
			out = []FilterOutput{}
			for i, chunk := range textChunks {
				out = append(out, f.WriteDecoded(chunk, likelihoodsChunks[i])...)
			}
			require.Equal(t, tt.want, out)
		})
	}
}
