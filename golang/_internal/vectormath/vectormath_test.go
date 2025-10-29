package vectormath

import (
	"math"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	"github.com/x448/float16"
)

func TestSum(t *testing.T) {
	cases := []struct {
		input    []int
		expected int
	}{
		{
			input:    []int{},
			expected: 0,
		},
		{
			input:    []int{1, 2, 3, 4},
			expected: 10,
		},
	}

	for _, tt := range cases {
		require.Equal(t, tt.expected, Sum(tt.input))
	}
}

func TestSumLengths(t *testing.T) {
	cases := []struct {
		input    [][]int
		expected int
	}{
		{
			input: [][]int{
				{1, 2},
				{3, 4},
			},
			expected: 4,
		},
		{
			input: [][]int{
				{1},
				{2, 3},
				{4, 5, 6},
			},
			expected: 6,
		},
	}

	for _, tt := range cases {
		require.Equal(t, tt.expected, SumLengths(tt.input))
	}
}

func TestSumColumnwise(t *testing.T) {
	cases := []struct {
		input    [][]int
		expected []int
	}{
		{
			input: [][]int{
				{1, 2, 3},
				{4, 5, 6},
			},
			expected: []int{5, 7, 9},
		},
	}

	for _, tt := range cases {
		require.Equal(t, tt.expected, SumColumnwise(tt.input))
	}
}

func TestMulByConstant(t *testing.T) {
	cases := []struct {
		input    []int
		mul      int
		expected []int
	}{
		{
			input:    []int{1, 2, 3},
			mul:      1,
			expected: []int{1, 2, 3},
		},
		{
			input:    []int{1, 2, 3},
			mul:      2,
			expected: []int{2, 4, 6},
		},
	}

	for _, tt := range cases {
		require.Equal(t, tt.expected, MulByConstant(tt.input, tt.mul))
	}
}

func TestExp(t *testing.T) {
	cases := []struct {
		input    []float32
		expected []float64
	}{
		{
			input:    []float32{0.1, 0.5, 0.7},
			expected: []float64{1.10517092, 1.64872127, 2.01375271},
		},
	}

	for _, tt := range cases {
		result := Exp(tt.input)
		require.Len(t, tt.expected, len(result))
		for i := range result {
			require.LessOrEqual(t, math.Abs(result[i]-tt.expected[i]), 1e-7)
		}
	}
}

func TestFlatten(t *testing.T) {
	cases := []struct {
		input    [][]int
		expected []int
	}{
		{
			input: [][]int{
				{1, 2, 3},
			},
			expected: []int{1, 2, 3},
		},
		{
			input: [][]int{
				{1, 2, 3},
				{4, 5, 6},
				{7, 8, 9},
			},
			expected: []int{1, 2, 3, 4, 5, 6, 7, 8, 9},
		},
	}

	for _, tt := range cases {
		require.Equal(t, tt.expected, Flatten(tt.input))
	}
}

func TestFlattenConvert(t *testing.T) {
	cases := []struct {
		input    [][]float32
		expected []float64
	}{
		{
			input: [][]float32{
				{1, 2, 3},
			},
			expected: []float64{1, 2, 3},
		},
		{
			input: [][]float32{
				{1, 2, 3},
				{4, 5, 6},
				{7, 8, 9},
			},
			expected: []float64{1, 2, 3, 4, 5, 6, 7, 8, 9},
		},
	}

	for _, tt := range cases {
		require.Equal(t, tt.expected, FlattenConvert[float32, float64](tt.input))
	}
}

func TestUnflatten(t *testing.T) {
	cases := []struct {
		input    [][]int
		expected []int
	}{
		{
			input: [][]int{
				{1, 2, 3},
				{4, 5, 6},
				{7, 8, 9},
			},
			expected: []int{1, 2, 3, 4, 5, 6, 7, 8, 9},
		},
	}

	for _, tt := range cases {
		require.Equal(t, tt.expected, Flatten(tt.input))
	}
}

func TestReshape(t *testing.T) {
	cases := []struct {
		size     int
		input    [][]int
		expected [][]int
	}{
		{
			size: 4,
			input: [][]int{
				{1, 2},
				{3, 4},
			},
			expected: [][]int{
				{1}, {2}, {3}, {4},
			},
		},
		{
			size: 2,
			input: [][]int{
				{1, 2, 3, 4},
				{5, 6},
				{7, 8, 9, 10},
			},
			expected: [][]int{
				{1, 2, 3, 4, 5},
				{6, 7, 8, 9, 10},
			},
		},
		{
			size: 5,
			input: [][]int{
				{1, 2, 3, 4, 5},
				{6, 7, 8, 9, 10},
			},
			expected: [][]int{
				{1, 2},
				{3, 4},
				{5, 6},
				{7, 8},
				{9, 10},
			},
		},
	}

	for _, tt := range cases {
		require.Equal(t, tt.expected, Reshape(tt.input, tt.size))
	}
}

func TestMarshalUnmarshal(t *testing.T) {
	intArray := []int32{1, 2, 3, 4}
	require.Equal(t, DeserializeFromBytes[int32](SerializeToBytes(intArray)), intArray)

	floatArray := []float32{1.1, 2.2, 3.3, 4.4}
	require.Equal(t, DeserializeFromBytes[float32](SerializeToBytes(floatArray)), floatArray)

	float16Array := []float16.Float16{
		float16.Fromfloat32(1.1),
		float16.Fromfloat32(2.2),
		float16.Fromfloat32(3.3),
		float16.Fromfloat32(4.4),
	}
	require.Equal(t, DeserializeFromBytes[float16.Float16](SerializeToBytes(float16Array)), float16Array)
}

func TestConvertF16ToF32(t *testing.T) {
	floatArray := []float32{1.1, 2.2, 3.3, 4.4}
	float16Array := ConvertF32ToF16(floatArray)
	convertedFloatArray := ConvertF16ToF32(float16Array)
	for i := 0; i < len(floatArray); i++ {
		require.InDelta(t, floatArray[i], convertedFloatArray[i], 0.1)
	}
}

func TestPad2D(t *testing.T) {
	cases := []struct {
		input    [][]int
		expected [][]int
	}{
		{
			input: [][]int{
				{1, 2},
			},
			expected: [][]int{
				{1, 2},
			},
		},
		{
			input: [][]int{
				{1, 2},
				{1, 2, 3, 4},
			},
			expected: [][]int{
				{1, 2, 0, 0},
				{1, 2, 3, 4},
			},
		},
	}

	for _, tt := range cases {
		arr := tt.input
		Pad2D(arr)
		require.Equal(t, tt.expected, arr)
	}
}

func TestPadToLength2D(t *testing.T) {
	cases := []struct {
		length   int
		input    [][]int
		expected [][]int
	}{
		{
			length: 4,
			input: [][]int{
				{1, 2},
			},
			expected: [][]int{
				{1, 2, 0, 0},
			},
		},
		{
			length: 3,
			input: [][]int{
				{1, 2},
				{1, 2, 3, 4},
			},
			expected: [][]int{
				{1, 2, 0},
				{1, 2, 3},
			},
		},
	}

	for _, tt := range cases {
		arr := tt.input
		PadToLength2D(arr, tt.length)
		require.Equal(t, tt.expected, arr)
	}
}

func TestPadToLength2DNonZeroPadToken(t *testing.T) {
	cases := []struct {
		length   int
		input    [][]int
		padToken int
		expected [][]int
	}{
		{
			length: 4,
			input: [][]int{
				{1, 2},
			},
			padToken: 5,
			expected: [][]int{
				{1, 2, 5, 5},
			},
		},
		{
			length: 3,
			input: [][]int{
				{1, 2},
				{1, 2, 3, 4},
			},
			padToken: 5,
			expected: [][]int{
				{1, 2, 5},
				{1, 2, 3},
			},
		},
		{
			length: 2,
			input: [][]int{
				{1, 2},
				{1, 2, 3, 4},
			},
			padToken: 5,
			expected: [][]int{
				{1, 2},
				{1, 2},
			},
		},
	}

	for _, tt := range cases {
		arr := tt.input
		PadToLength2DNonZeroPadToken(arr, tt.length, tt.padToken)
		require.Equal(t, tt.expected, arr)
	}
}

func TestPadToLength2DNonZeroPadTokenLeft(t *testing.T) {
	cases := []struct {
		length   int
		input    [][]int
		padToken int
		expected [][]int
	}{
		{
			length: 4,
			input: [][]int{
				{1, 2},
			},
			padToken: 5,
			expected: [][]int{
				{5, 5, 1, 2},
			},
		},
		{
			length: 3,
			input: [][]int{
				{1, 2},
				{1, 2, 3, 4},
			},
			padToken: 5,
			expected: [][]int{
				{5, 1, 2},
				{1, 2, 3},
			},
		},
		{
			length: 2,
			input: [][]int{
				{1, 2},
				{1, 2, 3, 4},
			},
			padToken: 5,
			expected: [][]int{
				{1, 2},
				{1, 2},
			},
		},
	}

	for _, tt := range cases {
		arr := tt.input
		PadToLength2DNonZeroPadTokenLeft(arr, tt.length, tt.padToken)
		require.Equal(t, tt.expected, arr)
	}
}

func TestStacking(t *testing.T) {
	t.Parallel()

	t.Run("stack float32", func(tt *testing.T) {
		stacked := GetStacked[float32](0.24, 3)
		require.Equal(tt, []float32{0.24, 0.24, 0.24}, stacked)
	})

	t.Run("stack uint32", func(tt *testing.T) {
		stacked := GetStacked[uint32](41, 3)
		require.Equal(tt, []uint32{41, 41, 41}, stacked)
	})

	t.Run("stack int32", func(tt *testing.T) {
		stacked := GetStacked[int32](41, 3)
		require.Equal(tt, []int32{41, 41, 41}, stacked)
	})
}

func TestCreateChunks(t *testing.T) {
	t.Parallel()

	testCases := []struct {
		name          string
		tokens        []int64
		chunks        [][]int64
		contextLength int
	}{
		{
			name:          "chunks at context length into one chunk",
			tokens:        []int64{1, 2, 3},
			chunks:        [][]int64{{1, 2, 3}},
			contextLength: 3,
		},
		{
			name:          "chunks longer than context length into two chunks with padding",
			tokens:        []int64{1, 2, 3, 4},
			chunks:        [][]int64{{1, 2, 3}, {4, 0, 0}},
			contextLength: 3,
		},
		{
			name:          "chunks shorter than context length into one chunks with padding",
			tokens:        []int64{1, 2},
			chunks:        [][]int64{{1, 2, 0}},
			contextLength: 3,
		},
	}
	for _, tc := range testCases {
		t.Run(tc.name, func(tt *testing.T) {
			require.Equal(tt, tc.chunks, CreateChunks(tc.tokens, tc.contextLength))
		})
	}
}

func TestGetSizes(t *testing.T) {
	cases := []struct {
		input    [][]int
		expected []int
	}{
		{
			input:    [][]int{},
			expected: []int{},
		},
		{
			input: [][]int{
				{1},
				{2, 3},
				{4, 5, 6},
			},
			expected: []int{1, 2, 3},
		},
	}

	for _, tt := range cases {
		require.Equal(t, tt.expected, GetSizes[int](tt.input))
	}
}

func TestSumColumn(t *testing.T) {
	cases := []struct {
		input    [][]int
		colIdx   int
		expected int
	}{
		{
			input:    [][]int{},
			colIdx:   0,
			expected: 0,
		},
		{
			input: [][]int{
				{1, 0, 0},
				{2, 3, 0},
				{4, 5, 6},
			},
			colIdx:   0,
			expected: 7,
		},
	}

	for _, tt := range cases {
		require.Equal(t, tt.expected, SumColumn(tt.input, tt.colIdx))
	}
}

func TestSumRow(t *testing.T) {
	cases := []struct {
		input    [][]int
		rowIdx   int
		expected int
	}{
		{
			input:    [][]int{},
			rowIdx:   0,
			expected: 0,
		},
		{
			input: [][]int{
				{1, 0, 0},
				{2, 3, 0},
				{4, 5, 6},
			},
			rowIdx:   1,
			expected: 5,
		},
	}

	for _, tt := range cases {
		require.Equal(t, tt.expected, SumRow(tt.input, tt.rowIdx))
	}
}

func TestSigmoid1D(t *testing.T) {
	tests := []struct {
		name string
		x    []float64
		want []float64
	}{
		{
			name: "0",
			x:    []float64{0},
			want: []float64{0.5},
		},
		{
			name: "negative case",
			x:    []float64{-1},
			want: []float64{0.2689414213699951},
		},
		{
			name: "positive case",
			x:    []float64{1},
			want: []float64{0.7310585786300049},
		},
		{
			name: "multiple",
			x:    []float64{1, 2, 3},
			want: []float64{0.7310585786300049, 0.8807970779778823, 0.9525741268224334},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := Sigmoid1D(tt.x)
			assert.Equal(t, tt.want, got)
		})
	}
}

func benchmarkFlatten(n int) {
	// since we're flattening embeds, I'm fixing the length the inner arrays to 4000
	innerLen := 4000

	arr := make([][]int, n)
	for i := range arr {
		arr[i] = make([]int, innerLen)
	}

	Flatten(arr)
}

func BenchmarkFlatten1000(_ *testing.B)   { benchmarkFlatten(1000) }
func BenchmarkFlatten10000(_ *testing.B)  { benchmarkFlatten(10000) }
func BenchmarkFlatten100000(_ *testing.B) { benchmarkFlatten(100000) }
