package tokenizers

import (
	"testing"

	"github.com/stretchr/testify/require"
)

func benchmarkEncode(i int, b *testing.B) {
	encoder, err := GetTokenizer("50k")
	require.NoError(b, err)

	prompt := ""
	for x := 0; x < i; x++ {
		prompt += "oo"
	}
	for n := 0; n < b.N; n++ {
		_, err := encoder.Encode(prompt)
		require.NoError(b, err)
	}
}
func BenchmarkEncode128(b *testing.B)  { benchmarkEncode(128, b) }
func BenchmarkEncode256(b *testing.B)  { benchmarkEncode(256, b) }
func BenchmarkEncode512(b *testing.B)  { benchmarkEncode(512, b) }
func BenchmarkEncode1024(b *testing.B) { benchmarkEncode(1024, b) }
func BenchmarkEncode2048(b *testing.B) { benchmarkEncode(2048, b) }
func BenchmarkEncode4096(b *testing.B) { benchmarkEncode(4096, b) }
func BenchmarkEncode8192(b *testing.B) { benchmarkEncode(8192, b) }
