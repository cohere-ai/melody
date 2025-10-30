package melody

// #cgo LDFLAGS: -L${SRCDIR}/../rust/target/release -lmelody_parsing
// #include <stdlib.h>
// #include "melody.h"
import "C"
import (
	"runtime"
	"unsafe"
)

// cFilter is the internal CGO wrapper around the Rust filter
type cFilter struct {
	ptr *C.CFilter
}

// newCFilter creates a new C filter
func newCFilter() *cFilter {
	ptr := C.melody_filter_new()
	if ptr == nil {
		return nil
	}
	f := &cFilter{ptr: ptr}
	runtime.SetFinalizer(f, (*cFilter).free)
	return f
}

// newCFilterMultiHopCmd3 creates a new C filter with multi-hop CMD3 options
func newCFilterMultiHopCmd3(streamToolActions bool) *cFilter {
	ptr := C.melody_filter_new_multi_hop_cmd3(C.bool(streamToolActions))
	if ptr == nil {
		return nil
	}
	f := &cFilter{ptr: ptr}
	runtime.SetFinalizer(f, (*cFilter).free)
	return f
}

// newCFilterMultiHopCmd4 creates a new C filter with multi-hop CMD4 options
func newCFilterMultiHopCmd4(streamToolActions bool) *cFilter {
	ptr := C.melody_filter_new_multi_hop_cmd4(C.bool(streamToolActions))
	if ptr == nil {
		return nil
	}
	f := &cFilter{ptr: ptr}
	runtime.SetFinalizer(f, (*cFilter).free)
	return f
}

// newCFilterRAG creates a new C filter with RAG options
func newCFilterRAG() *cFilter {
	ptr := C.melody_filter_new_rag()
	if ptr == nil {
		return nil
	}
	f := &cFilter{ptr: ptr}
	runtime.SetFinalizer(f, (*cFilter).free)
	return f
}

// free releases the C filter resources
func (f *cFilter) free() {
	if f.ptr != nil {
		C.melody_filter_free(f.ptr)
		f.ptr = nil
	}
}

// writeDecoded writes a decoded token to the filter
func (f *cFilter) writeDecoded(decodedToken string, logprobs TokenIDsWithLogProb) []FilterOutput {
	if f.ptr == nil {
		return nil
	}

	cToken := C.CString(decodedToken)
	defer C.free(unsafe.Pointer(cToken))

	var cTokenIds *C.uint32_t
	var cLogprobs *C.float
	tokenIdsLen := C.size_t(len(logprobs.TokenIDs))
	logprobsLen := C.size_t(len(logprobs.Logprobs))

	if len(logprobs.TokenIDs) > 0 {
		tokenIds := make([]uint32, len(logprobs.TokenIDs))
		for i, id := range logprobs.TokenIDs {
			tokenIds[i] = uint32(id)
		}
		cTokenIds = (*C.uint32_t)(unsafe.Pointer(&tokenIds[0]))
	}

	if len(logprobs.Logprobs) > 0 {
		cLogprobs = (*C.float)(unsafe.Pointer(&logprobs.Logprobs[0]))
	}

	cArr := C.melody_filter_write_decoded(f.ptr, cToken, cTokenIds, tokenIdsLen, cLogprobs, logprobsLen)
	if cArr == nil {
		return nil
	}
	defer C.melody_filter_output_array_free(cArr)

	return convertCOutputArray(cArr)
}

// flushPartials flushes any partial outputs from the filter
func (f *cFilter) flushPartials() []FilterOutput {
	if f.ptr == nil {
		return nil
	}

	cArr := C.melody_filter_flush_partials(f.ptr)
	if cArr == nil {
		return nil
	}
	defer C.melody_filter_output_array_free(cArr)

	return convertCOutputArray(cArr)
}

// convertCOutputArray converts a C output array to Go FilterOutput slice
func convertCOutputArray(cArr *C.CFilterOutputArray) []FilterOutput {
	if cArr == nil || cArr.len == 0 {
		return nil
	}

	outputs := make([]FilterOutput, int(cArr.len))
	cOutputs := unsafe.Slice(cArr.outputs, int(cArr.len))

	for i := 0; i < int(cArr.len); i++ {
		outputs[i] = convertCOutput(&cOutputs[i])
	}

	return outputs
}

// convertCOutput converts a C output to Go FilterOutput
func convertCOutput(cOutput *C.CFilterOutput) FilterOutput {
	output := FilterOutput{}

	// Convert text
	if cOutput.text != nil {
		output.Text = C.GoString(cOutput.text)
	}

	// Convert logprobs
	if cOutput.token_ids != nil && cOutput.token_ids_len > 0 {
		tokenIds := unsafe.Slice(cOutput.token_ids, int(cOutput.token_ids_len))
		output.Logprobs.TokenIDs = make([]uint32, len(tokenIds))
		for i, id := range tokenIds {
			output.Logprobs.TokenIDs[i] = uint32(id)
		}
	}

	if cOutput.logprobs != nil && cOutput.logprobs_len > 0 {
		logprobs := unsafe.Slice(cOutput.logprobs, int(cOutput.logprobs_len))
		output.Logprobs.Logprobs = make([]float32, len(logprobs))
		for i, lp := range logprobs {
			output.Logprobs.Logprobs[i] = float32(lp)
		}
	}

	// Convert search query
	if cOutput.search_query_index >= 0 {
		output.SearchQuery = &FilterSearchQueryDelta{
			Index: int(cOutput.search_query_index),
			Text:  C.GoString(cOutput.search_query_text),
		}
	}

	// Convert citations
	if cOutput.citations != nil && cOutput.citations_len > 0 {
		cCitations := unsafe.Slice(cOutput.citations, int(cOutput.citations_len))
		output.Citations = make([]FilterCitation, len(cCitations))
		for i := 0; i < len(cCitations); i++ {
			output.Citations[i] = convertCCitation(&cCitations[i])
		}
	}

	// Convert tool calls
	if cOutput.tool_call_index >= 0 {
		tc := &FilterToolCallDelta{
			Index:         int(cOutput.tool_call_index),
			ID:            C.GoString(cOutput.tool_call_id),
			Name:          C.GoString(cOutput.tool_call_name),
			RawParamDelta: C.GoString(cOutput.tool_call_raw_param_delta),
		}

		if cOutput.tool_call_param_name != nil {
			tc.ParamDelta = &FilterToolParameter{
				Name:       C.GoString(cOutput.tool_call_param_name),
				ValueDelta: C.GoString(cOutput.tool_call_param_value_delta),
			}
		}

		output.ToolCalls = tc
	}

	output.IsPostAnswer = bool(cOutput.is_post_answer)
	output.IsToolsReason = bool(cOutput.is_tools_reason)

	return output
}

// convertCCitation converts a C citation to Go FilterCitation
func convertCCitation(cCitation *C.CFilterCitation) FilterCitation {
	citation := FilterCitation{
		StartIndex: int(cCitation.start_index),
		EndIndex:   int(cCitation.end_index),
		Text:       C.GoString(cCitation.text),
		IsThinking: bool(cCitation.is_thinking),
	}

	if cCitation.sources != nil && cCitation.sources_len > 0 {
		cSources := unsafe.Slice(cCitation.sources, int(cCitation.sources_len))
		citation.Sources = make([]Source, len(cSources))
		for i := 0; i < len(cSources); i++ {
			citation.Sources[i] = convertCSource(&cSources[i])
		}
	}

	return citation
}

// convertCSource converts a C source to Go Source
func convertCSource(cSource *C.CSource) Source {
	source := Source{
		ToolCallIndex: int(cSource.tool_call_index),
	}

	if cSource.tool_result_indices != nil && cSource.tool_result_indices_len > 0 {
		indices := unsafe.Slice(cSource.tool_result_indices, int(cSource.tool_result_indices_len))
		source.ToolResultIndices = make([]int, len(indices))
		for i, idx := range indices {
			source.ToolResultIndices[i] = int(idx)
		}
	}

	return source
}
