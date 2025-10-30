package filter

/*
#cgo LDFLAGS: ${SRCDIR}/libmelody.a -ldl -lm -lstdc++
#include <stdlib.h>
#include <stdint.h>

typedef struct {
    const uint32_t* token_ids;
    const float* logprobs;
    size_t len;
} CTokenIDsWithLogProb;

typedef struct {
    const char* text;
    const uint32_t* token_ids;
    const float* logprobs;
    size_t len;
} CFilterOutput;

typedef struct {
    CFilterOutput* outputs;
    size_t len;
} CFilterOutputVec;

void* filterimpl_new();
void filterimpl_free(void* ptr);
CFilterOutputVec filterimpl_write_decoded(
    void* ptr,
    const char* decoded_token,
    const uint32_t* token_ids,
    const float* logprobs,
    size_t len
);
CFilterOutputVec filterimpl_flush_partials(void* ptr);
void filterimpl_free_outputs(CFilterOutputVec vec);
*/
import "C"
import (
    "unsafe"
)

type FilterImpl struct {
    ptr unsafe.Pointer
}

type FilterSearchQueryDelta struct {
    Index int
    Text  string
}

type FilterToolParameter struct {
    Name       string
    ValueDelta string
}

type FilterToolCallDelta struct {
    Index        int
    ID           string
    Name         string
    ParamDelta   *FilterToolParameter
    RawParamDelta string
}

type Source struct {
    ToolCallIndex      int
    ToolResultIndices  []int
}

type FilterCitation struct {
    StartIndex int
    EndIndex   int
    Text       string
    Sources    []Source
    IsThinking bool
}

type FilterOutput struct {
    Text          string
    TokenIDs      []uint32
    Logprobs      []float32
    SearchQuery   *FilterSearchQueryDelta
    Citations     []FilterCitation
    ToolCalls     *FilterToolCallDelta
    IsPostAnswer  bool
    IsToolsReason bool
}

func NewFilterImpl() *FilterImpl {
    ptr := C.filterimpl_new()
    f := &FilterImpl{ptr: ptr}
    runtime.AddCleanup(f, func(p unsafe.Pointer) {
        C.filterimpl_free(p)
    }, f.ptr)
    return f
}

func (f *FilterImpl) Free() {
    if f.ptr != nil {
        C.filterimpl_free(f.ptr)
        f.ptr = nil
    }
}

func (f *FilterImpl) WriteDecoded(decodedToken string, tokenIDs []uint32, logprobs []float32) []FilterOutput {
    cToken := C.CString(decodedToken)
    defer C.free(unsafe.Pointer(cToken))

    var idsPtr *C.uint32_t
    var probsPtr *C.float
    if len(tokenIDs) > 0 {
        idsPtr = (*C.uint32_t)(unsafe.Pointer(&tokenIDs[0]))
    }
    if len(logprobs) > 0 {
        probsPtr = (*C.float)(unsafe.Pointer(&logprobs[0]))
    }

    outVec := C.filterimpl_write_decoded(
        f.ptr,
        cToken,
        idsPtr,
        probsPtr,
        C.size_t(len(tokenIDs)),
    )
    defer C.filterimpl_free_outputs(outVec)
    return cFilterOutputVecToGo(outVec)
}

func (f *FilterImpl) FlushPartials() []FilterOutput {
    outVec := C.filterimpl_flush_partials(f.ptr)
    defer C.filterimpl_free_outputs(outVec)
    return cFilterOutputVecToGo(outVec)
}

func cFilterOutputVecToGo(vec C.CFilterOutputVec) []FilterOutput {
    n := int(vec.len)
    if n == 0 || vec.outputs == nil {
        return nil
    }
    outputs := make([]FilterOutput, n)
    slice := (*[1 << 30]C.CFilterOutput)(unsafe.Pointer(vec.outputs))[:n:n]
    for i := 0; i < n; i++ {
        out := slice[i]
        text := C.GoString(out.text)
        ids := make([]uint32, int(out.len))
        probs := make([]float32, int(out.len))
        if out.token_ids != nil && out.len > 0 {
            idsSlice := (*[1 << 30]C.uint32_t)(unsafe.Pointer(out.token_ids))[:out.len:out.len]
            for j := range ids {
                ids[j] = uint32(idsSlice[j])
            }
        }
        if out.logprobs != nil && out.len > 0 {
            probsSlice := (*[1 << 30]C.float)(unsafe.Pointer(out.logprobs))[:out.len:out.len]
            for j := range probs {
                probs[j] = float32(probsSlice[j])
            }
        }
        // SearchQuery
        var searchQuery *FilterSearchQueryDelta
        if out.search_query != nil {
            sq := (*C.CFilterSearchQueryDelta)(unsafe.Pointer(out.search_query))
            searchQuery = &FilterSearchQueryDelta{
                Index: int(sq.index),
                Text:  C.GoString(sq.text),
            }
        }
        // Citations
        citations := make([]FilterCitation, int(out.citations_len))
        if out.citations != nil && out.citations_len > 0 {
            citSlice := (*[1 << 30]C.CFilterCitation)(unsafe.Pointer(out.citations))[:out.citations_len:out.citations_len]
            for j := range citations {
                cit := citSlice[j]
                sources := make([]Source, int(cit.sources_len))
                if cit.sources != nil && cit.sources_len > 0 {
                    srcSlice := (*[1 << 30]C.CSource)(unsafe.Pointer(cit.sources))[:cit.sources_len:cit.sources_len]
                    for k := range sources {
                        src := srcSlice[k]
                        indices := make([]int, int(src.tool_result_indices_len))
                        if src.tool_result_indices != nil && src.tool_result_indices_len > 0 {
                            idxSlice := (*[1 << 30]C.size_t)(unsafe.Pointer(src.tool_result_indices))[:src.tool_result_indices_len:src.tool_result_indices_len]
                            for l := range indices {
                                indices[l] = int(idxSlice[l])
                            }
                        }
                        sources[k] = Source{
                            ToolCallIndex:     int(src.tool_call_index),
                            ToolResultIndices: indices,
                        }
                    }
                }
                citations[j] = FilterCitation{
                    StartIndex: int(cit.start_index),
                    EndIndex:   int(cit.end_index),
                    Text:       C.GoString(cit.text),
                    Sources:    sources,
                    IsThinking: bool(cit.is_thinking),
                }
            }
        }
        // ToolCalls
        var toolCalls *FilterToolCallDelta
        if out.tool_calls != nil {
            tc := (*C.CFilterToolCallDelta)(unsafe.Pointer(out.tool_calls))
            var paramDelta *FilterToolParameter
            if tc.param_delta != nil {
                param := (*C.CFilterToolParameter)(unsafe.Pointer(tc.param_delta))
                paramDelta = &FilterToolParameter{
                    Name:       C.GoString(param.name),
                    ValueDelta: C.GoString(param.value_delta),
                }
            }
            toolCalls = &FilterToolCallDelta{
                Index:        int(tc.index),
                ID:           C.GoString(tc.id),
                Name:         C.GoString(tc.name),
                ParamDelta:   paramDelta,
                RawParamDelta: C.GoString(tc.raw_param_delta),
            }
        }
        outputs[i] = FilterOutput{
            Text:          text,
            TokenIDs:      ids,
            Logprobs:      probs,
            SearchQuery:   searchQuery,
            Citations:     citations,
            ToolCalls:     toolCalls,
            IsPostAnswer:  bool(out.is_post_answer),
            IsToolsReason: bool(out.is_tools_reason),
        }
    }
    return outputs
}
