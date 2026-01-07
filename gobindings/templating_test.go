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
		}, {
			name: "with basic substitutions",
			input: RenderCmd3Options{
				Messages: []Message{
					{
						Role: RoleChatbot,
						Content: []Content{
							{
								Type: ContentText,
								Text: "Hello user!",
							},
						},
					},
					{
						Role: RoleUser,
						Content: []Content{
							{
								Type: ContentText,
								Text: "Some content",
							}, {
								Type: ContentText,
								Text: ". More content",
							},
						},
					},
				},
				Template: "{% for message in messages %}{{message.role}}: {% for content_item in message.content %}{{content_item.data}}{% endfor %}\n{% endfor %}",
			},
			want: "CHATBOT: Hello user!\nUSER: Some content. More content\n",
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
