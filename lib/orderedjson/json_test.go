package orderedjson

import (
	"encoding/json"
	"testing"

	"github.com/stretchr/testify/require"
)

func TestObject_MarshalJSON(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name     string
		input    Object
		expected string
	}{
		{
			name:     "basic",
			input:    New(WithInitialData(Pair{"b", "1"}, Pair{"a", "2"})),
			expected: `{"b":"1","a":"2"}`,
		},
		{
			name:     "nested object",
			input:    New(WithInitialData(Pair{"b", "1"}, Pair{"obj", New(WithInitialData(Pair{"b", "1"}, Pair{"a", "2"}))})),
			expected: `{"b":"1","obj":{"b":"1","a":"2"}}`,
		},
		{
			name:     "numeric types",
			input:    New(WithInitialData(Pair{"b", 1.0}, Pair{"a", 2})),
			expected: `{"b":1.0,"a":2}`,
		},
		{
			name:     "nil type",
			input:    New(WithInitialData(Pair{"b", nil}, Pair{"a", 2})),
			expected: `{"b":null,"a":2}`,
		},
		{
			name:     "handles scientific notation",
			input:    New(WithInitialData(Pair{"loan_amount", 1000000.0}, Pair{"interest_rate", 0.03}, Pair{"loan_period", 30})),
			expected: `{"loan_amount":1e+06,"interest_rate":0.03,"loan_period":30}`,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			got, err := json.Marshal(tc.input)
			require.NoError(t, err)
			require.Equal(t, tc.expected, string(got))
		})
	}
}

func TestObject_UnmarshalJSON(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name        string
		input       string
		expected    Object
		expectedErr error
	}{
		{
			name:     "basic object",
			input:    `{"b" : "v1", "a": 2}`,
			expected: New(WithInitialData(Pair{"b", "v1"}, Pair{"a", int64(2)})),
		},
		{
			name:     "basic object, float preserved",
			input:    `{"b" : "v1", "a": 2.0}`,
			expected: New(WithInitialData(Pair{"b", "v1"}, Pair{"a", float64(2)})),
		},
		{
			name:     "nested object ordered",
			input:    `{"obj" : {"b": "1", "a": "2"}}`,
			expected: New(WithInitialData(Pair{"obj", New(WithInitialData(Pair{Key: "b", Value: "1"}, Pair{Key: "a", Value: "2"}))})),
		}, {
			name:     "handles scientific notation",
			input:    `{"loan_amount":1e+06,"interest_rate":0.03,"loan_period":30}`,
			expected: New(WithInitialData(Pair{"loan_amount", float64(1000000.0)}, Pair{"interest_rate", float64(0.03)}, Pair{"loan_period", int64(30)})),
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()

			var got Object
			err := json.Unmarshal([]byte(tc.input), &got)
			if tc.expectedErr != nil {
				require.Equal(t, tc.expectedErr, err)
				return
			}
			require.NoError(t, err)
			require.Equal(t, tc.expected, got)
		})
	}
}

func TestUnMarshalMarshalProducesSameJSON(t *testing.T) {
	jsonStr := `{"obj":{"b":"1","a":"2"}, "b":"1", "c":["1", 2], "a":"2"}`
	var obj Object
	err := json.Unmarshal([]byte(jsonStr), &obj)
	require.NoError(t, err)
	orderedJSON, err := json.Marshal(obj)
	require.NoError(t, err)
	require.JSONEq(t, jsonStr, string(orderedJSON))
}
