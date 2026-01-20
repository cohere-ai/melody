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
