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
    CContentType_Multipart = 5,
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

typedef struct CContent CContent;

struct CContent {
    CContentType content_type;
    const char* text;
    const char* thinking;
    const CImage* image;          // null if None
    const char* document_json;    // null if None; JSON Map<String, Value>
    const CContent* multipart;    // null if None; array of content items
    size_t multipart_len;         // number of multipart items
};

typedef struct {
    const char* id;
    const char* name;
    const char* parameters;
} CToolCall;

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
    CRole role;
    const CContent* content;
    size_t content_len;
    const CToolCall* tool_calls;
    size_t tool_calls_len;
    const char* tool_call_id; // null if None
    const CFilterCitation* citations;
    size_t citations_len;
} CMessage;

typedef struct {
    const CMessage* messages;
    size_t messages_len;
    const char* template_id;
    const char* template;
    const char* template_jinja;
    bool use_jinja;
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
    const char* template_id;
    const char* template;
    const char* template_jinja;
    bool use_jinja;
    const char* dev_instruction;
    const char* platform_instruction;
    const char* const* documents_json;
    size_t documents_len;
    const CTool* available_tools;
    size_t available_tools_len;
    CGrounding grounding;
    bool has_grounding;
    CReasoningType reasoning_type;
    bool has_reasoning_type;
    const char* response_prefix;
    const char* json_schema;
    bool json_mode;
    const char* additional_template_fields_json;
    const char* escaped_special_tokens_json;
} CRenderCmd4Options;

// CMD5 uses the same option layout as CMD4; this typedef makes the binding
// intent explicit at call sites without duplicating the struct definition.
typedef CRenderCmd4Options CRenderCmd5Options;

// ============================================================================
// Templating FFI functions
// ============================================================================

typedef struct {
    char* result; // null if error
    char* error;  // null if success
} CRenderResult;

extern CRenderResult* melody_render_cmd3(const CRenderCmd3Options* opts);
extern CRenderResult* melody_render_cmd4(const CRenderCmd4Options* opts);
extern CRenderResult* melody_render_cmd5(const CRenderCmd5Options* opts);
extern void melody_render_result_free(CRenderResult* res);

typedef struct CFilter CFilter;
typedef struct CFilterOptions CFilterOptions;

typedef struct {
    char* name;
    char* value_delta;
} CFilterToolParameter;

typedef struct {
    size_t index;
    char* id;
    char* name;
    char* arguments;
    CFilterToolParameter* processed_params;
    size_t processed_params_len;
} CAccumulatedToolCall;

typedef struct {
    size_t index;
    char* text;
} CSearchQueryDelta;

typedef struct {
    char* content;
    char* reasoning;
    CAccumulatedToolCall* tool_calls;
    size_t tool_calls_len;
    CFilterCitation* citations;
    size_t citations_len;
    CSearchQueryDelta* search_queries;
    size_t search_queries_len;
} CAggregatedResult;

typedef struct {
    CAggregatedResult* result; // null if error
    char* error;               // null if success
} CAggregatedResultResponse;

// FilterOptions functions
extern CFilterOptions* melody_filter_options_new();
extern void melody_filter_options_free(CFilterOptions* options);
extern void melody_filter_options_cmd3(CFilterOptions* options);
extern void melody_filter_options_cmd4(CFilterOptions* options);
extern void melody_filter_options_cmd5(CFilterOptions* options);
extern void melody_filter_options_handle_rag(CFilterOptions* options);
extern void melody_filter_options_handle_search_query(CFilterOptions* options);
extern void melody_filter_options_handle_multi_hop(CFilterOptions* options);
extern void melody_filter_options_stream_non_grounded_answer(CFilterOptions* options);
extern void melody_filter_options_stream_tool_actions(CFilterOptions* options);
extern void melody_filter_options_stream_processed_params(CFilterOptions* options);
extern void melody_filter_options_cofl_no_xml_text_decode(CFilterOptions* options);
extern void melody_filter_options_cofl_nested_xml(CFilterOptions* options);
extern void melody_filter_options_with_left_trimmed(CFilterOptions* options);
extern void melody_filter_options_with_right_trimmed(CFilterOptions* options);
extern void melody_filter_options_with_chunk_size(CFilterOptions* options, size_t size);
extern void melody_filter_options_with_inclusive_stops(CFilterOptions* options, const char** stops, size_t stops_len);
extern void melody_filter_options_with_exclusive_stops(CFilterOptions* options, const char** stops, size_t stops_len);
extern void melody_filter_options_remove_token(CFilterOptions* options, const char* token);

// Filter functions
extern CFilter* melody_filter_new(const CFilterOptions* options);
extern void melody_filter_free(CFilter* filter);
extern CAggregatedResultResponse* melody_filter_write_decoded(CFilter* filter, const char* decoded_token);
extern CAggregatedResultResponse* melody_filter_flush_partials(CFilter* filter);
extern void melody_aggregated_result_free(CAggregatedResultResponse* res);
