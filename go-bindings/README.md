# Golang Melody

## Parsing

```Go
import "github.com/cohere-ai/melody"

textChunks := []string{
    "<|START_THINKING|>", "This", " is", " a", " rainbow", " <", "co", ">", "emoji", ":", " 🌈",
    "</", "co", ":", " ", "0", ":[", "1", "]>", "<|END_THINKING|>", "\n", "<|START_RESPONSE|>",
    "foo", " <", "co", ">", "bar", "</", "co", ":", " ", "0", ":[", "1", ",", "2", "],", "1",
    ":[", "3", ",", "4", "]>", "<|END_RESPONSE|>"
}

// Create a filter with options using the builder pattern
f := melody.NewFilter(melody.HandleMultiHopCmd3(), melody.StreamToolActions())

// Process tokens synchronously
out := []melody.FilterOutput{}
for _, chunk := range textChunks {
    outputs := f.WriteDecoded(chunk, nil)
    out = append(out, outputs...)
}

// Flush any remaining partial outputs
out = append(out, f.FlushPartials()...)

/*
Expected output:
[]melody.FilterOutput{
    {IsToolsReason: true, Text: "This"},
    {IsToolsReason: true, Text: " is"},
    {IsToolsReason: true, Text: " a"},
    {IsToolsReason: true, Text: " rainbow"},
    {IsToolsReason: true, Text: " "},
    {IsToolsReason: true, Text: "emoji"},
    {IsToolsReason: true, Text: ":"},
    {IsToolsReason: true, Text: " 🌈"},
    {Citations: []melody.FilterCitation{{
        StartIndex: 18,
        EndIndex:   26,
        Text:       "emoji: 🌈",
        Sources:    []melody.Source{{ToolCallIndex: 0, ToolResultIndices: []int{1}}},
        IsThinking: true,
    }}},
    {Text: "foo"},
    {Text: " "},
    {Text: "bar"},
    {Citations: []melody.FilterCitation{{
        StartIndex: 4,
        EndIndex:   7,
        Text:       "bar",
        Sources:    []melody.Source{{ToolCallIndex: 0, ToolResultIndices: []int{1, 2}}, {ToolCallIndex: 1, ToolResultIndices: []int{3, 4}}},
        IsThinking: false,
    }}},
}
*/
```

## Available Options

The filter supports various configuration options:

```Go
// Format handlers
melody.HandleMultiHopCmd3()      // Configure for multi-hop CMD3 format
melody.HandleMultiHopCmd4()      // Configure for multi-hop CMD4 format
melody.HandleRAG()               // Configure for RAG format
melody.HandleSearchQuery()       // Configure for search query format
melody.HandleMultiHop()          // Configure for multi-hop format

// Streaming options
melody.StreamToolActions()       // Enable streaming of tool actions
melody.StreamNonGroundedAnswer() // Enable streaming of non-grounded answer
melody.StreamProcessedParams()   // Enable streaming of processed parameters

// Trimming options
melody.WithLeftTrimmed()         // Enable left trimming
melody.WithRightTrimmed()        // Enable right trimming
melody.WithPrefixTrim("PREFIX:") // Set a prefix to trim

// Size and limit options
melody.WithChunkSize(100)        // Set the chunk size
melody.WithRepetitionLimit(10, 5) // Set repetition limit and max sequence length

// Stop sequences
melody.WithInclusiveStops([]string{"STOP1", "STOP2"}) // Set inclusive stops
melody.WithExclusiveStops([]string{"END1", "END2"})   // Set exclusive stops

// Token removal
melody.RemoveToken("<|END_RESPONSE|>") // Remove specific tokens from output
```

Example with multiple options:

```Go
f := melody.NewFilter(
    melody.HandleMultiHopCmd3(),
    melody.StreamToolActions(),
    melody.WithChunkSize(100),
    melody.WithLeftTrimmed(),
    melody.RemoveToken("<|END_RESPONSE|>"),
)
```
