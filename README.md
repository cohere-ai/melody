# melody
> [!WARNING]
> This library is currently in development and the interfaces are subject to change. Be sure to pin this dependency to a specific version.

Templating rendering and generation parsing for Cohere models.

# Parsing

## Command 3
### Synchronous
```Go
import "github.com/cohere-ai/melody/parsing"
text := "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>"
f := parsing.NewFilter(logger, tokenizer, parsing.HandleMultiHopCmd3(), parsing.StreamToolActions())
out := []FilterOutput{}
for _, token := range(tokenizer.Encode(text)) {
	o, err := f.Write(token, nil)
	out = append(out, o...)
}
out = append(out, f.FlushPartials()...)
/*
[]FilterOutput{
    {IsToolsReason: true, Text: "This", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4184}}},
    {IsToolsReason: true, Text: " is", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1801}}},
    {IsToolsReason: true, Text: " a", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1671}}},
    {IsToolsReason: true, Text: " rainbow", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{84470}}},
    {IsToolsReason: true, Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}}},
    {IsToolsReason: true, Text: "emoji", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{104150}}},
    {IsToolsReason: true, Text: ":", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{33}}},
    {IsToolsReason: true, Text: " 🌈", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{11254, 242, 238}}},
    {IsToolsReason: true, Citations: []FilterCitation{{
        StartIndex: 18,
        EndIndex:   26,
        Text:       "emoji: 🌈",
        Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1}}},
        IsThinking: true,
    }}},
    {Text: "foo", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{15579}}},
    {Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}}},
    {Text: "bar", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4962}}},
    {Citations: []FilterCitation{{
        StartIndex: 4,
        EndIndex:   7,
        Text:       "bar",
        Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1, 2}}, {ToolCallIndex: 1, ToolResultIndices: []int{3, 4}}},
        IsThinking: false,
    }}},
},
 */
```
### Asynchronously
```Go
import "github.com/cohere-ai/melody/parsing"
text := "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>"
f := parsing.NewStreamFilter(logger, tokenizer, parsing.HandleMultiHopCmd3(), parsing.StreamToolActions())
var wg sync.WaitGroup
defer wg.Wait()
wg.Go(func(){
	defer f.Close()
    for _, token := range(tokenizer.Encode(text)) {
		err := f.Write(token, nil)
    }
})
out := []FilterOutput{}
for o := range f.Read() {
	out = append(out, o...)
} // FlushPartials called internally when f is closed

/*
[]FilterOutput{
    {IsToolsReason: true, Text: "This", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4184}}},
    {IsToolsReason: true, Text: " is", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1801}}},
    {IsToolsReason: true, Text: " a", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{1671}}},
    {IsToolsReason: true, Text: " rainbow", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{84470}}},
    {IsToolsReason: true, Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}}},
    {IsToolsReason: true, Text: "emoji", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{104150}}},
    {IsToolsReason: true, Text: ":", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{33}}},
    {IsToolsReason: true, Text: " 🌈", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{11254, 242, 238}}},
    {IsToolsReason: true, Citations: []FilterCitation{{
        StartIndex: 18,
        EndIndex:   26,
        Text:       "emoji: 🌈",
        Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1}}},
        IsThinking: true,
    }}},
    {Text: "foo", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{15579}}},
    {Text: " ", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{37}}},
    {Text: "bar", Logprobs: TokenIDsWithLogProb{TokenIDs: []int64{4962}}},
    {Citations: []FilterCitation{{
        StartIndex: 4,
        EndIndex:   7,
        Text:       "bar",
        Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1, 2}}, {ToolCallIndex: 1, ToolResultIndices: []int{3, 4}}},
        IsThinking: false,
    }}},
},
 */
```


### With Raw Text
```Go
import "github.com/cohere-ai/melody/parsing"
textChunks := []string{
    "<|START_THINKING|>", "This", " is", " a", " rainbow", " <", "co", ">", "emoji", ":", " 🌈",
    "</", "co", ":", " ", "0", ":[", "1", "]>", "<|END_THINKING|>", "\n", "<|START_RESPONSE|>",
    "foo", " <", "co", ">", "bar", "</", "co", ":", " ", "0", ":[", "1", ",", "2", "],", "1",
    ":[", "3", ",", "4", "]>", "<|END_RESPONSE|>"
}
f := parsing.NewFilter(logger, nil, parsing.HandleMultiHopCmd3(), parsing.StreamToolActions())
var wg sync.WaitGroup
defer wg.Wait()
wg.Go(func(){
    defer f.Close()
    for _, chunk := range(textChunks) {
        f.WriteDecoded(token, nil)
    }
})
out := []FilterOutput{}
for o := range f.Read() {
    out = append(out, o...)
} // FlushPartials called internally when f is closed

/*
[]FilterOutput{
    {IsToolsReason: true, Text: "This"},
    {IsToolsReason: true, Text: " is"},
    {IsToolsReason: true, Text: " a"},
    {IsToolsReason: true, Text: " rainbow"},
    {IsToolsReason: true, Text: " "},
    {IsToolsReason: true, Text: "emoji"},
    {IsToolsReason: true, Text: ":"},
    {IsToolsReason: true, Text: " 🌈"},
    {IsToolsReason: true, Citations: []FilterCitation{{
        StartIndex: 18,
        EndIndex:   26,
        Text:       "emoji: 🌈",
        Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1}}},
        IsThinking: true,
    }}},
    {Text: "foo"},
    {Text: " "},
    {Text: "bar"},
    {Citations: []FilterCitation{{
        StartIndex: 4,
        EndIndex:   7,
        Text:       "bar",
        Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1, 2}}, {ToolCallIndex: 1, ToolResultIndices: []int{3, 4}}},
        IsThinking: false,
    }}},
},
 */
```