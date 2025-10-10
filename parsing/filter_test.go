package parsing

import (
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/cohere-ai/melody/_internal/tokenizers"
)

func TestFilter_Command3(t *testing.T) {
	tests := []struct {
		name  string
		input string
		want  []FilterOutput
	}{
		{
			name:  "basic test",
			input: "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
			want:  []FilterOutput{},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			tkzr, err := tokenizers.GetTokenizer("multilingual+255k+bos+eos+sptok+fim+agents3")
			f := NewFilter(nil, tkzr)
			require.NoError(t, err)
			tokens, err := tkzr.Encode(tt.input)
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
