package gobindings

// #cgo LDFLAGS: ${SRCDIR}/../target/release/libcohere_melody.a -ldl -lm -lstdc++
// #include <stdlib.h>
// #include "melody.h"
import "C"
import (
	"encoding/json"
	"errors"
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
func (opts *FilterOptions) Cmd4() *FilterOptions {
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

// StreamToolActions enables streaming of tool actions
func (opts *FilterOptions) StreamToolActions() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_stream_tool_actions(opts.ptr)
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

		output.ToolCallDelta = tc
	}

	output.IsPostAnswer = bool(cOutput.is_post_answer)
	output.IsReasoning = bool(cOutput.is_reasoning)

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

// Templating enums (mirror ffi.rs C enums)
type Role int32

const (
	RoleUnknown Role = 0
	RoleSystem  Role = 1
	RoleUser    Role = 2
	RoleChatbot Role = 3
	RoleTool    Role = 4
)

type ContentType int32

const (
	ContentUnknown  ContentType = 0
	ContentText     ContentType = 1
	ContentThinking ContentType = 2
	ContentImage    ContentType = 3
	ContentDocument ContentType = 4
)

type CitationQuality int32

const (
	CitationQualityUnknown CitationQuality = 0
	CitationQualityOff     CitationQuality = 1
	CitationQualityOn      CitationQuality = 2
)

type Grounding int32

const (
	GroundingUnknown  Grounding = 0
	GroundingEnabled  Grounding = 1
	GroundingDisabled Grounding = 2
)

type SafetyMode int32

const (
	SafetyModeUnknown    SafetyMode = 0
	SafetyModeNone       SafetyMode = 1
	SafetyModeStrict     SafetyMode = 2
	SafetyModeContextual SafetyMode = 3
)

type ReasoningType int32

const (
	ReasoningTypeUnknown  ReasoningType = 0
	ReasoningTypeEnabled  ReasoningType = 1
	ReasoningTypeDisabled ReasoningType = 2
)

// Templating Go-side types
type Tool struct {
	Name        string
	Description string
	Parameters  map[string]any // will be JSON-encoded
}

type Image struct {
	TemplatePlaceholder string
}

type Content struct {
	Type     ContentType
	Text     string         // optional: empty means omitted
	Thinking string         // optional: empty means omitted
	Image    *Image         // optional
	Document map[string]any // optional: will be JSON-encoded
}

type ToolCall struct {
	ID         string
	Name       string
	Parameters map[string]any // will be JSON-encoded
}

type Message struct {
	Role       Role
	Content    []Content
	ToolCalls  []ToolCall
	ToolCallID string // optional: empty means omitted
}

type RenderCmd3Options struct {
	Messages                 []Message
	Template                 string
	DevInstruction           string           // optional: empty means omitted
	Documents                []map[string]any // JSON objects
	AvailableTools           []Tool
	SafetyMode               *SafetyMode      // optional
	CitationQuality          *CitationQuality // optional
	ReasoningType            *ReasoningType   // optional
	SkipPreamble             bool
	ResponsePrefix           string // optional: empty means omitted
	JSONSchema               string // optional: empty means omitted
	JSONMode                 bool
	AdditionalTemplateFields map[string]any    // optional: JSON-encoded
	EscapedSpecialTokens     map[string]string // optional: JSON-encoded
}

type RenderCmd4Options struct {
	Messages                 []Message
	Template                 string
	DevInstruction           string // optional: empty means omitted
	PlatformInstruction      string // optional: empty means omitted
	Documents                []map[string]any
	AvailableTools           []Tool
	Grounding                *Grounding // optional
	ResponsePrefix           string     // optional: empty means omitted
	JSONSchema               string     // optional: empty means omitted
	JSONMode                 bool
	AdditionalTemplateFields map[string]any    // optional
	EscapedSpecialTokens     map[string]string // optional
}

// Internal C allocator helper to track and free C allocations
type cAllocator struct {
	ptrs []unsafe.Pointer
}

func (a *cAllocator) CString(s string) *C.char {
	if s == "" {
		return nil
	}
	p := C.CString(s)
	a.ptrs = append(a.ptrs, unsafe.Pointer(p))
	return p
}

func (a *cAllocator) Malloc(size uintptr) unsafe.Pointer {
	if size == 0 {
		return nil
	}
	p := C.malloc(C.size_t(size))
	a.ptrs = append(a.ptrs, p)
	return p
}

func (a *cAllocator) FreeAll() {
	for i := len(a.ptrs) - 1; i >= 0; i-- {
		C.free(a.ptrs[i])
	}
	a.ptrs = nil
}

// Helpers to map Go enums to C enums
func roleToC(r Role) C.CRole                                  { return C.CRole(r) }
func contentTypeToC(t ContentType) C.CContentType             { return C.CContentType(t) }
func citationQualityToC(q CitationQuality) C.CCitationQuality { return C.CCitationQuality(q) }
func groundingToC(g Grounding) C.CGrounding                   { return C.CGrounding(g) }
func safetyModeToC(s SafetyMode) C.CSafetyMode                { return C.CSafetyMode(s) }
func reasoningTypeToC(rt ReasoningType) C.CReasoningType      { return C.CReasoningType(rt) }

func jsonCString(a *cAllocator, v any) *C.char {
	if v == nil {
		return nil
	}
	b, err := json.Marshal(v)
	if err != nil || len(b) == 0 {
		return nil
	}
	return a.CString(string(b))
}

func buildCDocuments(a *cAllocator, docs []map[string]any) (**C.char, C.size_t) {
	if len(docs) == 0 {
		return nil, 0
	}
	n := len(docs)
	// allocate array of *char in C memory
	size := uintptr(n) * unsafe.Sizeof((*C.char)(nil))
	base := (**C.char)(a.Malloc(size))
	arr := unsafe.Slice(base, n)
	for i := 0; i < n; i++ {
		arr[i] = jsonCString(a, docs[i])
	}
	return base, C.size_t(n)
}

func buildCTools(a *cAllocator, tools []Tool) (*C.CTool, C.size_t) {
	if len(tools) == 0 {
		return nil, 0
	}
	n := len(tools)
	var sample C.CTool
	size := uintptr(n) * unsafe.Sizeof(sample)
	base := (*C.CTool)(a.Malloc(size))
	var arr []C.CTool = unsafe.Slice(base, n)
	for i := 0; i < n; i++ {
		arr[i].name = a.CString(tools[i].Name)
		arr[i].description = a.CString(tools[i].Description)
		arr[i].parameters_json = jsonCString(a, tools[i].Parameters)
	}
	return base, C.size_t(n)
}

func buildCContents(a *cAllocator, contents []Content) (*C.CContent, C.size_t) {
	if len(contents) == 0 {
		return nil, 0
	}
	n := len(contents)
	var sample C.CContent
	size := uintptr(n) * unsafe.Sizeof(sample)
	base := (*C.CContent)(a.Malloc(size))
	var arr []C.CContent = unsafe.Slice(base, n)
	for i := 0; i < n; i++ {
		c := contents[i]
		arr[i].content_type = contentTypeToC(c.Type)
		if c.Text != "" {
			arr[i].text = a.CString(c.Text)
		}
		if c.Thinking != "" {
			arr[i].thinking = a.CString(c.Thinking)
		}
		// image (optional)
		if c.Image != nil {
			var imgSample C.CImage
			imgPtr := a.Malloc(unsafe.Sizeof(imgSample))
			img := (*C.CImage)(imgPtr)
			img.template_placeholder = a.CString(c.Image.TemplatePlaceholder)
			arr[i].image = img
		}
		// document_json (optional)
		if len(c.Document) > 0 {
			arr[i].document_json = jsonCString(a, c.Document)
		}
	}
	return base, C.size_t(n)
}

func buildCToolCalls(a *cAllocator, calls []ToolCall) (*C.CToolCall, C.size_t) {
	if len(calls) == 0 {
		return nil, 0
	}
	n := len(calls)
	var sample C.CToolCall
	size := uintptr(n) * unsafe.Sizeof(sample)
	base := (*C.CToolCall)(a.Malloc(size))
	var arr []C.CToolCall = unsafe.Slice(base, n)
	for i := 0; i < n; i++ {
		tc := calls[i]
		arr[i].id = a.CString(tc.ID)
		arr[i].name = a.CString(tc.Name)
		arr[i].parameters_json = jsonCString(a, tc.Parameters)
	}
	return base, C.size_t(n)
}

func buildCMessages(a *cAllocator, msgs []Message) (*C.CMessage, C.size_t) {
	if len(msgs) == 0 {
		return nil, 0
	}
	n := len(msgs)
	var sample C.CMessage
	size := uintptr(n) * unsafe.Sizeof(sample)
	base := (*C.CMessage)(a.Malloc(size))
	var arr []C.CMessage = unsafe.Slice(base, n)
	for i := 0; i < n; i++ {
		m := msgs[i]
		arr[i].role = roleToC(m.Role)

		// contents
		cContent, cContentLen := buildCContents(a, m.Content)
		arr[i].content = cContent
		arr[i].content_len = cContentLen

		// tool calls
		cCalls, cCallsLen := buildCToolCalls(a, m.ToolCalls)
		arr[i].tool_calls = cCalls
		arr[i].tool_calls_len = cCallsLen

		// optional tool_call_id
		if m.ToolCallID != "" {
			arr[i].tool_call_id = a.CString(m.ToolCallID)
		}
	}
	return base, C.size_t(n)
}

// RenderCMD3 renders CMD3 using the Rust templating engine via FFI.
func RenderCMD3(opts RenderCmd3Options) (string, error) {
	if opts.Template == "" {
		return "", errors.New("template is required")
	}

	var a cAllocator
	defer a.FreeAll()

	// Build nested arrays
	cMsgs, cMsgsLen := buildCMessages(&a, opts.Messages)
	cDocs, cDocsLen := buildCDocuments(&a, opts.Documents)
	cTools, cToolsLen := buildCTools(&a, opts.AvailableTools)

	// Optional enums with presence flags
	var cSafety C.CSafetyMode
	var hasSafety C.bool
	if opts.SafetyMode != nil {
		cSafety = safetyModeToC(*opts.SafetyMode)
		hasSafety = C.bool(true)
	}

	var cCitation C.CCitationQuality
	var hasCitation C.bool
	if opts.CitationQuality != nil {
		cCitation = citationQualityToC(*opts.CitationQuality)
		hasCitation = C.bool(true)
	}

	var cReason C.CReasoningType
	var hasReason C.bool
	if opts.ReasoningType != nil {
		cReason = reasoningTypeToC(*opts.ReasoningType)
		hasReason = C.bool(true)
	}

	// Optional strings
	devInstr := a.CString(opts.DevInstruction)
	respPrefix := a.CString(opts.ResponsePrefix)
	jsonSchema := a.CString(opts.JSONSchema)
	additionalFields := jsonCString(&a, opts.AdditionalTemplateFields)
	escapedTokens := jsonCString(&a, opts.EscapedSpecialTokens)

	// Build options struct (lives on Go stack; nested buffers are C-allocated)
	cOpts := C.CRenderCmd3Options{
		messages:                        cMsgs,
		messages_len:                    cMsgsLen,
		template:                        a.CString(opts.Template),
		dev_instruction:                 devInstr,
		documents_json:                  cDocs,
		documents_len:                   cDocsLen,
		available_tools:                 cTools,
		available_tools_len:             cToolsLen,
		safety_mode:                     cSafety,
		has_safety_mode:                 hasSafety,
		citation_quality:                cCitation,
		has_citation_quality:            hasCitation,
		reasoning_type:                  cReason,
		has_reasoning_type:              hasReason,
		skip_preamble:                   C.bool(opts.SkipPreamble),
		response_prefix:                 respPrefix,
		json_schema:                     jsonSchema,
		json_mode:                       C.bool(opts.JSONMode),
		additional_template_fields_json: additionalFields,
		escaped_special_tokens_json:     escapedTokens,
	}

	// Call into Rust
	cs := C.melody_render_cmd3(&cOpts)
	if cs == nil {
		return "", errors.New("melody_render_cmd3 returned null")
	}
	defer C.melody_string_free(cs)

	result := C.GoString(cs)
	runtime.KeepAlive(&cOpts)
	return result, nil
}

// RenderCMD4 renders CMD4 using the Rust templating engine via FFI.
func RenderCMD4(opts RenderCmd4Options) (string, error) {
	if opts.Template == "" {
		return "", errors.New("template is required")
	}

	var a cAllocator
	defer a.FreeAll()

	cMsgs, cMsgsLen := buildCMessages(&a, opts.Messages)
	cDocs, cDocsLen := buildCDocuments(&a, opts.Documents)
	cTools, cToolsLen := buildCTools(&a, opts.AvailableTools)

	var cGround C.CGrounding
	var hasGround C.bool
	if opts.Grounding != nil {
		cGround = groundingToC(*opts.Grounding)
		hasGround = C.bool(true)
	}

	devInstr := a.CString(opts.DevInstruction)
	platInstr := a.CString(opts.PlatformInstruction)
	respPrefix := a.CString(opts.ResponsePrefix)
	jsonSchema := a.CString(opts.JSONSchema)
	additionalFields := jsonCString(&a, opts.AdditionalTemplateFields)
	escapedTokens := jsonCString(&a, opts.EscapedSpecialTokens)

	cOpts := C.CRenderCmd4Options{
		messages:                        cMsgs,
		messages_len:                    cMsgsLen,
		template:                        a.CString(opts.Template),
		dev_instruction:                 devInstr,
		platform_instruction:            platInstr,
		documents_json:                  cDocs,
		documents_len:                   cDocsLen,
		available_tools:                 cTools,
		available_tools_len:             cToolsLen,
		grounding:                       cGround,
		has_grounding:                   hasGround,
		response_prefix:                 respPrefix,
		json_schema:                     jsonSchema,
		json_mode:                       C.bool(opts.JSONMode),
		additional_template_fields_json: additionalFields,
		escaped_special_tokens_json:     escapedTokens,
	}

	cs := C.melody_render_cmd4(&cOpts)
	if cs == nil {
		return "", errors.New("melody_render_cmd4 returned null")
	}
	defer C.melody_string_free(cs)

	result := C.GoString(cs)
	runtime.KeepAlive(&cOpts)
	return result, nil
}
