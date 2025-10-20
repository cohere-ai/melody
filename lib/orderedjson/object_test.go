package orderedjson

import (
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
