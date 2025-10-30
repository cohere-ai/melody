#include <stdbool.h>
#include <stdint.h>
typedef struct CFilter CFilter;

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
    bool is_tools_reason;
} CFilterOutput;

typedef struct {
    CFilterOutput* outputs;
    size_t len;
} CFilterOutputArray;

extern CFilter* melody_filter_new();

extern CFilter* melody_filter_new_multi_hop_cmd3(bool stream_tool_actions);

extern CFilter* melody_filter_new_multi_hop_cmd4(bool stream_tool_actions);

extern CFilter* melody_filter_new_rag();

extern void melody_filter_free(CFilter* filter);

extern CFilterOutputArray* melody_filter_write_decoded(CFilter* filter, const char* decoded_token, const uint32_t* token_ids, size_t token_ids_len, const float* logprobs, size_t logprobs_len);

extern CFilterOutputArray* melody_filter_flush_partials(CFilter* filter);

extern void melody_filter_output_array_free(CFilterOutputArray* arr);