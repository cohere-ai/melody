package gobindings

import (
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"testing"

	"github.com/stretchr/testify/require"
)

type templateTest struct {
	name   string
	input  []byte
	output string
}

func readTemplatingTestCases(t *testing.T, version string) []templateTest {
	t.Helper()
	var cases []templateTest
	// Find the root directory (project root)
	_, filename, _, ok := runtime.Caller(1)
	require.True(t, ok)
	curDir := filepath.Dir(filename)
	// Find the tests/templating/<version> directory
	testDir := filepath.Join(curDir, "..", "tests", "templating", version)
	entries, err := os.ReadDir(testDir)
	require.NoError(t, err)
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}
		dir := filepath.Join(testDir, entry.Name())
		inputPath := filepath.Join(dir, "input.json")
		outputPath := filepath.Join(dir, "output.txt")
		input, err1 := os.ReadFile(inputPath)
		output, err2 := os.ReadFile(outputPath)
		require.NoError(t, err1)
		require.NoError(t, err2)
		cases = append(cases, struct {
			name   string
			input  []byte
			output string
		}{
			name:   entry.Name(),
			input:  input,
			output: string(output),
		})
	}
	return cases
}

func TestTemplating_RenderCMD3_DirCases(t *testing.T) {
	t.Parallel()
	cases := readTemplatingTestCases(t, "cmd3")
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			var opts RenderCmd3Options
			err := json.Unmarshal(tc.input, &opts)
			require.NoError(t, err)
			got, err := RenderCMD3(opts)
			require.NoError(t, err)
			require.Equal(t, tc.output, got)
		})
	}
}

func TestTemplating_RenderCMD4_DirCases(t *testing.T) {
	t.Parallel()
	cases := readTemplatingTestCases(t, "cmd4")
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			var opts RenderCmd4Options
			err := json.Unmarshal(tc.input, &opts)
			require.NoError(t, err)
			got, err := RenderCMD4(opts)
			require.NoError(t, err)
			require.Equal(t, tc.output, got)
		})
	}
}

func TestTemplating_RenderCMD3_DirCases_Jinja(t *testing.T) {
	t.Parallel()
	cases := readTemplatingTestCases(t, "cmd3")
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			var opts RenderCmd3Options
			err := json.Unmarshal(tc.input, &opts)
			require.NoError(t, err)
			opts.UseJinja = true
			got, err := RenderCMD3(opts)
			require.NoError(t, err)
			require.Equal(t, tc.output, got)
		})
	}
}

func TestTemplating_RenderCMD4_DirCases_Jinja(t *testing.T) {
	t.Parallel()
	cases := readTemplatingTestCases(t, "cmd4")
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			var opts RenderCmd4Options
			err := json.Unmarshal(tc.input, &opts)
			require.NoError(t, err)
			opts.UseJinja = true
			got, err := RenderCMD4(opts)
			require.NoError(t, err)
			require.Equal(t, tc.output, got)
		})
	}
}

func TestTemplating_RenderCMD3Detailed_TopLevelDocs(t *testing.T) {
	t.Parallel()

	input := []byte(`{
		"messages": [
			{"role": "user", "content": [{"type": "text", "text": "Hi"}]}
		],
		"documents": [
			{"id": "doc-a", "title": "A"},
			{"id": "doc-b", "title": "B"},
			{"title": "C-no-id"}
		]
	}`)
	var opts RenderCmd3Options
	require.NoError(t, json.Unmarshal(input, &opts))

	out, err := RenderCMD3Detailed(opts)
	require.NoError(t, err)
	require.NotNil(t, out)
	require.Contains(t, out.Prompt, "Hi", "prompt should render")
	require.Equal(t, []string{""}, out.ToolCallIDs)
	require.Equal(t, [][]string{{"doc-a", "doc-b", ""}}, out.DocumentIDs)
}

func TestTemplating_RenderCMD3Detailed_ToolCallDocs(t *testing.T) {
	t.Parallel()

	input := []byte(`{
		"messages": [
			{"role": "user", "content": [{"type": "text", "text": "Search"}]},
			{
				"role": "chatbot",
				"content": [],
				"tool_calls": [
					{"id": "call_1", "name": "search", "parameters": "{}"},
					{"id": "call_2", "name": "search", "parameters": "{}"}
				]
			},
			{
				"role": "tool",
				"tool_call_id": "call_1",
				"content": [
					{"type": "document", "document": {"id": "res-x", "text": "hit1"}},
					{"type": "document", "document": {"id": "res-y", "text": "hit2"}}
				]
			},
			{
				"role": "tool",
				"tool_call_id": "call_2",
				"content": [
					{"type": "document", "document": {"id": "res-z", "text": "hit3"}}
				]
			}
		]
	}`)
	var opts RenderCmd3Options
	require.NoError(t, json.Unmarshal(input, &opts))

	out, err := RenderCMD3Detailed(opts)
	require.NoError(t, err)
	require.NotNil(t, out)
	require.Equal(t, []string{"call_1", "call_2"}, out.ToolCallIDs)
	require.Equal(t, [][]string{{"res-x", "res-y"}, {"res-z"}}, out.DocumentIDs)
}

func TestTemplating_RenderCMD3Detailed_RoundTripThroughParser(t *testing.T) {
	t.Parallel()

	input := []byte(`{
		"messages": [],
		"documents": [
			{"id": "doc-a"},
			{"id": "doc-b"},
			{"id": "doc-c"}
		]
	}`)
	var opts RenderCmd3Options
	require.NoError(t, json.Unmarshal(input, &opts))

	out, err := RenderCMD3Detailed(opts)
	require.NoError(t, err)
	require.NotNil(t, out)
	require.Equal(t, [][]string{{"doc-a", "doc-b", "doc-c"}}, out.DocumentIDs)

	// Feed the returned lookup table into the parser.
	f := NewFilter(HandleMultiHopCmd3(), WithDocumentIDs(out.DocumentIDs))
	require.NotNil(t, f)
	result, err := f.WriteDecoded(
		"<|START_RESPONSE|>foo <co>bar</co: 0:[0,2]><|END_RESPONSE|>",
	)
	require.NoError(t, err)
	require.NotNil(t, result)
	require.Len(t, result.Citations, 1)
	src := result.Citations[0].Sources[0]
	require.Equal(t, uint(0), src.ToolCallIndex)
	require.Equal(t, []uint{0, 2}, src.ToolResultIndices)
	require.Equal(t, []string{"doc-a", "doc-c"}, src.DocumentIDs)
}

func TestTemplating_RenderCMD4Detailed_TopLevelDocs(t *testing.T) {
	t.Parallel()

	input := []byte(`{
		"messages": [
			{"role": "user", "content": [{"type": "text", "text": "Hi"}]}
		],
		"documents": [
			{"id": "doc-a"},
			{"id": "doc-b"}
		]
	}`)
	var opts RenderCmd4Options
	require.NoError(t, json.Unmarshal(input, &opts))

	out, err := RenderCMD4Detailed(opts)
	require.NoError(t, err)
	require.NotNil(t, out)
	require.Contains(t, out.Prompt, "Hi")
	require.Equal(t, []string{""}, out.ToolCallIDs)
	require.Equal(t, [][]string{{"doc-a", "doc-b"}}, out.DocumentIDs)
}
