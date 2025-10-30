package filter

import (
	"testing"
)

func TestFilterImplBasic(t *testing.T) {
	filter := NewFilterImpl()
	defer filter.Free()

	decoded := "hello"
	tokenIDs := []uint32{1, 2, 3}
	logprobs := []float32{-1.0, -2.0, -3.0}

	outputs := filter.WriteDecoded(decoded, tokenIDs, logprobs)
	if outputs == nil {
		t.Fatal("expected outputs, got nil")
	}
	// Just check that the output struct is accessible and has expected fields
	for _, out := range outputs {
		_ = out.Text
		_ = out.TokenIDs
		_ = out.Logprobs
		_ = out.Citations
		_ = out.ToolCalls
		_ = out.SearchQuery
	}
}

