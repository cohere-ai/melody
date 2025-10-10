package tokenizers

import (
	"testing"

	"github.com/stretchr/testify/require"
)

func TestGetTokenizer(t *testing.T) {
	t.Parallel()

	testCases := []struct {
		name            string
		tokenizerID     string
		wantTokenizerID string
	}{
		{
			name:            "No tokenizer ID specified",
			tokenizerID:     "",
			wantTokenizerID: "50k",
		},
		{
			name:            "Cohere tokenizer",
			tokenizerID:     "cohere",
			wantTokenizerID: "50k",
		},
		{
			name:            "Chat tokenizer",
			tokenizerID:     "75k+bos+eos+eop",
			wantTokenizerID: "75k+bos+eos+eop",
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(tt *testing.T) {
			tt.Parallel()

			tokenizer, err := GetTokenizer(tc.tokenizerID)
			require.NoError(tt, err)
			require.Equal(tt, tc.wantTokenizerID, tokenizer.ID())
		})
	}
}
