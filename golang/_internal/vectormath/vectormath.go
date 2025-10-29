package vectormath

import (
	"bytes"
	"encoding/base64"
	"encoding/binary"
	"fmt"
	"math"
	"unsafe"

	"github.com/x448/float16"
	"golang.org/x/exp/constraints"
)

// use uint16 instead of ~uint16 because float16.Float16 can't by converted like other unsigned integers
type Numeric interface {
	~uint | ~uint8 | uint16 | ~uint32 | ~uint64 | ~uintptr | constraints.Signed | constraints.Float
}

type SizedNumeric interface {
	~int8 | ~int16 | ~int32 | ~int64 | ~uint8 | ~uint16 | ~uint32 | ~uint64 | constraints.Float
}

func Abs[T Numeric](v T) T {
	if v < 0 {
		return -v
	}
	return v
}

// Sum returns the total sum of every element in the vector.
func Sum[T Numeric](a []T) T {
	var sum T
	for _, v := range a {
		sum += v
	}
	return sum
}

// SumLengths returns the sum of the lengths of all rows in the tensor.
func SumLengths[T Numeric](t [][]T) int {
	var length int
	for _, row := range t {
		length += len(row)
	}
	return length
}

// SumColumnwise produces a vector of size (N, 1) from a tensor of size (M, N)
// by summing together each column.
// For example:
//
//	[][]int{
//	  {1, 2, 3},
//	  {4, 5, 6}
//	}
//
// becomes
//
//	[]int{5, 7, 9}
func SumColumnwise[T Numeric](t [][]T) []T {
	result := make([]T, len(t[0]))
	for c := range t[0] {
		var sum T = 0
		for r := range t {
			sum += t[r][c]
		}
		result[c] = sum
	}
	return result
}

func SumColumn[T Numeric](t [][]T, col int) T {
	if len(t) == 0 {
		return 0
	}

	var sum T = 0
	for r := range t {
		sum += t[r][col]
	}
	return sum
}

func SumRow[T Numeric](t [][]T, row int) T {
	if len(t) == 0 {
		return 0
	}
	var sum T = 0
	for c := range t[row] {
		sum += t[row][c]
	}
	return sum
}

// MulByConstant multiplies each element in the vector by a given multiplier.
func MulByConstant[T Numeric](a []T, multiplier T) []T {
	result := make([]T, len(a))
	for i, v := range a {
		result[i] = v * multiplier
	}
	return result
}

// Exp returns a vector containing the exponential of each element.
func Exp[T constraints.Float](a []T) []float64 {
	result := make([]float64, len(a))
	for i, val := range a {
		result[i] = math.Exp(float64(val))
	}
	return result
}

// Flatten flattens the given tensor into a vector.
func Flatten[T Numeric](t [][]T) []T {
	l := 0
	for _, x := range t {
		l += len(x)
	}
	result := make([]T, l)

	s := 0
	for _, x := range t {
		copy(result[s:s+len(x)], x)
		s += len(x)
	}
	return result
}

// FlattenConvert takes a 2D array of Numeric and flattens them to a different Numeric.
// This _could_ have been written as Flatten, and then convert, but we combine
// these two operations in oder to save calls to malloc
func FlattenConvert[T Numeric, U Numeric](t [][]T) []U {
	l := 0
	for _, x := range t {
		l += len(x)
	}
	result := make([]U, l)

	i := 0
	for _, r := range t {
		for _, x := range r {
			result[i] = U(x)
			i += 1
		}
	}
	return result
}

// Unflatten converts a vector into a tensor by breaking the vector into chunks
// of the given size. If the vector has a length N, the resulting tensor will
// have a the shape (N/size, size).
func Unflatten[T Numeric](a []T, size int) [][]T {
	if len(a) == 0 {
		return [][]T{{}}
	}

	result := make([][]T, 0, (len(a)+size-1)/size)
	for i := 0; i < len(a); i += size {
		end := i + size
		if end > len(a) {
			end = len(a)
		}
		result = append(result, a[i:end])
	}
	return result
}

// Reshape reshapes a given 2-D tensor with N elements into a new tensor
// with the shape (size, D) where D = N/size.
// If the size is not a multiple of the total number of elements, it returns a panic.
func Reshape[T Numeric](t [][]T, size int) [][]T {
	result := [][]T{}

	numElements := 0
	for _, r := range t {
		numElements += len(r)
	}
	if numElements%size != 0 {
		panic(fmt.Errorf("size must be a factor of the total number of elements in the tensor %d", numElements))
	}
	dims := numElements / size

	count := 0
	row := make([]T, 0, dims)
	for i := range t {
		for j := range t[i] {
			row = append(row, t[i][j])
			count++

			if count >= dims {
				result = append(result, row)
				row = make([]T, 0, dims)
				count = 0
			}
		}
	}

	if count > 0 {
		result = append(result, row)
	}
	return result
}

// Convert transforms a vector of type T into a vector of type U.
func Convert[U Numeric, T Numeric](a []T) []U {
	result := make([]U, len(a))
	for i := range a {
		result[i] = U(a[i])
	}
	return result
}

// Convert transforms a vector of type float16 into a vector of type float32.
func ConvertF16ToF32(a []float16.Float16) []float32 {
	result := make([]float32, len(a))
	for i := range a {
		result[i] = a[i].Float32()
	}
	return result
}

// Convert transforms a vector of type float32 into a vector of type float16.
func ConvertF32ToF16(a []float32) []float16.Float16 {
	result := make([]float16.Float16, len(a))
	for i := range a {
		result[i] = float16.Fromfloat32(a[i])
	}
	return result
}

func SerializeToBytes[T SizedNumeric](a []T) []byte {
	var buf bytes.Buffer
	for _, v := range a {
		err := binary.Write(&buf, binary.LittleEndian, v)
		if err != nil {
			panic(err)
		}
	}
	return buf.Bytes()
}

func DecodeString(b []byte) string {
	return DecodeStrings(b)[0]
}

func DecodeStrings(b []byte) []string {
	i := 0
	result := []string{}
	for i < len(b) {
		numStringBytes := int(binary.LittleEndian.Uint32(b[i : i+4]))
		i += 4
		str := string(b[i : i+numStringBytes])
		result = append(result, str)
		i += numStringBytes
	}
	return result
}

func EncodeString(s string) []byte {
	stringBytes := []byte(s)

	// the first 4 bytes are designated to indicate the length of the string
	result := make([]byte, 4)
	binary.LittleEndian.PutUint32(result, uint32(len(stringBytes)))

	// the rest is going to be the string data itself
	result = append(result, stringBytes...)
	return result
}

func encodeEntry(in []byte, out *[]byte) {
	// each entry starts with 4 bytes designated to indicate the length of the string
	lengthIndicatorSegment := make([]byte, 4)
	binary.LittleEndian.PutUint32(lengthIndicatorSegment, uint32(len(in)))

	*out = append(*out, lengthIndicatorSegment...)
	*out = append(*out, in...)
}

func EncodeStrings(strings []string) []byte {
	result := []byte{}
	for _, s := range strings {
		encodeEntry([]byte(s), &result)
	}
	return result
}

func EncodeImages(images [][]byte) []byte {
	result := []byte{}
	for _, i := range images {
		b64Img := make([]byte, base64.StdEncoding.EncodedLen(len(i)))
		base64.StdEncoding.Encode(b64Img, i)
		encodeEntry(b64Img, &result)
	}
	return result
}

func DeserializeFromBytes[T SizedNumeric](b []byte) []T {
	numBytes := int(unsafe.Sizeof(T(0)))
	if len(b)%numBytes != 0 {
		panic(fmt.Errorf("byte slice must have a length which is a multiple of %d", numBytes))
	}

	reader := bytes.NewReader(b)
	result := make([]T, 0, len(b)/numBytes)
	for i := 0; i < len(b); i += numBytes {
		var value T
		err := binary.Read(reader, binary.LittleEndian, &value)
		if err != nil {
			panic(err)
		}
		result = append(result, value)
	}
	return result
}

func MarshalBool(b bool) []byte {
	if b {
		return []byte{1}
	}
	return []byte{0}
}

func MarshalValue[T SizedNumeric](v T) []byte {
	var buf bytes.Buffer
	err := binary.Write(&buf, binary.LittleEndian, v)
	if err != nil {
		panic(err)
	}
	return buf.Bytes()
}

func UnmarshalValue[T SizedNumeric](b []byte) T {
	numBytes := int(unsafe.Sizeof(T(0)))
	if len(b) != numBytes {
		panic(fmt.Errorf("byte slice must have a length which is equal to %d", numBytes))
	}

	var value T
	err := binary.Read(bytes.NewReader(b), binary.LittleEndian, &value)
	if err != nil {
		panic(err)
	}
	return value
}

// Pad2D pads out all rows in the tensor to the length of the largest row.
func Pad2D[T Numeric](t [][]T) {
	largest := -1
	for _, t := range t {
		if len(t) > largest {
			largest = len(t)
		}
	}
	PadToLength2D(t, largest)
}

// PadToLength2D pads out all rows in the tensor to the given length,
// truncating rows which exceed it.
func PadToLength2D[T Numeric](t [][]T, targetLength int) {
	PadToLength2DNonZeroPadToken(t, targetLength, 0)
}

// PadToLength2DJNonZeroPadToken pads out all rows in the tensor to the given length
// padding with non zero pad token provided in function, and returns
// the number of padded tokens.
func PadToLength2DNonZeroPadToken[T Numeric](t [][]T, targetLength int, padToken T) []int {
	paddedTokens := make([]int, len(t))
	for i := range t {
		l := len(t[i])
		paddedTokens[i] = targetLength - l
		if l == targetLength {
			continue
		}
		padded := make([]T, targetLength)
		copy(padded, t[i])

		// pad with padToken if padding is required
		if padToken != 0 {
			for padIdx := l; padIdx < targetLength; padIdx++ {
				padded[padIdx] = padToken
			}
		}

		t[i] = padded
	}
	return paddedTokens
}

// PadToLength2DNonZeroPadTokenLeft is identical to PadToLength2DJNonZeroPadToken,
// except it adds padded tokens to the left instead of the right. Note that truncation is still from the right.
// TODO: combine with PadToLength2DJNonZeroPadToken and add a direction arg
func PadToLength2DNonZeroPadTokenLeft[T Numeric](t [][]T, targetLength int, padToken T) []int {
	paddedTokens := make([]int, len(t))
	for i := range t {
		l := len(t[i])
		paddedTokens[i] = targetLength - l
		if l == targetLength {
			continue
		}

		padded := make([]T, targetLength)
		offset := targetLength - l
		if offset >= 0 {
			copy(padded[offset:], t[i])

			// pad from left with padToken if padding is required
			if padToken != 0 {
				for i := range offset {
					padded[i] = padToken
				}
			}
		} else {
			// actual length exceeds target length, so we truncate instead of padding
			copy(padded, t[i])
		}

		t[i] = padded
	}
	return paddedTokens
}

func ArgMinIndex[T Numeric](a []T) int {
	argMin := 0
	curMin := a[0]
	for i, v := range a {
		if curMin > v {
			curMin = v
			argMin = i
		}
	}
	return argMin
}

func ArgMaxIndex[T Numeric](a []T) int {
	argMax := 0
	curMax := a[0]
	for i, v := range a {
		if curMax < v {
			curMax = v
			argMax = i
		}
	}
	return argMax
}

func GetSizes[U Numeric, T Numeric](a [][]T) []U {
	result := make([]U, len(a))
	for i, v := range a {
		result[i] = U(len(v))
	}
	return result
}

func GetStacked[T Numeric](base T, stackSize int32) []T {
	stacked := make([]T, stackSize)
	for i := range stacked {
		stacked[i] = base
	}
	return stacked
}

func GetStackedEncodedPrompts(encodedPrompts [][]int64, stackSize int32) [][]int64 {
	stacked := make([][]int64, stackSize)
	for i := range stacked {
		stacked[i] = encodedPrompts[i%len(encodedPrompts)]
	}
	return stacked
}

// CreateChunks is used to chunk data into contextLength sized chunks so it can be used for the model.
// The last slice will be padded with zeros if it is not a multiple of contextLength.
func CreateChunks[T Numeric](slice []T, contextLength int) [][]T {
	chunks := Unflatten(slice, contextLength)
	PadToLength2D(chunks, contextLength)
	return chunks
}

func Sigmoid1D[T Numeric](a []T) []T {
	result := make([]T, len(a))
	for i, v := range a {
		result[i] = T(1 / (1 + math.Exp(-float64(v))))
	}
	return result
}
