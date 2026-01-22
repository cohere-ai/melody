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

func TestFilter_Command3(t *testing.T) {
	t.Parallel()

	tkzr, err := tokenizers.FromBytes(tokenizerCommand3)
	require.NoError(t, err)

	// for simplicity's sake lets just generate likelihoods in intervals of thousandths
	testLikelihoods := make([]float32, 999)
	for i := range testLikelihoods {
		testLikelihoods[i] = float32(i) / 1000
	}
	tests := []struct {
		name        string
		input       string
		likelihoods []float32
		options     []melody.FilterOption
		want        []melody.FilterOutput
	}{
		{
			name:        "basic test",
			input:       "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
			likelihoods: testLikelihoods,
			want: []melody.FilterOutput{
				{Text: "<|START_THINKING|>", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{255019}, Logprobs: []float32{0}}},
				{Text: "This", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{4184}, Logprobs: []float32{0.001}}},
				{Text: " is", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{1801}, Logprobs: []float32{0.002}}},
				{Text: " a", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{1671}, Logprobs: []float32{0.003}}},
				{Text: " rainbow", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{84470}, Logprobs: []float32{0.004}}},
				{Text: " <", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{2154}, Logprobs: []float32{0.005}}},
				{Text: "co", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{2567}, Logprobs: []float32{0.006}}},
				{Text: ">", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{37}, Logprobs: []float32{0.007}}},
				{Text: "emoji", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{104150}, Logprobs: []float32{0.008}}},
				{Text: ":", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{33}, Logprobs: []float32{0.009}}},
				{Text: " 🌈", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{11254, 242, 238}, Logprobs: []float32{0.01, 0.011, 0.012}}},
				{Text: "</", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{1965}, Logprobs: []float32{0.013}}},
				{Text: "co", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{2567}, Logprobs: []float32{0.014}}},
				{Text: ":", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{33}, Logprobs: []float32{0.015}}},
				{Text: " ", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{228}, Logprobs: []float32{0.016}}},
				{Text: "0", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{23}, Logprobs: []float32{0.017}}},
				{Text: ":[", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{50706}, Logprobs: []float32{0.018}}},
				{Text: "1", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{24}, Logprobs: []float32{0.019}}},
				{Text: "]>", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{70118}, Logprobs: []float32{0.02}}},
				{Text: "<|END_THINKING|>", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{255020}, Logprobs: []float32{0.021}}},
				{Text: "\n", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{206}, Logprobs: []float32{0.022}}},
				{Text: "<|START_RESPONSE|>", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{255021}, Logprobs: []float32{0.023}}},
				{Text: "foo", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{15579}, Logprobs: []float32{0.024}}},
				{Text: " <", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{2154}, Logprobs: []float32{0.025}}},
				{Text: "co", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{2567}, Logprobs: []float32{0.026}}},
				{Text: ">", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{37}, Logprobs: []float32{0.027}}},
				{Text: "bar", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{4962}, Logprobs: []float32{0.028}}},
				{Text: "</", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{1965}, Logprobs: []float32{0.029}}},
				{Text: "co", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{2567}, Logprobs: []float32{0.03}}},
				{Text: ":", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{33}, Logprobs: []float32{0.031}}},
				{Text: " ", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{228}, Logprobs: []float32{0.032}}},
				{Text: "0", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{23}, Logprobs: []float32{0.033}}},
				{Text: ":[", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{50706}, Logprobs: []float32{0.034}}},
				{Text: "1", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{24}, Logprobs: []float32{0.035}}},
				{Text: ",", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{19}, Logprobs: []float32{0.036}}},
				{Text: "2", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{25}, Logprobs: []float32{0.037}}},
				{Text: "],", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{4085}, Logprobs: []float32{0.038}}},
				{Text: "1", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{24}, Logprobs: []float32{0.039}}},
				{Text: ":[", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{50706}, Logprobs: []float32{0.04}}},
				{Text: "3", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{26}, Logprobs: []float32{0.041}}},
				{Text: ",", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{19}, Logprobs: []float32{0.042}}},
				{Text: "4", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{27}, Logprobs: []float32{0.043}}},
				{Text: "]>", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{70118}, Logprobs: []float32{0.044}}},
				{Text: "<|END_RESPONSE|>", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{255022}, Logprobs: []float32{0.045}}},
			},
		},
		{
			name: "With command 3 parsing",
			options: []melody.FilterOption{
				melody.HandleMultiHopCmd3(),
				melody.StreamToolActions(),
			},
			likelihoods: testLikelihoods,
			input:       "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
			want: []melody.FilterOutput{
				{IsReasoning: true, Text: "This", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{4184}, Logprobs: []float32{0.001}}},
				{IsReasoning: true, Text: " is", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{1801}, Logprobs: []float32{0.002}}},
				{IsReasoning: true, Text: " a", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{1671}, Logprobs: []float32{0.003}}},
				{IsReasoning: true, Text: " rainbow", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{84470}, Logprobs: []float32{0.004}}},
				{IsReasoning: true, Text: " ", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{37}, Logprobs: []float32{0.007}}},
				{IsReasoning: true, Text: "emoji", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{104150}, Logprobs: []float32{0.008}}},
				{IsReasoning: true, Text: ":", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{33}, Logprobs: []float32{0.009}}},
				{IsReasoning: true, Text: " 🌈", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{11254, 242, 238}, Logprobs: []float32{0.01, 0.011, 0.012}}},
				{IsReasoning: true, Citations: []melody.FilterCitation{{
					StartIndex: 18,
					EndIndex:   26,
					Text:       "emoji: 🌈",
					Sources:    []melody.Source{{ToolCallIndex: 0, ToolResultIndices: []int{1}}},
					IsThinking: true,
				}},
					Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{70118}, Logprobs: []float32{0.02}},
				},
				{Text: "foo", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{15579}, Logprobs: []float32{0.024}}},
				{Text: " ", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{37}, Logprobs: []float32{0.027}}},
				{Text: "bar", Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{4962}, Logprobs: []float32{0.028}}},
				{
					Citations: []melody.FilterCitation{{
						StartIndex: 4,
						EndIndex:   7,
						Text:       "bar",
						Sources:    []melody.Source{{ToolCallIndex: 0, ToolResultIndices: []int{1, 2}}, {ToolCallIndex: 1, ToolResultIndices: []int{3, 4}}},
						IsThinking: false,
					}},
					Logprobs: melody.TokenIDsWithLogProb{TokenIDs: []uint32{70118}, Logprobs: []float32{0.044}},
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
			var likelihoodsChunks []melody.TokenIDsWithLogProb
			var likelihoodBuffer []float32
			for i, token := range tokens {
				buffer = append(buffer, token)
				decoded := tkzr.Decode(buffer, false)
				likelihoodBuffer = append(likelihoodBuffer, tt.likelihoods[i])
				if strings.HasSuffix(decoded, "\ufffd") {
					continue
				}
				textChunks = append(textChunks, decoded)
				likelihoodsChunks = append(likelihoodsChunks, melody.TokenIDsWithLogProb{
					TokenIDs: buffer,
					Logprobs: likelihoodBuffer,
				})
				buffer = []uint32{}
				likelihoodBuffer = []float32{}
			}
			f := melody.NewFilter(tt.options...)
			require.NotNil(t, f)
			out := []melody.FilterOutput{}
			for i, chunk := range textChunks {
				out = append(out, f.WriteDecoded(chunk, &likelihoodsChunks[i])...)
			}
			require.Equal(t, tt.want, out)
		})
	}
}
