package melody

// #cgo LDFLAGS: -L${SRCDIR}/../target/release -lcohere_melody
// #include <stdlib.h>
// #include "melody.h"
import "C"
import (
	"runtime"
	"unsafe"
)

// FilterOptions is the Go wrapper around CFilterOptions
type FilterOptions struct {
	ptr *C.CFilterOptions
}

// NewFilterOptions creates a new FilterOptions instance
func NewFilterOptions() *FilterOptions {
	ptr := C.melody_filter_options_new()
	if ptr == nil {
		return nil
	}
	opts := &FilterOptions{ptr: ptr}
	runtime.SetFinalizer(opts, (*FilterOptions).Free)
	return opts
}

// Free releases the FilterOptions resources
func (opts *FilterOptions) Free() {
	if opts.ptr != nil {
		C.melody_filter_options_free(opts.ptr)
		opts.ptr = nil
	}
}

// Cmd3 configures options for multi-hop CMD3 format
func (opts *FilterOptions) Cmd3() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_cmd3(opts.ptr)
	}
	return opts
}

// HandleMultiHopCmd4 configures options for multi-hop CMD4 format
func (opts *FilterOptions) HandleMultiHopCmd4() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_cmd4(opts.ptr)
	}
	return opts
}

// HandleRAG configures options for RAG format
func (opts *FilterOptions) HandleRAG() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_handle_rag(opts.ptr)
	}
	return opts
}

// HandleSearchQuery configures options for search query handling
func (opts *FilterOptions) HandleSearchQuery() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_handle_search_query(opts.ptr)
	}
	return opts
}

// HandleMultiHop configures options for multi-hop format
func (opts *FilterOptions) HandleMultiHop() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_handle_multi_hop(opts.ptr)
	}
	return opts
}

// StreamNonGroundedAnswer enables streaming of non-grounded answer
func (opts *FilterOptions) StreamNonGroundedAnswer() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_stream_non_grounded_answer(opts.ptr)
	}
	return opts
}

// StreamProcessedParams enables streaming of processed parameters
func (opts *FilterOptions) StreamProcessedParams() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_stream_processed_params(opts.ptr)
	}
	return opts
}

// WithLeftTrimmed enables left trimming
func (opts *FilterOptions) WithLeftTrimmed() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_with_left_trimmed(opts.ptr)
	}
	return opts
}

// WithRightTrimmed enables right trimming
func (opts *FilterOptions) WithRightTrimmed() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_with_right_trimmed(opts.ptr)
	}
	return opts
}

// WithPrefixTrim sets a prefix to trim
func (opts *FilterOptions) WithPrefixTrim(prefix string) *FilterOptions {
	if opts.ptr != nil {
		cPrefix := C.CString(prefix)
		defer C.free(unsafe.Pointer(cPrefix))
		C.melody_filter_options_with_prefix_trim(opts.ptr, cPrefix)
	}
	return opts
}

// WithChunkSize sets the chunk size
func (opts *FilterOptions) WithChunkSize(size int) *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_with_chunk_size(opts.ptr, C.size_t(size))
	}
	return opts
}

// WithInclusiveStops sets inclusive stop sequences
func (opts *FilterOptions) WithInclusiveStops(stops []string) *FilterOptions {
	if opts.ptr != nil && len(stops) > 0 {
		cStops := make([]*C.char, len(stops))
		for i, stop := range stops {
			cStops[i] = C.CString(stop)
		}
		C.melody_filter_options_with_inclusive_stops(opts.ptr, (**C.char)(unsafe.Pointer(&cStops[0])), C.size_t(len(stops)))

		// Free all C strings after the call
		for _, cStr := range cStops {
			C.free(unsafe.Pointer(cStr))
		}
	}
	return opts
}

// WithExclusiveStops sets exclusive stop sequences
func (opts *FilterOptions) WithExclusiveStops(stops []string) *FilterOptions {
	if opts.ptr != nil && len(stops) > 0 {
		cStops := make([]*C.char, len(stops))
		for i, stop := range stops {
			cStops[i] = C.CString(stop)
		}
		C.melody_filter_options_with_exclusive_stops(opts.ptr, (**C.char)(unsafe.Pointer(&cStops[0])), C.size_t(len(stops)))

		// Free all C strings after the call
		for _, cStr := range cStops {
			C.free(unsafe.Pointer(cStr))
		}
	}
	return opts
}

// RemoveToken removes a specific token from the output
func (opts *FilterOptions) RemoveToken(token string) *FilterOptions {
	if opts.ptr != nil {
		cToken := C.CString(token)
		defer C.free(unsafe.Pointer(cToken))
		C.melody_filter_options_remove_token(opts.ptr, cToken)
	}
	return opts
}

// cFilter is the internal CGO wrapper around the Rust filter
type cFilter struct {
	ptr *C.CFilter
}

// newCFilter creates a new C filter with the given options
func newCFilter(options *FilterOptions) *cFilter {
	var ptr *C.CFilter
	if options == nil {
		ptr = C.melody_filter_new(nil)
	} else {
		ptr = C.melody_filter_new(options.ptr)
	}
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
