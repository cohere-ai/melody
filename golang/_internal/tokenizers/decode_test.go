package tokenizers

import (
	"testing"

	"github.com/stretchr/testify/require"
)

func TestTokenizerDecode(t *testing.T) {
	t.Parallel()

	testCases := []struct {
		name           string
		tokenizerID    string
		tokens         []int64
		expectedString string
	}{
		{
			name:           "safely handle out of vocabulary tokens",
			tokenizerID:    "multilingual+255k+bos+eos+sptok+fim+agents3+img",
			tokens:         []int64{4184, 4647, 12523, 1801, 2268, 1719, 61556, 33, 256000},
			expectedString: "This next token is out of vocabulary:",
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()

			tokenizer, err := GetTokenizer(tc.tokenizerID)
			require.NoError(t, err)
			str, err := tokenizer.Decode(tc.tokens, false)
			require.NoError(t, err)
			require.Equal(t, tc.expectedString, str)
		})
	}
}
