#include <stdbool.h>
#include <stdint.h>

// ============================================================================
// Templating enums and C-compatible types (mirror ffi.rs)
// ============================================================================

typedef enum {
    CRole_Unknown = 0,
    CRole_System = 1,
    CRole_User = 2,
    CRole_Chatbot = 3,
    CRole_Tool = 4,
} CRole;

typedef enum {
    CContentType_Unknown = 0,
    CContentType_Text = 1,
    CContentType_Thinking = 2,
    CContentType_Image = 3,
    CContentType_Document = 4,
} CContentType;

typedef enum {
    CCitationQuality_Unknown = 0,
    CCitationQuality_Off = 1,
    CCitationQuality_On = 2,
} CCitationQuality;

typedef enum {
    CGrounding_Unknown = 0,
    CGrounding_Enabled = 1,
    CGrounding_Disabled = 2,
} CGrounding;

typedef enum {
    CSafetyMode_Unknown = 0,
    CSafetyMode_None = 1,
    CSafetyMode_Strict = 2,
    CSafetyMode_Contextual = 3,
} CSafetyMode;

typedef enum {
    CReasoningType_Unknown = 0,
    CReasoningType_Enabled = 1,
    CReasoningType_Disabled = 2,
} CReasoningType;

typedef struct {
    const char* name;
    const char* description;
    const char* parameters_json; // JSON string representing Map<String, Value>
} CTool;

typedef struct {
    const char* template_placeholder;
} CImage;

typedef struct {
    CContentType content_type;
    const char* text;
    const char* thinking;
    const CImage* image;          // null if None
    const char* document_json;    // null if None; JSON Map<String, Value>
} CContent;

typedef struct {
    const char* id;
    const char* name;
    const char* parameters;
} CToolCall;

typedef struct {
    CRole role;
    const CContent* content;
    size_t content_len;
    const CToolCall* tool_calls;
    size_t tool_calls_len;
    const char* tool_call_id; // null if None
} CMessage;

typedef struct {
    const CMessage* messages;
    size_t messages_len;
    const char* template;
    const char* dev_instruction;
    const char* const* documents_json;
    size_t documents_len;
    const CTool* available_tools;
    size_t available_tools_len;
    CSafetyMode safety_mode;
    bool has_safety_mode;
    CCitationQuality citation_quality;
    bool has_citation_quality;
    CReasoningType reasoning_type;
    bool has_reasoning_type;
    bool skip_preamble;
    const char* response_prefix;
    const char* json_schema;
    bool json_mode;
    const char* additional_template_fields_json; // JSON BTreeMap<String, Value>
    const char* escaped_special_tokens_json;     // JSON BTreeMap<String, String>
} CRenderCmd3Options;

typedef struct {
    const CMessage* messages;
    size_t messages_len;
    const char* template;
    const char* dev_instruction;
    const char* platform_instruction;
    const char* const* documents_json;
    size_t documents_len;
    const CTool* available_tools;
    size_t available_tools_len;
    CGrounding grounding;
    bool has_grounding;
    const char* response_prefix;
    const char* json_schema;
    bool json_mode;
    const char* additional_template_fields_json;
    const char* escaped_special_tokens_json;
} CRenderCmd4Options;

// ============================================================================
// Templating FFI functions
// ============================================================================

extern char* melody_render_cmd3(const CRenderCmd3Options* opts);
extern char* melody_render_cmd4(const CRenderCmd4Options* opts);
extern void melody_string_free(char* s);

typedef struct CFilter CFilter;
typedef struct CFilterOptions CFilterOptions;

typedef struct {
    size_t tool_call_index;
    size_t* tool_result_indices;
    size_t tool_result_indices_len;
} CSource;

typedef struct {
    size_t start_index;
    size_t end_index;
    char* text;
    CSource* sources;
    size_t sources_len;
    bool is_thinking;
} CFilterCitation;

typedef struct {
    char* text;
    size_t text_len;
    uint32_t* token_ids;
    size_t token_ids_len;
    float* logprobs;
    size_t logprobs_len;
    int32_t search_query_index;
    char* search_query_text;
    CFilterCitation* citations;
    size_t citations_len;
    int32_t tool_call_index;
    char* tool_call_id;
    char* tool_call_name;
    char* tool_call_param_name;
    char* tool_call_param_value_delta;
    char* tool_call_raw_param_delta;
    bool is_post_answer;
    bool is_reasoning;
} CFilterOutput;

typedef struct {
    CFilterOutput* outputs;
    size_t len;
} CFilterOutputArray;

// FilterOptions functions
extern CFilterOptions* melody_filter_options_new();
extern void melody_filter_options_free(CFilterOptions* options);
extern void melody_filter_options_cmd3(CFilterOptions* options);
extern void melody_filter_options_cmd4(CFilterOptions* options);
extern void melody_filter_options_handle_rag(CFilterOptions* options);
extern void melody_filter_options_handle_search_query(CFilterOptions* options);
extern void melody_filter_options_handle_multi_hop(CFilterOptions* options);
extern void melody_filter_options_stream_non_grounded_answer(CFilterOptions* options);
extern void melody_filter_options_stream_tool_actions(CFilterOptions* options);
extern void melody_filter_options_stream_processed_params(CFilterOptions* options);
extern void melody_filter_options_with_left_trimmed(CFilterOptions* options);
extern void melody_filter_options_with_right_trimmed(CFilterOptions* options);
extern void melody_filter_options_with_chunk_size(CFilterOptions* options, size_t size);
extern void melody_filter_options_with_inclusive_stops(CFilterOptions* options, const char** stops, size_t stops_len);
extern void melody_filter_options_with_exclusive_stops(CFilterOptions* options, const char** stops, size_t stops_len);
extern void melody_filter_options_remove_token(CFilterOptions* options, const char* token);

// Filter functions
extern CFilter* melody_filter_new(const CFilterOptions* options);
extern void melody_filter_free(CFilter* filter);
extern CFilterOutputArray* melody_filter_write_decoded(CFilter* filter, const char* decoded_token, const uint32_t* token_ids, size_t token_ids_len, const float* logprobs, size_t logprobs_len);
extern CFilterOutputArray* melody_filter_flush_partials(CFilter* filter);
extern void melody_filter_output_array_free(CFilterOutputArray* arr);
