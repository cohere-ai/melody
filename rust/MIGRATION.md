# Migration Guide: Go to Rust

This document helps you migrate from the Go implementation to the Rust port of the Melody parsing library.

## Core API Comparison

### Creating a Filter

**Go:**
```go
import "github.com/cohere-ai/melody/parsing"

filter := parsing.NewFilter(logger, tokenizer,
    parsing.WithLeftTrimmed(),
    parsing.WithRightTrimmed(),
)
```

**Rust:**
```rust
use melody_parsing::{FilterImpl, FilterOptions};

let options = FilterOptions::new()
    .with_left_trimmed()
    .with_right_trimmed();

let mut filter = FilterImpl::new(Some(tokenizer));
options.apply_to_filter(&mut filter);
```

### Writing Tokens

**Go:**
```go
outputs, err := filter.Write(token, &likelihood)
if err != nil {
    return err
}
for _, output := range outputs {
    fmt.Println(output.Text)
}
```

**Rust:**
```rust
let outputs = filter.write(token, Some(likelihood))?;
for output in outputs {
    println!("{}", output.text);
}
```

### Writing Decoded Text

**Go:**
```go
outputs := filter.WriteDecoded(decodedToken, logprobs)
```

**Rust:**
```rust
let outputs = filter.write_decoded(decoded_token, logprobs);
```

### Flushing Partials

**Go:**
```go
outputs := filter.FlushPartials()
```

**Rust:**
```rust
let outputs = filter.flush_partials();
```

## Stream Filter

### Go

```go
streamFilter := parsing.NewStreamFilter(logger, tokenizer, opts...)
go func() {
    for output := range streamFilter.Read() {
        fmt.Println(output.Text)
    }
}()

streamFilter.Write(token, &likelihood)
streamFilter.Close()
```

### Rust

```rust
let stream = StreamFilter::new(filter);

let rx = stream.read();
thread::spawn(move || {
    for output in rx {
        println!("{}", output.text);
    }
});

stream.write_decoded("text", logprobs);
stream.close();
```

## Options Comparison

| Go Function | Rust Method | Description |
|------------|-------------|-------------|
| `WithLeftTrimmed()` | `.with_left_trimmed()` | Trim whitespace from start |
| `WithRightTrimmed()` | `.with_right_trimmed()` | Trim whitespace from end |
| `WithPrefixTrim(prefix)` | `.with_prefix_trim(prefix)` | Trim specific prefix |
| `WithInclusiveStops(...)` | `.with_inclusive_stops(vec![...])` | Include stop tokens |
| `WithExclusiveStops(...)` | `.with_exclusive_stops(vec![...])` | Exclude stop tokens |
| `WithChunkSize(size)` | `.with_chunk_size(size)` | Set chunk size |
| `WithRepetitionLimit(limit, len)` | `.with_repetition_limit(limit, len)` | Limit repetition |
| `HandleRag()` | `.handle_rag()` | Handle RAG format |
| `HandleMultiHop()` | `.handle_multi_hop()` | Handle multi-hop |
| `HandleMultiHopCmd3()` | `.handle_multi_hop_cmd3()` | Handle Command R |
| `HandleMultiHopCmd4()` | `.handle_multi_hop_cmd4()` | Handle Command R+ |
| `HandleLlama()` | `.handle_llama()` | Handle LLaMA |
| `StreamNonGroundedAnswer()` | `.stream_non_grounded_answer()` | Stream answers |
| `StreamToolActions()` | `.stream_tool_actions()` | Stream tool calls |
| `StreamProcessedParams()` | `.stream_processed_params()` | Stream parameters |
| `RemoveToken(token)` | `.remove_token(token)` | Remove token |

## Type Mapping

| Go Type | Rust Type | Notes |
|---------|-----------|-------|
| `Decoder` | `Decoder` trait | Must be implemented |
| `Filter` | `Filter` trait | Core interface |
| `*filter` | `FilterImpl<D>` | Implementation |
| `TokenIDsWithLogProb` | `TokenIDsWithLogProb` | Same structure |
| `FilterOutput` | `FilterOutput` | Same structure |
| `FilterCitation` | `FilterCitation` | Same structure |
| `FilterSearchQueryDelta` | `FilterSearchQueryDelta` | Same structure |
| `FilterToolCallDelta` | `FilterToolCallDelta` | Same structure |
| `FilterToolParameter` | `FilterToolParameter` | Same structure |
| `Source` | `Source` | Same structure |

## Field Name Conventions

Rust follows snake_case for field names while Go uses camelCase:

| Go Field | Rust Field |
|----------|------------|
| `TokenIDs` | `token_ids` |
| `Logprobs` | `logprobs` |
| `SearchQuery` | `search_query` |
| `ToolCalls` | `tool_calls` |
| `IsPostAnswer` | `is_post_answer` |
| `IsToolsReason` | `is_tools_reason` |
| `StartIndex` | `start_index` |
| `EndIndex` | `end_index` |
| `ToolCallIndex` | `tool_call_index` |
| `ToolResultIndices` | `tool_result_indices` |

## Error Handling

**Go:**
```go
outputs, err := filter.Write(token, likelihood)
if err != nil {
    return err
}
```

**Rust:**
```rust
let outputs = filter.write(token, likelihood)?;
// or
match filter.write(token, likelihood) {
    Ok(outputs) => { /* process */ },
    Err(e) => { /* handle error */ },
}
```

## Implementing the Decoder Trait

**Go:**
```go
type MyDecoder struct {
    // fields
}

func (d *MyDecoder) Decode(tokens []int64, skipSpecialTokens bool) (string, error) {
    // implementation
}
```

**Rust:**
```rust
struct MyDecoder {
    // fields
}

impl Decoder for MyDecoder {
    fn decode(&self, tokens: &[i64], skip_special_tokens: bool)
        -> Result<String, Box<dyn std::error::Error>> {
        // implementation
    }
}
```

## Testing

**Go:**
```go
import "github.com/stretchr/testify/require"

func TestExample(t *testing.T) {
    filter := newF(nil, nil)
    output, remove := filter.ParseCitations(input, groundedAnswer)
    require.Equal(t, expected, output)
}
```

**Rust:**
```rust
#[test]
fn test_example() {
    let mut filter = FilterImpl::<MockDecoder>::new(None);
    let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);
    assert_eq!(output, expected);
}
```

## Common Patterns

### Option/Nil Handling

**Go:**
```go
if tokenLogProb != nil {
    // use tokenLogProb
}
```

**Rust:**
```rust
if let Some(log_prob) = token_log_prob {
    // use log_prob
}
```

### Slice Operations

**Go:**
```go
text := str[:idx]
remainder := str[idx:]
```

**Rust:**
```rust
let text = &s[..idx];
let remainder = &s[idx..];
```

### String Building

**Go:**
```go
var buf bytes.Buffer
buf.Write(text)
str := buf.String()
```

**Rust:**
```rust
let mut buf = Vec::<u8>::new();
buf.extend_from_slice(text);
let s = String::from_utf8_lossy(&buf);
```

## Performance Considerations

1. **Zero-copy operations**: Rust uses slices (`&str`, `&[u8]`) where possible to avoid copying
2. **Move semantics**: Rust moves values by default; use `.clone()` when needed
3. **Borrowing**: The borrow checker ensures memory safety without garbage collection
4. **Channel performance**: Rust's `mpsc` channels are similar to Go channels but with different performance characteristics

## Best Practices

1. **Use the builder pattern** for options instead of multiple function calls
2. **Prefer `?` operator** for error propagation
3. **Use `if let` and `match`** for Option and Result handling
4. **Implement traits** for custom decoders
5. **Use `cargo clippy`** to catch common mistakes
6. **Run `cargo fmt`** to maintain consistent formatting
