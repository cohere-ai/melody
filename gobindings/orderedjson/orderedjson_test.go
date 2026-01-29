package orderedjson

import (
	"encoding/json"
	"testing"

	"github.com/stretchr/testify/require"
)

func TestObjectLen(t *testing.T) {
	ob := New()
	require.Equal(t, 0, ob.Len())
	ob.Set("b", "1")
	require.Equal(t, 1, ob.Len())
	ob.Delete("b")
	require.Equal(t, 0, ob.Len())
}

func TestObjectInit(t *testing.T) {
	ob := New(WithInitialData(Pair{"b", "1"}, Pair{"a", "2"}))
	require.Equal(t, 2, ob.Len())
	require.Equal(t, []string{"b", "a"}, ob.Keys())

	ob2 := New(WithInitialData(Pair{"b", "1"}, Pair{"b", "2"}))
	require.Equal(t, 1, ob2.Len())
	require.Equal(t, []string{"b"}, ob2.Keys())
}

// func TestObjectIter(t *testing.T) {
//	ob := New(WithInitialData(Pair{"b", "1"}, Pair{"a", "2"}))
//	i := 0
//	for k, v := range ob.Pairs() {
//		if i == 0 {
//			require.Equal(t, "b", k)
//			require.Equal(t, "1", v)
//		} else {
//			require.Equal(t, "a", k)
//			require.Equal(t, "2", v)
//		}
//		i++
//	}
// }

func TestObject_ToMap(t *testing.T) {
	ob := New(WithInitialData(Pair{"b", "1"}, Pair{"a", "2"}))
	m := ob.ToMap()
	require.Equal(t, map[string]any{"b": "1", "a": "2"}, m)
	// add nested map
	ob2 := New(WithInitialData(Pair{"c", []string{"3"}}, Pair{"d", 4}, Pair{"e", []any{"5", 6}}))
	ob.Set("f", ob2)
	m = ob.ToMap()
	require.Equal(t, map[string]any{"b": "1", "a": "2", "f": map[string]any{"c": []string{"3"}, "d": 4, "e": []any{"5", 6}}}, m)
}

func TestObjectSetGetDelete(t *testing.T) {
	ob := New(WithInitialData(Pair{"b", "1"}, Pair{"a", "2"}))
	v, ok := ob.Get("b")
	require.True(t, ok)
	require.Equal(t, "1", v)
	_, ok = ob.Get("c")
	require.False(t, ok)

	// override b
	ob.Set("b", "3")
	v, ok = ob.Get("b")
	require.True(t, ok)
	require.Equal(t, "3", v)
	// ensure order does not change
	require.Equal(t, []string{"b", "a"}, ob.Keys())

	// add new value
	ob.Set("c", "4")
	v, ok = ob.Get("c")
	require.True(t, ok)
	require.Equal(t, "4", v)
	require.Equal(t, []string{"b", "a", "c"}, ob.Keys())

	// remove key
	ob.Delete("a")
	require.Equal(t, []string{"b", "c"}, ob.Keys())
}

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
		{
			name:     "doesn't escape html characters",
			input:    New(WithInitialData(Pair{"b", "<>&'\""}, Pair{"a", 2})),
			expected: `{"b":"<>&'\"","a":2}`,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			got, err := tc.input.MarshalJSON()
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
		}, {
			name:     "ensure escaped characters are handled correctly",
			input:    `{"key": "hel\\\"lo"}`,
			expected: New(WithInitialData(Pair{"key", `hel\"lo`})),
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
