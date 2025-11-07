#include <stdbool.h>
#include <stdint.h>

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
    bool is_tools_reason;
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
extern void melody_filter_options_stream_processed_params(CFilterOptions* options);
extern void melody_filter_options_with_left_trimmed(CFilterOptions* options);
extern void melody_filter_options_with_right_trimmed(CFilterOptions* options);
extern void melody_filter_options_with_prefix_trim(CFilterOptions* options, const char* prefix);
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
