package gobindings

import (
	"testing"

	"github.com/stretchr/testify/require"
)

func TestTemplating_RenderCMD3(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name  string
		input RenderCmd3Options
		want  string
	}{
		{
			name: "basic test",
			input: RenderCmd3Options{
				Template: "Hello!",
			},
			want: "Hello!",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			actual, err := RenderCMD3(tt.input)
			require.NoError(t, err)
			require.Equal(t, tt.want, actual)
		})
	}
}
