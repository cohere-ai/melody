package gobindings

// #cgo LDFLAGS: ${SRCDIR}/../target/release/libcohere_melody.a -ldl -lm -lstdc++
// #include <stdlib.h>
// #include "melody.h"
import "C"
import (
	"encoding/json"
	"errors"
	"runtime"
	"strings"
	"unsafe"

	"github.com/cohere-ai/melody/gobindings/orderedjson"
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

// Cmd5 configures options for multi-hop CMD5 format
func (opts *FilterOptions) Cmd5() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_cmd5(opts.ptr)
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

// CoflNoXMLTextDecode disables XML entity decoding for cofl parameter bodies.
func (opts *FilterOptions) CoflNoXMLTextDecode() *FilterOptions {
	if opts.ptr != nil {
		C.melody_filter_options_cofl_no_xml_text_decode(opts.ptr)
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
func (f *cFilter) writeDecoded(decodedToken string) (*AggregatedResult, error) {
	if f.ptr == nil {
		return nil, nil
	}

	cToken := C.CString(decodedToken)
	defer C.free(unsafe.Pointer(cToken))

	res := C.melody_filter_write_decoded(f.ptr, cToken)
	if res == nil {
		return nil, nil
	}
	defer C.melody_aggregated_result_free(res)

	if res.error != nil {
		return nil, errors.New(C.GoString(res.error))
	}

	return convertCAggregatedResult(res.result), nil
}

// flushPartials flushes any partial outputs from the filter
func (f *cFilter) flushPartials() (*AggregatedResult, error) {
	if f.ptr == nil {
		return nil, nil
	}

	res := C.melody_filter_flush_partials(f.ptr)
	if res == nil {
		return nil, nil
	}
	defer C.melody_aggregated_result_free(res)

	if res.error != nil {
		return nil, errors.New(C.GoString(res.error))
	}

	return convertCAggregatedResult(res.result), nil
}

// Helper to convert C array of FilterToolParameter to Go slice
func convertCFilterToolParameters(cParams *C.CFilterToolParameter, length C.size_t) []FilterToolParameter {
	if cParams == nil || length == 0 {
		return nil
	}
	arr := unsafe.Slice(cParams, int(length))
	params := make([]FilterToolParameter, len(arr))
	for i, cp := range arr {
		params[i] = FilterToolParameter{
			Name:       C.GoString(cp.name),
			ValueDelta: C.GoString(cp.value_delta),
		}
	}
	return params
}

// convertCAggregatedResult converts a C aggregated result to Go AggregatedResult
func convertCAggregatedResult(cResult *C.CAggregatedResult) *AggregatedResult {
	if cResult == nil {
		return nil
	}
	result := &AggregatedResult{}

	if cResult.content != nil {
		s := C.GoString(cResult.content)
		result.Content = &s
	}
	if cResult.reasoning != nil {
		s := C.GoString(cResult.reasoning)
		result.Reasoning = &s
	}

	if cResult.tool_calls != nil && cResult.tool_calls_len > 0 {
		cToolCalls := unsafe.Slice(cResult.tool_calls, int(cResult.tool_calls_len))
		result.ToolCalls = make([]AccumulatedToolCall, len(cToolCalls))
		for i, ctc := range cToolCalls {
			result.ToolCalls[i] = AccumulatedToolCall{
				Index:           uint(ctc.index),
				ID:              C.GoString(ctc.id),
				Name:            C.GoString(ctc.name),
				Arguments:       C.GoString(ctc.arguments),
				ProcessedParams: convertCFilterToolParameters(ctc.processed_params, ctc.processed_params_len),
			}
		}
	}

	if cResult.citations != nil && cResult.citations_len > 0 {
		cCitations := unsafe.Slice(cResult.citations, int(cResult.citations_len))
		result.Citations = make([]FilterCitation, len(cCitations))
		for i := 0; i < len(cCitations); i++ {
			result.Citations[i] = convertCCitation(&cCitations[i])
		}
	}

	if cResult.search_queries != nil && cResult.search_queries_len > 0 {
		cSQs := unsafe.Slice(cResult.search_queries, int(cResult.search_queries_len))
		result.SearchQueries = make([]SearchQueryDelta, len(cSQs))
		for i, csq := range cSQs {
			result.SearchQueries[i] = SearchQueryDelta{
				Index: uint(csq.index),
				Text:  C.GoString(csq.text),
			}
		}
	}

	return result
}

// convertCCitation converts a C citation to Go FilterCitation
func convertCCitation(cCitation *C.CFilterCitation) FilterCitation {
	citation := FilterCitation{
		StartIndex: uint(cCitation.start_index),
		EndIndex:   uint(cCitation.end_index),
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
		ToolCallIndex: uint(cSource.tool_call_index),
	}

	if cSource.tool_result_indices != nil && cSource.tool_result_indices_len > 0 {
		indices := unsafe.Slice(cSource.tool_result_indices, int(cSource.tool_result_indices_len))
		source.ToolResultIndices = make([]uint, len(indices))
		for i, idx := range indices {
			source.ToolResultIndices[i] = uint(idx)
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
	ContentUnknown   ContentType = 0
	ContentText      ContentType = 1
	ContentThinking  ContentType = 2
	ContentImage     ContentType = 3
	ContentDocument  ContentType = 4
	ContentMultipart ContentType = 5
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

// Unmarshalers for enums (case-insensitive string support; numbers map directly)

func (r *Role) UnmarshalJSON(data []byte) error {
	var s string
	if err := json.Unmarshal(data, &s); err == nil {
		switch strings.ToLower(strings.TrimSpace(s)) {
		case "unknown":
			*r = RoleUnknown
		case "system":
			*r = RoleSystem
		case "user":
			*r = RoleUser
		case "chatbot", "assistant":
			*r = RoleChatbot
		case "tool":
			*r = RoleTool
		default:
			return errors.New("invalid Role: " + s)
		}
		return nil
	}
	var n int32
	if err := json.Unmarshal(data, &n); err == nil {
		*r = Role(n)
		return nil
	}
	return errors.New("Role must be a string or number")
}

func (t *ContentType) UnmarshalJSON(data []byte) error {
	var s string
	if err := json.Unmarshal(data, &s); err == nil {
		switch strings.ToLower(strings.TrimSpace(s)) {
		case "unknown":
			*t = ContentUnknown
		case "text":
			*t = ContentText
		case "thinking":
			*t = ContentThinking
		case "image":
			*t = ContentImage
		case "document":
			*t = ContentDocument
		case "multipart":
			*t = ContentMultipart
		default:
			return errors.New("invalid ContentType: " + s)
		}
		return nil
	}
	var n int32
	if err := json.Unmarshal(data, &n); err == nil {
		*t = ContentType(n)
		return nil
	}
	return errors.New("ContentType must be a string or number")
}

func (q *CitationQuality) UnmarshalJSON(data []byte) error {
	var s string
	if err := json.Unmarshal(data, &s); err == nil {
		switch strings.ToLower(strings.TrimSpace(s)) {
		case "unknown":
			*q = CitationQualityUnknown
		case "off", "disabled", "false", "0":
			*q = CitationQualityOff
		case "on", "enabled", "true", "1":
			*q = CitationQualityOn
		default:
			return errors.New("invalid CitationQuality: " + s)
		}
		return nil
	}
	var n int32
	if err := json.Unmarshal(data, &n); err == nil {
		*q = CitationQuality(n)
		return nil
	}
	return errors.New("CitationQuality must be a string or number")
}

func (g *Grounding) UnmarshalJSON(data []byte) error {
	var s string
	if err := json.Unmarshal(data, &s); err == nil {
		switch strings.ToLower(strings.TrimSpace(s)) {
		case "unknown":
			*g = GroundingUnknown
		case "enabled", "on", "true", "1":
			*g = GroundingEnabled
		case "disabled", "off", "false", "0":
			*g = GroundingDisabled
		default:
			return errors.New("invalid Grounding: " + s)
		}
		return nil
	}
	var n int32
	if err := json.Unmarshal(data, &n); err == nil {
		*g = Grounding(n)
		return nil
	}
	return errors.New("Grounding must be a string or number")
}

func (s *SafetyMode) UnmarshalJSON(data []byte) error {
	var str string
	if err := json.Unmarshal(data, &str); err == nil {
		switch strings.ToLower(strings.TrimSpace(str)) {
		case "unknown":
			*s = SafetyModeUnknown
		case "none":
			*s = SafetyModeNone
		case "strict":
			*s = SafetyModeStrict
		case "contextual":
			*s = SafetyModeContextual
		default:
			return errors.New("invalid SafetyMode: " + str)
		}
		return nil
	}
	var n int32
	if err := json.Unmarshal(data, &n); err == nil {
		*s = SafetyMode(n)
		return nil
	}
	return errors.New("SafetyMode must be a string or number")
}

func (rt *ReasoningType) UnmarshalJSON(data []byte) error {
	var s string
	if err := json.Unmarshal(data, &s); err == nil {
		switch strings.ToLower(strings.TrimSpace(s)) {
		case "unknown":
			*rt = ReasoningTypeUnknown
		case "enabled", "on", "true", "1":
			*rt = ReasoningTypeEnabled
		case "disabled", "off", "false", "0":
			*rt = ReasoningTypeDisabled
		default:
			return errors.New("invalid ReasoningType: " + s)
		}
		return nil
	}
	var n int32
	if err := json.Unmarshal(data, &n); err == nil {
		*rt = ReasoningType(n)
		return nil
	}
	return errors.New("ReasoningType must be a string or number")
}

// Templating Go-side types
type Tool struct {
	Name        string             `json:"name"`
	Description string             `json:"description,omitempty"`
	Parameters  orderedjson.Object `json:"parameters,omitempty"`
}

type Image struct {
	TemplatePlaceholder string `json:"template_placeholder"`
}

type Content struct {
	Type      ContentType        `json:"type"`
	Text      string             `json:"text,omitempty"`     // optional: empty means omitted
	Thinking  string             `json:"thinking,omitempty"` // optional: empty means omitted
	Image     *Image             `json:"image,omitempty"`    // optional
	Document  orderedjson.Object `json:"document,omitempty"`
	Multipart []Content          `json:"multipart,omitempty"` // optional
}

type ToolCall struct {
	ID         string `json:"id"`
	Name       string `json:"name"`
	Parameters string `json:"parameters,omitempty"`
}

type Message struct {
	Role       Role             `json:"role"`
	Content    []Content        `json:"content"`
	ToolCalls  []ToolCall       `json:"tool_calls,omitempty"`
	ToolCallID string           `json:"tool_call_id,omitempty"` // optional: empty means omitted
	Citations  []FilterCitation `json:"citations,omitempty"`
}

type RenderCmd3Options struct {
	Messages                 []Message            `json:"messages"`
	TemplateID               *string              `json:"template_id,omitempty"`
	Template                 string               `json:"template"`
	TemplateJinja            string               `json:"template_jinja"`
	UseJinja                 bool                 `json:"use_jinja"`
	DevInstruction           *string              `json:"dev_instruction,omitempty"`
	Documents                []orderedjson.Object `json:"documents,omitempty"` // JSON objects
	AvailableTools           []Tool               `json:"available_tools,omitempty"`
	SafetyMode               *SafetyMode          `json:"safety_mode,omitempty"`      // optional
	CitationQuality          *CitationQuality     `json:"citation_quality,omitempty"` // optional
	ReasoningType            *ReasoningType       `json:"reasoning_type,omitempty"`   // optional
	SkipPreamble             bool                 `json:"skip_preamble,omitempty"`
	ResponsePrefix           *string              `json:"response_prefix,omitempty"`
	JSONSchema               *string              `json:"json_schema,omitempty"`
	JSONMode                 bool                 `json:"json_mode,omitempty"`
	AdditionalTemplateFields map[string]any       `json:"additional_template_fields,omitempty"` // optional: JSON-encoded
	EscapedSpecialTokens     map[string]string    `json:"escaped_special_tokens,omitempty"`     // optional: JSON-encoded
}

type RenderCmd4Options struct {
	Messages                 []Message            `json:"messages"`
	TemplateID               *string              `json:"template_id,omitempty"`
	Template                 string               `json:"template"`
	TemplateJinja            string               `json:"template_jinja"`
	UseJinja                 bool                 `json:"use_jinja"`
	DevInstruction           *string              `json:"dev_instruction,omitempty"`
	PlatformInstruction      *string              `json:"platform_instruction,omitempty"`
	Documents                []orderedjson.Object `json:"documents,omitempty"`
	AvailableTools           []Tool               `json:"available_tools,omitempty"`
	Grounding                *Grounding           `json:"grounding,omitempty"` // optional
	ReasoningType            *ReasoningType       `json:"reasoning_type,omitempty"`
	ResponsePrefix           *string              `json:"response_prefix,omitempty"`
	JSONSchema               *string              `json:"json_schema,omitempty"`
	JSONMode                 bool                 `json:"json_mode,omitempty"`
	AdditionalTemplateFields map[string]any       `json:"additional_template_fields,omitempty"` // optional
	EscapedSpecialTokens     map[string]string    `json:"escaped_special_tokens,omitempty"`     // optional
}

// RenderCmd5Options uses the same shape as RenderCmd4Options. The CMD5 render
// path differs only in which jinja template is used by default.
type RenderCmd5Options = RenderCmd4Options

// Internal C allocator helper to track and free C allocations
type cAllocator struct {
	ptrs []unsafe.Pointer
}

func (a *cAllocator) CString(s string) *C.char {
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

func buildCDocuments(a *cAllocator, docs []orderedjson.Object) (**C.char, C.size_t) {
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
		// Explicitly nil pointer fields
		arr[i].text = nil
		arr[i].thinking = nil
		arr[i].image = nil
		arr[i].document_json = nil
		arr[i].multipart = nil
		arr[i].multipart_len = 0

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
		if c.Type == ContentDocument {
			arr[i].document_json = jsonCString(a, c.Document)
		}
		// multipart (optional)
		if c.Type == ContentMultipart {
			multipartPtr, multipartLen := buildCContents(a, c.Multipart)
			arr[i].multipart = multipartPtr
			arr[i].multipart_len = multipartLen
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
		// Explicitly nil pointer fields
		arr[i].id = nil
		arr[i].name = nil
		arr[i].parameters = nil

		arr[i].id = a.CString(tc.ID)
		arr[i].name = a.CString(tc.Name)
		arr[i].parameters = a.CString(tc.Parameters)
	}
	return base, C.size_t(n)
}

func buildCSources(a *cAllocator, sources []Source) (*C.CSource, C.size_t) {
	if len(sources) == 0 {
		return nil, 0
	}
	n := len(sources)
	var sample C.CSource
	size := uintptr(n) * unsafe.Sizeof(sample)
	base := (*C.CSource)(a.Malloc(size))
	var arr []C.CSource = unsafe.Slice(base, n)
	for i := 0; i < n; i++ {
		source := sources[i]
		arr[i].tool_call_index = C.size_t(source.ToolCallIndex)

		lenResIdxs := len(source.ToolResultIndices)
		if lenResIdxs > 0 {
			// Can't use a.Malloc here for some reason, got this line from North
			ssize := uintptr(lenResIdxs) * unsafe.Sizeof(C.size_t(0))
			baseResIdxs := (*C.size_t)(a.Malloc(ssize))
			var cResIdxs []C.size_t = unsafe.Slice(baseResIdxs, lenResIdxs)
			// Copy tool res indicies array data
			for i, v := range source.ToolResultIndices {
				cResIdxs[i] = C.size_t(v)
			}
			arr[i].tool_result_indices = baseResIdxs
			arr[i].tool_result_indices_len = C.size_t(lenResIdxs)
		} else {
			arr[i].tool_result_indices = nil
			arr[i].tool_result_indices_len = 0
		}
	}
	return base, C.size_t(n)
}

func buildCCitations(a *cAllocator, citations []FilterCitation) (*C.CFilterCitation, C.size_t) {
	if len(citations) == 0 {
		return nil, 0
	}
	n := len(citations)
	var sample C.CFilterCitation
	size := uintptr(n) * unsafe.Sizeof(sample)
	base := (*C.CFilterCitation)(a.Malloc(size))
	var arr []C.CFilterCitation = unsafe.Slice(base, n)
	for i := 0; i < n; i++ {
		cit := citations[i]
		arr[i].start_index = C.size_t(cit.StartIndex)
		arr[i].end_index = C.size_t(cit.EndIndex)
		arr[i].text = a.CString(cit.Text)
		arr[i].sources, arr[i].sources_len = buildCSources(a, cit.Sources)
		arr[i].is_thinking = C.bool(cit.IsThinking)
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

		// Explicitly nil pointer fields
		arr[i].content = nil
		arr[i].tool_calls = nil
		arr[i].tool_call_id = nil

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

		cCitations, cCitationsLen := buildCCitations(a, m.Citations)
		arr[i].citations = cCitations
		arr[i].citations_len = cCitationsLen
	}
	return base, C.size_t(n)
}

// RenderCMD3 renders CMD3 using the Rust templating engine via FFI.
func RenderCMD3(opts RenderCmd3Options) (string, error) {
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
	additionalFields := jsonCString(&a, opts.AdditionalTemplateFields)
	escapedTokens := jsonCString(&a, opts.EscapedSpecialTokens)

	// Build options struct (lives on Go stack; nested buffers are C-allocated)
	cOpts := C.CRenderCmd3Options{
		messages:                        cMsgs,
		messages_len:                    cMsgsLen,
		template:                        a.CString(opts.Template),
		template_jinja:                  a.CString(opts.TemplateJinja),
		use_jinja:                       C.bool(opts.UseJinja),
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
		json_mode:                       C.bool(opts.JSONMode),
		additional_template_fields_json: additionalFields,
		escaped_special_tokens_json:     escapedTokens,
	}

	if opts.TemplateID != nil {
		cOpts.template_id = a.CString(*opts.TemplateID)
	}
	if opts.DevInstruction != nil {
		cOpts.dev_instruction = a.CString(*opts.DevInstruction)
	}
	if opts.ResponsePrefix != nil {
		cOpts.response_prefix = a.CString(*opts.ResponsePrefix)
	}
	if opts.JSONSchema != nil {
		cOpts.json_schema = a.CString(*opts.JSONSchema)
	}

	// Call into Rust
	res := C.melody_render_cmd3(&cOpts)
	if res == nil {
		return "", errors.New("melody_render_cmd3 returned null result struct")
	}
	defer C.melody_render_result_free(res)

	if res.result != nil {
		return C.GoString(res.result), nil
	}
	if res.error != nil {
		return "", errors.New(C.GoString(res.error))
	}
	return "", errors.New("melody_render_cmd3 returned neither result nor error")
}

// buildCCmd4Options marshals RenderCmd4Options-shaped options into the C-side
// CRenderCmd4Options struct (also used for CMD5 since it shares the layout).
// Allocations are tracked in the provided allocator and freed by the caller.
func buildCCmd4Options(a *cAllocator, opts RenderCmd4Options) C.CRenderCmd4Options {
	cMsgs, cMsgsLen := buildCMessages(a, opts.Messages)
	cDocs, cDocsLen := buildCDocuments(a, opts.Documents)
	cTools, cToolsLen := buildCTools(a, opts.AvailableTools)

	var cGround C.CGrounding
	var hasGround C.bool
	if opts.Grounding != nil {
		cGround = groundingToC(*opts.Grounding)
		hasGround = C.bool(true)
	}

	var cReason C.CReasoningType
	var hasReason C.bool
	if opts.ReasoningType != nil {
		cReason = reasoningTypeToC(*opts.ReasoningType)
		hasReason = C.bool(true)
	}

	additionalFields := jsonCString(a, opts.AdditionalTemplateFields)
	escapedTokens := jsonCString(a, opts.EscapedSpecialTokens)

	cOpts := C.CRenderCmd4Options{
		messages:                        cMsgs,
		messages_len:                    cMsgsLen,
		template:                        a.CString(opts.Template),
		template_jinja:                  a.CString(opts.TemplateJinja),
		use_jinja:                       C.bool(opts.UseJinja),
		documents_json:                  cDocs,
		documents_len:                   cDocsLen,
		available_tools:                 cTools,
		available_tools_len:             cToolsLen,
		grounding:                       cGround,
		has_grounding:                   hasGround,
		reasoning_type:                  cReason,
		has_reasoning_type:              hasReason,
		json_mode:                       C.bool(opts.JSONMode),
		additional_template_fields_json: additionalFields,
		escaped_special_tokens_json:     escapedTokens,
	}

	if opts.TemplateID != nil {
		cOpts.template_id = a.CString(*opts.TemplateID)
	}
	if opts.DevInstruction != nil {
		cOpts.dev_instruction = a.CString(*opts.DevInstruction)
	}
	if opts.PlatformInstruction != nil {
		cOpts.platform_instruction = a.CString(*opts.PlatformInstruction)
	}
	if opts.ResponsePrefix != nil {
		cOpts.response_prefix = a.CString(*opts.ResponsePrefix)
	}
	if opts.JSONSchema != nil {
		cOpts.json_schema = a.CString(*opts.JSONSchema)
	}

	return cOpts
}

// RenderCMD4 renders CMD4 using the Rust templating engine via FFI.
func RenderCMD4(opts RenderCmd4Options) (string, error) {
	var a cAllocator
	defer a.FreeAll()

	cOpts := buildCCmd4Options(&a, opts)

	res := C.melody_render_cmd4(&cOpts)
	if res == nil {
		return "", errors.New("melody_render_cmd4 returned null result struct")
	}
	defer C.melody_render_result_free(res)

	if res.result != nil {
		return C.GoString(res.result), nil
	}
	if res.error != nil {
		return "", errors.New(C.GoString(res.error))
	}
	return "", errors.New("melody_render_cmd4 returned neither result nor error")
}

// RenderCMD5 renders CMD5 using the Rust templating engine via FFI.
//
// CMD5 reuses the CMD4 option schema; the only behavioral difference is the
// underlying jinja template selected on the Rust side.
func RenderCMD5(opts RenderCmd5Options) (string, error) {
	var a cAllocator
	defer a.FreeAll()

	cOpts := buildCCmd4Options(&a, opts)

	res := C.melody_render_cmd5(&cOpts)
	if res == nil {
		return "", errors.New("melody_render_cmd5 returned null result struct")
	}
	defer C.melody_render_result_free(res)

	if res.result != nil {
		return C.GoString(res.result), nil
	}
	if res.error != nil {
		return "", errors.New(C.GoString(res.error))
	}
	return "", errors.New("melody_render_cmd5 returned neither result nor error")
}
