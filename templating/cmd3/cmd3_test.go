package cmd3

import (
	"fmt"
	"os"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/cohere-ai/melody"
)

func TestCmd3(t *testing.T) {
	tests := []struct {
		name     string
		messages []melody.Message
		options  []Option
	}{
		{
			name: "1 message",
			messages: []melody.Message{
				{Role: melody.UserRole, Content: []melody.Content{{Type: melody.TextContentType}}},
			},
			options: []Option{
				WithSkipPreamble(true),
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			filename := fmt.Sprintf("test_data/%s.txt", strings.ReplaceAll(tt.name, " ", "_"))
			expected, err := os.ReadFile(filename)
			require.NoError(t, err)
			actual, err := Render(tt.messages, tt.options...)
			require.NoError(t, err)
			require.Equal(t, string(expected), actual)
		})
	}

}
