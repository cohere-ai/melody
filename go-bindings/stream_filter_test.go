package melody

import (
	"fmt"
	"strings"
	"sync"
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/cohere-ai/melody/_internal/tokenizers"
)

func ptr[T any](val T) *T {
	return &val
}

//	func TestStreamFilter(t *testing.T) {
//		t.Parallel()
//		tests := []struct {
//			name             string
//			input            string
//			inputLogProb     *float32
//			inclusiveStops   []string
//			exclusiveStops   []string
//			leftTrimmed      bool
//			rightTrimmed     bool
//			prefixTrim       string
//			want             string
//			expectedLogProbs []TokenIDsWithLogProb
//			isCmd3           bool
//		}{
//			{
//				name:           "Exclusive stop sequences with one byte at a time",
//				input:          "foo bar baz boo",
//				exclusiveStops: []string{"ba"},
//				want:           "foo ",
//			},
//			{
//				name:           "Exclusive stop sequences with two bytes at a time, misaligned",
//				input:          "foo bar baz boo",
//				exclusiveStops: []string{"ar"},
//				want:           "foo b",
//			},
//			{
//				name:           "Exclusive stop sequences with multi message stop sequence",
//				input:          "foo bar baz boo",
//				exclusiveStops: []string{"ar baz"},
//				want:           "foo b",
//			},
//			{
//				name:           "Exclusive stop sequences - multiple",
//				input:          "foo bar baz boo",
//				exclusiveStops: []string{"ba", "az"},
//				want:           "foo ",
//			},
//			{
//				name:           "Exclusive stop sequences - partial match",
//				input:          "foo bar baz boo",
//				exclusiveStops: []string{"baz"},
//				want:           "foo bar ",
//			},
//			{
//				name:           "Exclusive stop sequences with where stop is longer than intake",
//				input:          "foo bar baz boo",
//				exclusiveStops: []string{"ba"},
//				want:           "foo ",
//			},
//			{
//				name:           "Inclusive stop sequences with one byte at a time",
//				input:          "foo bar baz boo",
//				inclusiveStops: []string{"ba"},
//				want:           "foo ba",
//			},
//			{
//				name:           "Inclusive stop sequences with two bytes at a time, misaligned",
//				input:          "foo bar baz boo",
//				inclusiveStops: []string{"ar"},
//				want:           "foo bar",
//			},
//			{
//				name:           "Inclusive stop sequences with multi message stop sequence",
//				input:          "foo bar baz boo",
//				inclusiveStops: []string{"ar baz"},
//				want:           "foo bar baz",
//			},
//			{
//				name:           "Inclusive stop sequences - multiple",
//				input:          "foo bar baz boo",
//				inclusiveStops: []string{"ba", "az"},
//				want:           "foo ba",
//			},
//			{
//				name:           "Inclusive stop sequences - partial match",
//				input:          "foo bar baz boo",
//				inclusiveStops: []string{"baz"},
//				want:           "foo bar baz",
//			},
//			{
//				name:           "Inclusive stop sequences with where stop is longer than intake",
//				input:          "foo bar baz boo",
//				inclusiveStops: []string{"ba"},
//				want:           "foo ba",
//			},
//			{
//				name:           "Both exclusive and inclusive stop sequences - inclusive first",
//				input:          "foo bar baz boo",
//				exclusiveStops: []string{"r "},
//				inclusiveStops: []string{"ar"},
//				want:           "foo bar",
//			},
//			{
//				name:           "Both exclusive and inclusive stop sequences - inclusive first, far apart",
//				input:          "foo bar baz boo",
//				exclusiveStops: []string{"bo"},
//				inclusiveStops: []string{"ar"},
//				want:           "foo bar",
//			},
//			{
//				name:           "Both exclusive and inclusive stop sequences - exclusive first",
//				input:          "foo bar baz boo",
//				exclusiveStops: []string{"ar"},
//				inclusiveStops: []string{"r "},
//				want:           "foo b",
//			},
//			{
//				name:           "Both exclusive and inclusive stop sequences - exclusive first, far apart",
//				input:          "foo bar baz boo",
//				exclusiveStops: []string{"ar"},
//				inclusiveStops: []string{" b"},
//				want:           "foo b",
//			},
//			{
//				name:           "Unicode",
//				input:          "foo😂bar",
//				exclusiveStops: []string{"bar"},
//				want:           "foo😂",
//			},
//			{
//				name:        "Left trim",
//				input:       " \n \nfoo bar baz boo\n \n \n",
//				leftTrimmed: true,
//				want:        "foo bar baz boo\n \n \n",
//			},
//			{
//				name:         "Right trim",
//				input:        " \n \nfoo bar baz boo",
//				rightTrimmed: true,
//				want:         " \n \nfoo bar baz boo",
//			},
//			{
//				name:         "Both left and right trim",
//				input:        " \n \nfoo bar baz boo\n \n \n",
//				leftTrimmed:  true,
//				rightTrimmed: true,
//				want:         "foo bar baz boo",
//			},
//			{
//				name:         "Both left and right trim - short trim",
//				input:        "\nfoo bar baz boo\n",
//				leftTrimmed:  true,
//				rightTrimmed: true,
//				want:         "foo bar baz boo",
//			},
//			{
//				name:        "Chat prefix and left space trim",
//				input:       "\nConcept: foo bar baz boo\n",
//				leftTrimmed: true,
//				prefixTrim:  "Concept: ",
//				want:        "foo bar baz boo\n",
//			},
//			{
//				name:        "Chat prefix and left space trim, incomplete prefix match",
//				input:       "\nCon: foo bar baz boo\n",
//				leftTrimmed: true,
//				prefixTrim:  "Concept: ",
//				want:        "Con: foo bar baz boo\n",
//			},
//			{
//				name:        "Chat prefix and left space trim, almost full prefix match",
//				input:       "\nConcep: foo bar baz boo\n",
//				leftTrimmed: true,
//				prefixTrim:  "Concept: ",
//				want:        "Concep: foo bar baz boo\n",
//			},
//			{
//				name:        "Chat prefix and left space trim, repeated presence",
//				input:       "\nConcept: foo bar Concept: baz boo\n",
//				leftTrimmed: true,
//				prefixTrim:  "Concept: ",
//				want:        "foo bar Concept: baz boo\n",
//			},
//			{
//				name:        "Chat prefix and left space trim, does not start with prefix",
//				input:       "\nyoConcept: foo bar baz boo\n",
//				leftTrimmed: true,
//				prefixTrim:  "Concept: ",
//				want:        "yoConcept: foo bar baz boo\n",
//			},
//			{
//				name:         "Single token with logprob",
//				input:        "foo",
//				inputLogProb: ptr(float32(0.1)),
//				want:         "foo",
//				expectedLogProbs: []TokenIDsWithLogProb{
//					{
//						TokenIDs: []int64{30430},
//						Logprobs: []float32{0.1},
//					},
//				},
//			},
//			{
//				name:         "Multiple tokens with logprob",
//				input:        "foo fizz",
//				inputLogProb: ptr(float32(0.1)),
//				want:         "foo fizz",
//				expectedLogProbs: []TokenIDsWithLogProb{
//					{
//						TokenIDs: []int64{30430},
//						Logprobs: []float32{0.1},
//					},
//					{
//						TokenIDs: []int64{278},
//						Logprobs: []float32{0.1},
//					},
//					{
//						TokenIDs: []int64{4667},
//						Logprobs: []float32{0.1},
//					},
//				},
//			},
//			{
//				name:         "Cmd3 with logprobs",
//				input:        "foo",
//				inputLogProb: ptr(float32(0.1)),
//				want:         "foo",
//				expectedLogProbs: []TokenIDsWithLogProb{
//					{
//						TokenIDs: []int64{30430},
//						Logprobs: []float32{0.1},
//					},
//				},
//				isCmd3: true,
//			},
//		}
//		for _, tt := range tests {
//			t.Run(tt.name, func(t *testing.T) {
//				t.Parallel()
//				var wg sync.WaitGroup
//				wgErr := make(chan error, 1)
//				defer func() { require.NoError(t, <-wgErr) }()
//				defer close(wgErr)
//				defer wg.Wait()
//
//				tkzr, err := tokenizers.GetTokenizer("50k")
//				require.NoError(t, err)
//
//				opts := []FilterOption{
//					WithInclusiveStops(tt.inclusiveStops...),
//					WithExclusiveStops(tt.exclusiveStops...),
//				}
//				if tt.leftTrimmed {
//					opts = append(opts, WithLeftTrimmed())
//				}
//				if tt.rightTrimmed {
//					opts = append(opts, WithRightTrimmed())
//				}
//				if tt.prefixTrim != "" {
//					opts = append(opts, WithPrefixTrim(tt.prefixTrim))
//				}
//				if tt.isCmd3 {
//					opts = append(opts, HandleMultiHopCmd3())
//				}
//				f := NewStreamFilter(zaptest.NewLogger(t), tkzr, opts...)
//				tokens, err := tkzr.Encode(tt.input)
//				require.NoError(t, err)
//				wg.Go(func() {
//					defer f.Close()
//					for _, token := range tokens {
//						err := f.Write(token, tt.inputLogProb)
//						if err != nil {
//							wgErr <- err
//							return
//						}
//					}
//				})
//				var got strings.Builder
//				var gotProbs []TokenIDsWithLogProb
//				for s := range f.Read() {
//					require.NotEmpty(t, s)
//					got.WriteString(s.Text)
//
//					if tt.inputLogProb != nil {
//						gotProbs = append(gotProbs, s.Logprobs)
//					}
//				}
//				if got.String() != tt.want {
//					t.Errorf("got %q, want %q", got.String(), tt.want)
//				}
//				require.Equal(t, tt.expectedLogProbs, gotProbs)
//			})
//		}
//	}
type MultiHopTestCase struct {
	name                        string
	completion                  string
	expectedText                string
	expectedPlan                string
	expected                    []testGeneratedToolInput
	expectedDeltaConcatenations []string
	expectedCitations           []FilterCitation
	completionTokens            []uint32
	tokenizerID                 string
}

func (tt *MultiHopTestCase) RunTest(t *testing.T, cmd3 bool) {
	t.Helper()
	wgErr := make(chan error, 2)
	defer func() {
		require.NoError(t, <-wgErr)
		require.NoError(t, <-wgErr)
	}()
	defer close(wgErr)
	var wg sync.WaitGroup
	defer wg.Wait()

	tokenizerID := "50k"
	if tt.tokenizerID != "" {
		tokenizerID = tt.tokenizerID
	}
	tkzr, err := tokenizers.GetTokenizer(tokenizerID)
	require.NoError(t, err)

	filterOption := HandleMultiHop()
	if cmd3 {
		filterOption = HandleMultiHopCmd3()
	}
	f := NewStreamFilter(tkzr, filterOption, StreamToolActions(), StreamProcessedParams())
	fraw := NewStreamFilter(tkzr, filterOption, StreamToolActions())
	tokens := tt.completionTokens
	if len(tokens) == 0 {
		tokens, err = tkzr.EncodeUint32(tt.completion)
		require.NoError(t, err)
	}
	wg.Go(func() {
		defer f.Close()
		for _, token := range tokens {
			err := f.Write(int64(token), nil)
			if err != nil {
				wgErr <- err
				return
			}
		}
	})
	wg.Go(func() {
		defer fraw.Close()
		for _, token := range tokens {
			err := fraw.Write(int64(token), nil)
			if err != nil {
				wgErr <- err
				return
			}
		}
	})
	var gotToolCalls []testGeneratedToolInput
	var gotText, gotPlan strings.Builder
	var citations []FilterCitation
	for s := range f.Read() {
		require.NotEmpty(t, s)
		if s.Text != "" {
			if s.IsToolsReason {
				gotPlan.WriteString(s.Text)
			} else {
				gotText.WriteString(s.Text)
			}
		}
		if s.ToolCalls != nil {
			gotToolCalls = MergeToolCalls(gotToolCalls, s.ToolCalls)
		}
		if s.Citations != nil {
			citations = append(citations, s.Citations...)
		}
	}
	require.Equal(t, tt.expectedPlan, gotPlan.String())
	require.Equal(t, tt.expectedText, gotText.String())
	require.Equal(t, tt.expected, gotToolCalls)
	require.Equal(t, tt.expectedCitations, citations)

	var gotToolCallDeltaConcatenations []string
	for s := range fraw.Read() {
		require.NotEmpty(t, s)
		if s.ToolCalls != nil {
			i := s.ToolCalls.Index
			if i >= len(gotToolCallDeltaConcatenations) {
				gotToolCallDeltaConcatenations = append(gotToolCallDeltaConcatenations, s.ToolCalls.RawParamDelta)
			} else {
				gotToolCallDeltaConcatenations[i] += s.ToolCalls.RawParamDelta
			}
		}
	}
	require.Equal(t, tt.expectedDeltaConcatenations, gotToolCallDeltaConcatenations)
}

func Test_ParseMultiHopCompletion(t *testing.T) {
	t.Parallel()

	testCode := strings.Join([]string{
		`import matplotlib.pyplot as plt`,
		``,
		`# Data for the mountains and number of climbers`,
		`data = {'Mount Everest': None}`,
		`# Sort the data by number of climbers`,
		`sorted_data = dict(sorted(data.items(), key=lambda x: x[1], reverse=True))`,
		`# Get the top 10 mountains`,
		`top_10_mountains = list(sorted_data.keys())[:10]`,
		`# Plot the graph`,
		`plt.figure(figsize=(10, 6))`,
		`plt.bar(top_10_mountains, [data[mountain] for mountain in top_10_mountains])`,
		`plt.xlabel('Mountain')`,
		`plt.ylabel('Number of Climbers')`,
		`plt.xticks(rotation=45, ha='right')`,
		`plt.tight_layout()`,
		`plt.savefig('top_ten_mountains_by_climbers.png')`,
	}, "\\n")

	testcases := []MultiHopTestCase{
		{
			name: "no plan or reflection",
			completion: strings.Join([]string{
				"Action: ```json",
				"[",
				"    {",
				`        "tool_name": "internet_search",`,
				`        "parameters": {`,
				`            "query": "query1"`,
				`        }`,
				"    }",
				"]```",
			}, "\n"),
			expectedPlan: "",
			expectedDeltaConcatenations: []string{strings.Join([]string{
				"{",
				`    "query": "query1"`,
				"}"}, "\n")},
			expected: []testGeneratedToolInput{
				{ToolName: "internet_search", Parameters: map[string]string{"query": "\"query1\""}},
			},
		},
		{
			name:         "Streaming emoji in citation",
			expectedPlan: "I will respond to the user.",
			expectedText: " This is the answer 🌈🌈🌈",
			expected: []testGeneratedToolInput{
				{ToolName: "directly_answer", Parameters: map[string]string{}},
			},
			expectedCitations: []FilterCitation{
				{
					Text:       "This is the answer 🌈🌈🌈",
					Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{0}}},
					StartIndex: 1,
					EndIndex:   23,
				},
			},
			expectedDeltaConcatenations: []string{
				"",
			},
			completion: "Plan: I will respond to the user.\nAction: ```json\n[\n    {\n        \"tool_name\": \"directly_answer\",\n        \"parameters\": {}\n    }\n]\n```\nRelevant Documents: None\nCited Documents: None\nGrounded answer: <co: 0>This is the answer 🌈🌈🌈</co: 0>",
		},
		{
			name: "single action - string - plan",
			completion: strings.Join([]string{
				"Plan: plan1",
				"Action: ```json",
				"[",
				"    {",
				`        "tool_name": "internet_search",`,
				`        "parameters": {`,
				`            "query": "query1"`,
				`        }`,
				"    }",
				"]```",
			}, "\n"),
			expectedPlan: "plan1",
			expectedDeltaConcatenations: []string{strings.Join([]string{
				"{",
				`    "query": "query1"`,
				"}"}, "\n")},
			expected: []testGeneratedToolInput{
				{ToolName: "internet_search", Parameters: map[string]string{"query": "\"query1\""}},
			},
		},
		{
			name: "single action - string - reflection",
			completion: strings.Join([]string{
				"Reflection: plan1",
				"Action: ```json",
				"[",
				"    {",
				`        "tool_name": "internet_search",`,
				`        "parameters": {`,
				`            "query": "query1"`,
				`        }`,
				"    }",
				"]```",
			}, "\n"),
			expectedPlan: "plan1",
			expectedDeltaConcatenations: []string{strings.Join([]string{
				"{",
				`    "query": "query1"`,
				"}"}, "\n")},
			expected: []testGeneratedToolInput{
				{ToolName: "internet_search", Parameters: map[string]string{"query": "\"query1\""}},
			},
		},
		{
			name: "single action - integer - plan",
			completion: strings.Join([]string{
				"Plan: plan",
				"Action: ```json",
				"[",
				"    {",
				`        "tool_name": "number_of_sales",`,
				`        "parameters": {`,
				`            "number": 10,`,
				`            "query": "top 10"`,
				`        }`,
				"    }",
				"]```",
			}, "\n"),
			expectedPlan: "plan",
			expectedDeltaConcatenations: []string{strings.Join([]string{
				"{",
				`    "number": 10,`,
				`    "query": "top 10"`,
				"}"}, "\n")},
			expected: []testGeneratedToolInput{
				{ToolName: "number_of_sales", Parameters: map[string]string{"number": "10", "query": "\"top 10\""}},
			},
		},
		{
			name: "single action - edge case with ```",
			completion: strings.Join([]string{
				"Plan: plan",
				"Action: ```json",
				"[",
				"    {",
				`        "tool_name": "internet_search",`,
				`        "parameters": {`,
				`            "query": "` + "```" + `"`,
				`        }`,
				"    }",
				"]```",
			}, "\n"),
			expectedPlan: "plan",
			expectedDeltaConcatenations: []string{strings.Join([]string{
				"{",
				`    "query": "` + "```" + `"`,
				"}"}, "\n")},
			expected: []testGeneratedToolInput{
				{ToolName: "internet_search", Parameters: map[string]string{"query": "\"```\""}},
			},
		},
		{
			name: "multiple actions",
			completion: strings.Join([]string{
				"Reflection: reflection33",
				"Action: ```json",
				"[",
				"    {",
				`        "tool_name": "foo",`,
				`        "parameters": {`,
				`            "query": "query1"`,
				`        }`,
				"    },",
				"    {",
				`        "tool_name": "bar",`,
				`        "parameters": {`,
				`            "query": "query2"`,
				`        }`,
				"    }",
				"]```",
			}, "\n"),
			expectedPlan: "reflection33",
			expectedDeltaConcatenations: []string{
				strings.Join([]string{
					"{",
					`    "query": "query1"`,
					"}"}, "\n"),
				strings.Join([]string{
					"{",
					`    "query": "query2"`,
					"}"}, "\n"),
			},
			expected: []testGeneratedToolInput{
				{ToolName: "foo", Parameters: map[string]string{"query": "\"query1\""}},
				{ToolName: "bar", Parameters: map[string]string{"query": "\"query2\""}},
			},
		},
		{
			name: "python example",
			completion: fmt.Sprintf(strings.Join([]string{
				"Reflection: I found the following mountains, in no particular order:",
				"- Mount Everest",
				"- K2",
				"I will now use Python to plot a graph of the top ten mountains by number of climbers.",
				"Action: ```json",
				"[",
				"    {",
				`        "tool_name": "python_interpreter",`,
				`        "parameters": {`,
				`            "code": "%s"`,
				`        }`,
				"    }",
				"]```",
			}, "\n"), testCode),
			expectedPlan: strings.Join([]string{
				`I found the following mountains, in no particular order:`,
				`- Mount Everest`,
				`- K2`,
				`I will now use Python to plot a graph of the top ten mountains by number of climbers.`,
			}, "\n"),
			expected: []testGeneratedToolInput{
				{ToolName: "python_interpreter", Parameters: map[string]string{
					"code": `"import matplotlib.pyplot as plt\n\n# Data for the mountains and number of climbers\ndata = {'Mount Everest': None}\n# Sort the data by number of climbers\nsorted_data = dict(sorted(data.items(), key=lambda x: x[1], reverse=True))\n# Get the top 10 mountains\ntop_10_mountains = list(sorted_data.keys())[:10]\n# Plot the graph\nplt.figure(figsize=(10, 6))\nplt.bar(top_10_mountains, [data[mountain] for mountain in top_10_mountains])\nplt.xlabel('Mountain')\nplt.ylabel('Number of Climbers')\nplt.xticks(rotation=45, ha='right')\nplt.tight_layout()\nplt.savefig('top_ten_mountains_by_climbers.png')"`},
				},
			},
			expectedDeltaConcatenations: []string{
				strings.Join([]string{
					"{",
					`    "code": "import matplotlib.pyplot as plt\n\n# Data for the mountains and number of climbers\ndata = {'Mount Everest': None}\n# Sort the data by number of climbers\nsorted_data = dict(sorted(data.items(), key=lambda x: x[1], reverse=True))\n# Get the top 10 mountains\ntop_10_mountains = list(sorted_data.keys())[:10]\n# Plot the graph\nplt.figure(figsize=(10, 6))\nplt.bar(top_10_mountains, [data[mountain] for mountain in top_10_mountains])\nplt.xlabel('Mountain')\nplt.ylabel('Number of Climbers')\nplt.xticks(rotation=45, ha='right')\nplt.tight_layout()\nplt.savefig('top_ten_mountains_by_climbers.png')"`,
					"}"}, "\n"),
			},
		},
		{
			name: "reflection, followed by an immediate grounded answer (no action)",
			completion: strings.Join([]string{
				"Reflection: reflection33",
				"Relevant Documents: 0,1",
				"Cited Documents: 0,1",
				"Answer: foo bar",
				"Grounded answer: foo <co: 0,1>bar</co: 0,1>",
			}, "\n"),
			expectedPlan: "reflection33",
			expectedText: "foo bar",
			expectedCitations: []FilterCitation{
				{
					Text:       "bar",
					Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{0, 1}}},
					StartIndex: 4,
					EndIndex:   7,
				},
			},
		},
	}

	for _, tt := range testcases {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			tt.RunTest(t, false)
		})
	}
}

func Test_ParseMultiHopCompletionCmd3(t *testing.T) {
	t.Parallel()

	testcases := []MultiHopTestCase{
		{
			name: "no plan or reflection",
			completion: strings.Join([]string{
				"<|START_ACTION|>",
				"[",
				"    {",
				`		 "tool_call_id": "1",`,
				`        "tool_name": "internet_search",`,
				`        "parameters": {`,
				`            "query": "query1"`,
				`        }`,
				"    }",
				"]<|END_ACTION|>",
			}, "\n"),
			expectedPlan: "",
			expectedDeltaConcatenations: []string{strings.Join([]string{
				"{",
				`    "query": "query1"`,
				"}"}, "\n")},
			expected: []testGeneratedToolInput{
				{ToolCallID: "1", ToolName: "internet_search", Parameters: map[string]string{"query": "\"query1\""}},
			},
		},
		{
			name:         "Citations in plan text and tool call",
			expectedPlan: "Emoji: 🌈🌈🌈",
			expectedText: "",
			expected: []testGeneratedToolInput{
				{ToolCallID: "1", ToolName: "web_research", Parameters: map[string]string{"query": "\"What is a rainbow?\""}},
			},
			expectedCitations: []FilterCitation{
				{
					Text:       "Emoji: 🌈🌈🌈",
					Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1}}},
					StartIndex: 0,
					EndIndex:   10,
					IsThinking: true,
				},
			},
			expectedDeltaConcatenations: []string{`{"query": "What is a rainbow?"}`},
			completion: strings.Join([]string{
				"<|START_THINKING|><co>Emoji: 🌈🌈🌈</co: 0:[1]><|END_THINKING|>",
				`<|START_ACTION|>`,
				`[`,
				`   {"tool_call_id": "1", "tool_name": "web_research", "parameters": {"query": "What is a rainbow?"}}`,
				`]<|END_ACTION|>`,
			}, "\n"),
		},
		{
			name: "single action - string - plan",
			completion: strings.Join([]string{
				"<|START_THINKING|> plan1<|END_THINKING|>",
				"<|START_ACTION|>",
				"[",
				"    {",
				`		 "tool_call_id": "2",`,
				`        "tool_name": "internet_search",`,
				`        "parameters": {`,
				`            "query": "query1"`,
				`        }`,
				"    }",
				"]<|END_ACTION|>",
			}, "\n"),
			expectedPlan: "plan1",
			expectedDeltaConcatenations: []string{strings.Join([]string{
				"{",
				`    "query": "query1"`,
				"}"}, "\n")},
			expected: []testGeneratedToolInput{
				{ToolCallID: "2", ToolName: "internet_search", Parameters: map[string]string{"query": "\"query1\""}},
			},
		},
		{
			name: "multiple actions",
			completion: strings.Join([]string{
				"<|START_THINKING|> reflection33<|END_THINKING|>",
				"<|START_ACTION|>",
				"[",
				"    {",
				`		 "tool_call_id": "1",`,
				`        "tool_name": "foo",`,
				`        "parameters": {`,
				`            "query": "query1"`,
				`        }`,
				"    },",
				"    {",
				`		 "tool_call_id": "2",`,
				`        "tool_name": "bar",`,
				`        "parameters": {`,
				`            "query": "query2"`,
				`        }`,
				"    }",
				"]<|END_ACTION|>",
			}, "\n"),
			expectedPlan: "reflection33",
			expectedDeltaConcatenations: []string{
				strings.Join([]string{
					"{",
					`    "query": "query1"`,
					"}"}, "\n"),
				strings.Join([]string{
					"{",
					`    "query": "query2"`,
					"}"}, "\n"),
			},
			expected: []testGeneratedToolInput{
				{ToolCallID: "1", ToolName: "foo", Parameters: map[string]string{"query": "\"query1\""}},
				{ToolCallID: "2", ToolName: "bar", Parameters: map[string]string{"query": "\"query2\""}},
			},
		},
		{
			name: "citation in response tokens with spaces in citation ids",
			completion: strings.Join([]string{
				"<|START_RESPONSE|>foo <co>bar</co: 0:[1, 2], 1:[3, 4]><|END_RESPONSE|>",
			}, "\n"),
			expectedText: "foo bar",
			expectedCitations: []FilterCitation{
				{
					Text: "bar",
					Sources: []Source{
						{ToolCallIndex: 0, ToolResultIndices: []int{1, 2}},
						{ToolCallIndex: 1, ToolResultIndices: []int{3, 4}},
					},
					StartIndex: 4,
					EndIndex:   7,
				},
			},
		},
		{
			name: "html tags in response",
			completion: strings.Join([]string{
				"<|START_RESPONSE|><completion_A> is nice <rating>5</rating><|END_RESPONSE|>",
			}, "\n"),
			expectedText: "<completion_A> is nice <rating>5</rating>",
		},
		{
			name:         "thinking and response with citations",
			completion:   "<|START_THINKING|>This is a rainbow <co>emoji: 🌈</co: 0:[1]><|END_THINKING|>\n<|START_RESPONSE|>foo <co>bar</co: 0:[1,2],1:[3,4]><|END_RESPONSE|>",
			expectedPlan: "This is a rainbow emoji: 🌈",
			expectedText: "foo bar",
			expectedCitations: []FilterCitation{
				{
					Text:       "emoji: 🌈",
					Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1}}},
					StartIndex: 18,
					EndIndex:   26,
					IsThinking: true,
				},
				{
					Text: "bar",
					Sources: []Source{
						{ToolCallIndex: 0, ToolResultIndices: []int{1, 2}},
						{ToolCallIndex: 1, ToolResultIndices: []int{3, 4}},
					},
					StartIndex: 4,
					EndIndex:   7,
				},
			},
		},
		{
			name: "bad utf8 token test",
			completionTokens: []uint32{ // Tokenized: "<|START_RESPONSE|>foo bar *invalid* <|END_RESPONSE|>"
				255021,
				15579,
				4634,
				260, // This is the token that gets detokenized to an invalid utf-8
				15579,
				4634,
				255022},
			expectedText: "foo bar�foo bar",
			tokenizerID:  "multilingual+255k+bos+eos+sptok+fim+agents3",
		},
		{
			name: "bad utf8 token test end",
			completionTokens: []uint32{ // Tokenized: "<|START_RESPONSE|>foo bar *invalid* <|END_RESPONSE|>"
				255021,
				15579,
				4634,
				15579,
				4634,
				260, // This is the token that gets detokenized to an invalid utf-8
				255022},
			expectedText: "foo barfoo bar�",
			tokenizerID:  "multilingual+255k+bos+eos+sptok+fim+agents3",
		},
	}

	for _, tt := range testcases {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			tt.RunTest(t, true)
		})
	}
}

func MergeToolCalls(got []testGeneratedToolInput, call *FilterToolCallDelta) []testGeneratedToolInput {
	// Append tool name
	i := call.Index
	if len(got) > i {
		// Already seen this tool
		got[i].ToolName += call.Name
	} else {
		// New call
		got = append((got), testGeneratedToolInput{ToolCallID: call.ID, ToolName: call.Name, Parameters: map[string]string{}})
	}
	if call.ParamDelta == nil {
		return got
	}
	// Append tool parameters
	_, ok := got[i].Parameters[call.ParamDelta.Name]
	if !ok {
		got[i].Parameters[call.ParamDelta.Name] = call.ParamDelta.ValueDelta
	} else {
		got[i].Parameters[call.ParamDelta.Name] += call.ParamDelta.ValueDelta
	}
	return got
}

// Used to mimic how we collect this data in the service
type testGeneratedToolInput struct {
	ToolCallID string
	ToolName   string
	Parameters map[string]string
}
