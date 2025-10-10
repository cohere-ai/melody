package parsing

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/require"
)

func Test_HandleParamValue(t *testing.T) {
	t.Parallel()
	testcases := []struct {
		name               string
		input              string
		metadata           FilterAction
		expectedParamValue string
		expectedRemove     int
		expectedEndState   ParamState
	}{
		{
			name:               "empty",
			input:              "",
			expectedParamValue: "",
			expectedRemove:     0,
		},
		{
			name:               "basic with next parameter",
			input:              "30   ,",
			expectedParamValue: "30",
			expectedRemove:     6,
		},
		{
			name:               "basic with end of the tool",
			input:              "1.2   \n}",
			expectedParamValue: "1.2",
			expectedRemove:     8,
		},
		{
			name:               "null with end of the tool",
			input:              "null   \n}",
			expectedParamValue: "null",
			expectedRemove:     9,
		},
		{
			name:               "boolean with end of the tool",
			input:              "true   \n}",
			expectedParamValue: "true",
			expectedRemove:     9,
		},
		{
			name:               "partial string",
			input:              "\"testing",
			expectedParamValue: "\"testing",
			expectedRemove:     8,
		},
		{
			name:               "whole string",
			input:              "\"testing string\"   \n}",
			expectedParamValue: "\"testing string\"",
			expectedRemove:     17,
		},
		{
			name:               "whole object",
			input:              "{\"tes t\": [\"}\"]}   \n,",
			expectedParamValue: "{\"tes t\": [\"}\"]}",
			expectedRemove:     17,
		},
		{
			name:               "partial object",
			input:              "{\"tes t\": [\"}    ,",
			expectedParamValue: "{\"tes t\": [\"}    ,",
			expectedRemove:     18,
		},
		{
			name:               "whole array",
			input:              "[{\"test\",[\"}\",\"]\"]}]   }",
			expectedParamValue: "[{\"test\",[\"}\",\"]\"]}]   }",
			expectedRemove:     24,
		},
		{
			name:               "partial array",
			input:              "[{\"test\",[\"}\",\"]    ,",
			expectedParamValue: "[{\"test\",[\"}\",\"]    ,",
			expectedRemove:     21,
		},
	}
	for _, tt := range testcases {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()

			f := filter{
				actionMetaData:    tt.metadata,
				streamToolActions: true,
			}
			out, actualRemove := f.HandleParamValue(tt.input)
			require.Equal(t, tt.expectedRemove, actualRemove)

			actualRes := strings.Builder{}
			for _, s := range out {
				require.NotNil(t, s.ToolCalls.ParamDelta)
				actualRes.WriteString(s.ToolCalls.ParamDelta.ValueDelta)
			}
			require.Equal(t, tt.expectedParamValue, actualRes.String())
		})
	}
}
