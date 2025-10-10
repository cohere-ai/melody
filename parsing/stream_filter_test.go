package parsing

import (
	"errors"
	"fmt"
	"math/rand"
	"strings"
	"sync"
	"testing"

	"go.uber.org/zap/zaptest"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	"github.com/cohere-ai/melody/_internal/tokenizers"
)

func ptr[T any](val T) *T {
	return &val
}

func TestStreamFilter(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name             string
		input            string
		inputLogProb     *float32
		inclusiveStops   []string
		exclusiveStops   []string
		leftTrimmed      bool
		rightTrimmed     bool
		prefixTrim       string
		want             string
		expectedLogProbs []TokenIDsWithLogProb
		isCmd3           bool
	}{
		{
			name:           "Exclusive stop sequences with one byte at a time",
			input:          "foo bar baz boo",
			exclusiveStops: []string{"ba"},
			want:           "foo ",
		},
		{
			name:           "Exclusive stop sequences with two bytes at a time, misaligned",
			input:          "foo bar baz boo",
			exclusiveStops: []string{"ar"},
			want:           "foo b",
		},
		{
			name:           "Exclusive stop sequences with multi message stop sequence",
			input:          "foo bar baz boo",
			exclusiveStops: []string{"ar baz"},
			want:           "foo b",
		},
		{
			name:           "Exclusive stop sequences - multiple",
			input:          "foo bar baz boo",
			exclusiveStops: []string{"ba", "az"},
			want:           "foo ",
		},
		{
			name:           "Exclusive stop sequences - partial match",
			input:          "foo bar baz boo",
			exclusiveStops: []string{"baz"},
			want:           "foo bar ",
		},
		{
			name:           "Exclusive stop sequences with where stop is longer than intake",
			input:          "foo bar baz boo",
			exclusiveStops: []string{"ba"},
			want:           "foo ",
		},
		{
			name:           "Inclusive stop sequences with one byte at a time",
			input:          "foo bar baz boo",
			inclusiveStops: []string{"ba"},
			want:           "foo ba",
		},
		{
			name:           "Inclusive stop sequences with two bytes at a time, misaligned",
			input:          "foo bar baz boo",
			inclusiveStops: []string{"ar"},
			want:           "foo bar",
		},
		{
			name:           "Inclusive stop sequences with multi message stop sequence",
			input:          "foo bar baz boo",
			inclusiveStops: []string{"ar baz"},
			want:           "foo bar baz",
		},
		{
			name:           "Inclusive stop sequences - multiple",
			input:          "foo bar baz boo",
			inclusiveStops: []string{"ba", "az"},
			want:           "foo ba",
		},
		{
			name:           "Inclusive stop sequences - partial match",
			input:          "foo bar baz boo",
			inclusiveStops: []string{"baz"},
			want:           "foo bar baz",
		},
		{
			name:           "Inclusive stop sequences with where stop is longer than intake",
			input:          "foo bar baz boo",
			inclusiveStops: []string{"ba"},
			want:           "foo ba",
		},
		{
			name:           "Both exclusive and inclusive stop sequences - inclusive first",
			input:          "foo bar baz boo",
			exclusiveStops: []string{"r "},
			inclusiveStops: []string{"ar"},
			want:           "foo bar",
		},
		{
			name:           "Both exclusive and inclusive stop sequences - inclusive first, far apart",
			input:          "foo bar baz boo",
			exclusiveStops: []string{"bo"},
			inclusiveStops: []string{"ar"},
			want:           "foo bar",
		},
		{
			name:           "Both exclusive and inclusive stop sequences - exclusive first",
			input:          "foo bar baz boo",
			exclusiveStops: []string{"ar"},
			inclusiveStops: []string{"r "},
			want:           "foo b",
		},
		{
			name:           "Both exclusive and inclusive stop sequences - exclusive first, far apart",
			input:          "foo bar baz boo",
			exclusiveStops: []string{"ar"},
			inclusiveStops: []string{" b"},
			want:           "foo b",
		},
		{
			name:           "Unicode",
			input:          "foo😂bar",
			exclusiveStops: []string{"bar"},
			want:           "foo😂",
		},
		{
			name:        "Left trim",
			input:       " \n \nfoo bar baz boo\n \n \n",
			leftTrimmed: true,
			want:        "foo bar baz boo\n \n \n",
		},
		{
			name:         "Right trim",
			input:        " \n \nfoo bar baz boo",
			rightTrimmed: true,
			want:         " \n \nfoo bar baz boo",
		},
		{
			name:         "Both left and right trim",
			input:        " \n \nfoo bar baz boo\n \n \n",
			leftTrimmed:  true,
			rightTrimmed: true,
			want:         "foo bar baz boo",
		},
		{
			name:         "Both left and right trim - short trim",
			input:        "\nfoo bar baz boo\n",
			leftTrimmed:  true,
			rightTrimmed: true,
			want:         "foo bar baz boo",
		},
		{
			name:        "Chat prefix and left space trim",
			input:       "\nConcept: foo bar baz boo\n",
			leftTrimmed: true,
			prefixTrim:  "Concept: ",
			want:        "foo bar baz boo\n",
		},
		{
			name:        "Chat prefix and left space trim, incomplete prefix match",
			input:       "\nCon: foo bar baz boo\n",
			leftTrimmed: true,
			prefixTrim:  "Concept: ",
			want:        "Con: foo bar baz boo\n",
		},
		{
			name:        "Chat prefix and left space trim, almost full prefix match",
			input:       "\nConcep: foo bar baz boo\n",
			leftTrimmed: true,
			prefixTrim:  "Concept: ",
			want:        "Concep: foo bar baz boo\n",
		},
		{
			name:        "Chat prefix and left space trim, repeated presence",
			input:       "\nConcept: foo bar Concept: baz boo\n",
			leftTrimmed: true,
			prefixTrim:  "Concept: ",
			want:        "foo bar Concept: baz boo\n",
		},
		{
			name:        "Chat prefix and left space trim, does not start with prefix",
			input:       "\nyoConcept: foo bar baz boo\n",
			leftTrimmed: true,
			prefixTrim:  "Concept: ",
			want:        "yoConcept: foo bar baz boo\n",
		},
		{
			name:         "Single token with logprob",
			input:        "foo",
			inputLogProb: ptr(float32(0.1)),
			want:         "foo",
			expectedLogProbs: []TokenIDsWithLogProb{
				{
					TokenIDs: []int64{30430},
					Logprobs: []float32{0.1},
				},
			},
		},
		{
			name:         "Multiple tokens with logprob",
			input:        "foo fizz",
			inputLogProb: ptr(float32(0.1)),
			want:         "foo fizz",
			expectedLogProbs: []TokenIDsWithLogProb{
				{
					TokenIDs: []int64{30430},
					Logprobs: []float32{0.1},
				},
				{
					TokenIDs: []int64{278},
					Logprobs: []float32{0.1},
				},
				{
					TokenIDs: []int64{4667},
					Logprobs: []float32{0.1},
				},
			},
		},
		{
			name:         "Cmd3 with logprobs",
			input:        "foo",
			inputLogProb: ptr(float32(0.1)),
			want:         "foo",
			expectedLogProbs: []TokenIDsWithLogProb{
				{
					TokenIDs: []int64{30430},
					Logprobs: []float32{0.1},
				},
			},
			isCmd3: true,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			var wg sync.WaitGroup
			defer wg.Wait()

			tkzr, err := tokenizers.GetTokenizer("50k")
			require.NoError(t, err)

			opts := []FilterOption{
				WithInclusiveStops(tt.inclusiveStops...),
				WithExclusiveStops(tt.exclusiveStops...),
			}
			if tt.leftTrimmed {
				opts = append(opts, WithLeftTrimmed())
			}
			if tt.rightTrimmed {
				opts = append(opts, WithRightTrimmed())
			}
			if tt.prefixTrim != "" {
				opts = append(opts, WithPrefixTrim(tt.prefixTrim))
			}
			if tt.isCmd3 {
				opts = append(opts, HandleMultiHopCmd3())
			}
			f := NewStreamFilter(zaptest.NewLogger(t), tkzr, opts...)
			wg.Add(1)
			go func() {
				defer wg.Done()
				defer f.Close()
				tokens, err := tkzr.Encode(tt.input)
				require.NoError(t, err)
				for _, token := range tokens {
					var err error
					if tt.inputLogProb != nil {
						err = f.Write(token, tt.inputLogProb)
					} else {
						err = f.Write(token, nil)
					}

					require.NoError(t, err)
				}
			}()
			var got string
			var gotProbs []TokenIDsWithLogProb
			for s := range f.Read() {
				assert.NotEmpty(t, s)
				got += s.Text

				if tt.inputLogProb != nil {
					gotProbs = append(gotProbs, s.Logprobs)
				}
			}
			if got != tt.want {
				t.Errorf("got %q, want %q", got, tt.want)
			}
			assert.Equal(t, tt.expectedLogProbs, gotProbs)
		})
	}
}

func TestMessages(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name           string
		tokenizerID    string
		input          string
		inclusiveStops []string
		exclusiveStops []string
		want           []string
	}{
		{
			name:           "Unicode, ends with stop sequence",
			input:          "foo😂bar",
			exclusiveStops: []string{"bar"},
			want:           []string{"foo", "😂"},
		},
		{
			name:           "Unicode, ends with stop sequence",
			input:          "foo😂bar baz",
			exclusiveStops: []string{"baz"},
			want:           []string{"foo", "😂", "bar", " "},
		},
		{
			name:           "Unicode, more tokens before stop sequence",
			input:          "foo😂bar",
			exclusiveStops: []string{"ar"},
			want:           []string{"foo", "😂", "b"},
		},
		{
			name:        "Unicode, with leading space",
			tokenizerID: "75k+bos+eos+eop",
			input:       "my favorite emoji is 🌈",
			want:        []string{"my", " favorite", " emoji", " is", " 🌈"},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			var wg sync.WaitGroup
			defer wg.Wait()
			tokenizer, err := tokenizers.GetTokenizer(tt.tokenizerID)
			require.NoError(t, err)

			f := NewStreamFilter(
				zaptest.NewLogger(t),
				tokenizer,
				WithInclusiveStops(tt.inclusiveStops...),
				WithExclusiveStops(tt.exclusiveStops...),
			)
			wg.Add(1)
			go func() {
				defer wg.Done()
				defer f.Close()
				tokens, err := tokenizer.Encode(tt.input, tokenizers.NoSpecialTokens())
				require.NoError(t, err)
				for _, token := range tokens {
					err := f.Write(token, nil)
					require.NoError(t, err)
				}
			}()
			var got []string
			for s := range f.Read() {
				got = append(got, s.Text)
			}
			assert.ElementsMatch(t, got, tt.want)
		})
	}
}

func TestMessages_SearchQueries(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name        string
		input       string
		wantOutputs []FilterOutput
	}{
		{
			name:  "single search query",
			input: "Search: foo bar baz boo",
			wantOutputs: []FilterOutput{
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: "foo"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " bar"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " baz"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " bo"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: "o"}},
			},
		},
		{
			name:  "multiple search queries",
			input: "Search: foo bar baz boo|||search query 2",
			wantOutputs: []FilterOutput{
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: "foo"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " bar"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " baz"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " bo"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: "o"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: "search"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: " query"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: " 2"}},
			},
		},
		{
			name:  "multiple search queries with white space",
			input: "Search: foo bar baz boo||| search query 2",
			wantOutputs: []FilterOutput{
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: "foo"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " bar"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " baz"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " bo"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: "o"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: "search"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: " query"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: " 2"}},
			},
		},
		{
			name:  "multiple search queries with new line",
			input: "Search: foo bar baz boo|||search query 2\nsearch query 3",
			wantOutputs: []FilterOutput{
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: "foo"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " bar"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " baz"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " bo"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: "o"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: "search"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: " query"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: " 2"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 2, Text: "search"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 2, Text: " query"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 2, Text: " 3"}},
			},
		},
		{
			name:  "multiple search queries with multiple new line",
			input: "Search: foo bar baz boo|||search query 2\n\nsearch query 3",
			wantOutputs: []FilterOutput{
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: "foo"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " bar"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " baz"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: " bo"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 0, Text: "o"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: "search"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: " query"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 1, Text: " 2"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 2, Text: "search"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 2, Text: " query"}},
				{SearchQuery: &FilterSearchQueryDelta{Index: 2, Text: " 3"}},
			},
		},
		// todo handle this
		// {
		//	name: "tool input with search query",
		//	input: strings.Join([]string{
		//		"Action: ```json",
		//		"[",
		//		"    {",
		//		`        "tool_name": "search",`,
		//		`        "parameters": {`,
		//		`            "queries": [`,
		//		`                "search query 1"`,
		//		`            ]`,
		//		`        }`,
		//		"    }",
		//		"]```",
		//	}, "\n"),
		//	wantOutputs: []FilterOutput{
		//	},
		// },
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			var wg sync.WaitGroup
			defer wg.Wait()

			tkzr, err := tokenizers.GetTokenizer("50k")
			require.NoError(t, err)

			f := NewStreamFilter(zaptest.NewLogger(t), tkzr, HandleSearchQuery())
			wg.Add(1)
			go func() {
				defer wg.Done()
				defer f.Close()
				tokens, err := tkzr.Encode(tc.input)
				require.NoError(t, err)
				for _, token := range tokens {
					err := f.Write(token, nil)
					require.NoError(t, err)
				}
			}()
			var got []FilterOutput
			for s := range f.Read() {
				assert.NotEmpty(t, s)
				got = append(got, s)
			}
			assert.Equal(t, tc.wantOutputs, got)
		})
	}
}

func TestMessages_Stops(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name   string
		input  string
		filter []FilterOption
		want   string
	}{
		{
			name:  "Exclusive stop mid word without following word",
			input: "hello",
			filter: []FilterOption{
				WithExclusiveStops("orange"),
			},
			want: "hello",
		},
		{
			name:  "Inclusive stop mid word without following word",
			input: "hello",
			filter: []FilterOption{
				WithInclusiveStops("orange"),
			},
			want: "hello",
		},
		{
			name:  "Exclusive stop mid word with following word",
			input: "hello what",
			filter: []FilterOption{
				WithExclusiveStops("orange"),
			},
			want: "hello what",
		},
		{
			name:  "Inclusive stop mid word with following word",
			input: "hello what",
			filter: []FilterOption{
				WithInclusiveStops("orange"),
			},
			want: "hello what",
		},
		{
			name:  "With actual EOS exclusive stop mid word without following word",
			input: "i<E",
			filter: []FilterOption{
				WithExclusiveStops("<EOS_TOKEN>"),
			},
			want: "i<E",
		},
		{
			name:  "With actual EOS inclusive stop mid word without following word",
			input: "i<E",
			filter: []FilterOption{
				WithInclusiveStops("<EOS_TOKEN>"),
			},
			want: "i<E",
		},
		{
			name:  "With actual EOS exclusive stop mid word with following word",
			input: "i<E what",
			filter: []FilterOption{
				WithExclusiveStops("<EOS_TOKEN>"),
			},
			want: "i<E what",
		},
		{
			name:  "With actual EOS inclusive stop mid word with following word",
			input: "i<E what",
			filter: []FilterOption{
				WithInclusiveStops("<EOS_TOKEN>"),
			},
			want: "i<E what",
		},
		{
			name:  "With actual complete EOS exclusive stop without following word",
			input: "i<EOS_TOKEN>",
			filter: []FilterOption{
				WithExclusiveStops("<EOS_TOKEN>"),
			},
			want: "i",
		},
		{
			name:  "With actual complete EOS inclusive stop without following word",
			input: "i<EOS_TOKEN>",
			filter: []FilterOption{
				WithInclusiveStops("<EOS_TOKEN>"),
			},
			want: "i<EOS_TOKEN>",
		},
		{
			name:  "With actual complete EOS exclusive stop with following word",
			input: "i<EOS_TOKEN> what",
			filter: []FilterOption{
				WithExclusiveStops("<EOS_TOKEN>"),
			},
			want: "i",
		},
		{
			name:  "With actual complete EOS inclusive stop with following word",
			input: "i<EOS_TOKEN> what",
			filter: []FilterOption{
				WithInclusiveStops("<EOS_TOKEN>"),
			},
			want: "i<EOS_TOKEN>",
		},
		{
			name:  "Exclusive stop with empty stop string",
			input: "hello what",
			filter: []FilterOption{
				WithExclusiveStops(""),
			},
			want: "hello what",
		},
		{
			name:  "Inclusive stop with empty stop string",
			input: "hello what",
			filter: []FilterOption{
				WithInclusiveStops(""),
			},
			want: "hello what",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			var wg sync.WaitGroup
			defer wg.Wait()

			tkzr, err := tokenizers.GetTokenizer("50k")
			require.NoError(t, err)

			f := NewStreamFilter(zaptest.NewLogger(t), tkzr, tt.filter...)
			wg.Add(1)
			go func() {
				defer wg.Done()
				defer f.Close()
				tokens, err := tkzr.Encode(tt.input)
				require.NoError(t, err)
				for _, token := range tokens {
					err := f.Write(token, nil)
					require.NoError(t, err)
				}
			}()
			var got string
			for s := range f.Read() {
				assert.NotEmpty(t, s)
				got += s.Text
			}
			assert.Equal(t, tt.want, got)
		})
	}
}

func TestMessages_Citations_Complete(t *testing.T) {
	// Heads up: if the indices are wrong you are probably not removing the right number of chars
	t.Parallel()
	tests := []struct {
		name               string
		input              string
		wantCompleteString string
		wantCitations      []FilterCitation
		wantFilterOutput   []FilterOutput
	}{
		{
			name: "Test with character '’' that is more than one byte",
			input: "Answer: Fiber is a type of carbohydrate that the body can’t digest." +
				"Grounded answer: Fiber is a type of <co: 1>carbohydrate</co: 1> that the body can’t <co: 1,2>digest.</co: 1,2>",
			wantCompleteString: "Fiber is a type of carbohydrate that the body can’t digest.",
			wantCitations: []FilterCitation{
				{
					StartIndex: 19,
					EndIndex:   31,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
					Text:       "carbohydrate",
				},
				{
					StartIndex: 52,
					EndIndex:   59,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1, 2}}},
					Text:       "digest.",
				},
			},
		},
		{
			name: "Two citations, one different document each, with new lines inside answer",
			input: "Relevant Documents: 0\nCited Documents: 0\nAnswer: The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.Grounded answer:" +
				" The tallest penguin is the <co: 0>emperor penguin</co: 0>.\n They are <co: 1>1.2 feet</co: 1> tall.",
			wantCompleteString: "The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.",
			wantCitations: []FilterCitation{
				{
					StartIndex: 27,
					EndIndex:   42,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
					Text:       "emperor penguin",
				},
				{
					StartIndex: 54,
					EndIndex:   62,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
					Text:       "1.2 feet",
				},
			},
		},
		{
			name: "Two citations, one document each",
			input: " Relevant Documents: 0,1\nCited Documents: 1\nAnswer: The tallest penguin is the emperor penguin, which is 1.2 feet tall.\n" +
				"Grounded answer: The tallest penguin is the <co: 1>emperor penguin</co: 1>, which is <co: 1>1.2 feet</co: 1> tall.\n",
			wantCompleteString: "The tallest penguin is the emperor penguin, which is 1.2 feet tall.",
			wantCitations: []FilterCitation{
				{
					StartIndex: 27,
					EndIndex:   42,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
					Text:       "emperor penguin",
				},
				{
					StartIndex: 53,
					EndIndex:   61,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
					Text:       "1.2 feet",
				},
			},
		},
		{
			name:               "no answer",
			input:              "not a grounded generation",
			wantCompleteString: "",
		},
		{
			name:               "empty answer",
			input:              "Grounded answer: ",
			wantCompleteString: "",
		},
		{
			name:               "no citations",
			input:              "Answer: no citations\nGrounded answer: no citations",
			wantCompleteString: "no citations",
		},
		{
			name:               "single citation, start of answer",
			input:              "Answer:foo\nGrounded answer:<co: 0>foo</co: 0>",
			wantCompleteString: "foo",
			wantCitations: []FilterCitation{
				{
					StartIndex: 0,
					EndIndex:   3,
					Text:       "foo",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
			},
			wantFilterOutput: []FilterOutput{
				{
					Text: "foo",
				},
				{
					Text: "\n",
				},
				{
					Citations: []FilterCitation{{StartIndex: 0, EndIndex: 3, Text: "foo", DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}}}},
				},
			},
		},
		{
			name:               "single citation, two documents",
			input:              "Answer: foo\nGrounded answer:<co: 0,1>foo</co: 0,1>",
			wantCompleteString: "foo",
			wantCitations: []FilterCitation{
				{
					StartIndex: 0,
					EndIndex:   3,
					Text:       "foo",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0, 1}}},
				},
			},
		},
		{
			name:               "single citation, multiple documents",
			input:              "Answer: foo\nGrounded answer:<co: 0,1,2,3,4>foo</co: 0,1,2,3,4>",
			wantCompleteString: "foo",
			wantCitations: []FilterCitation{
				{
					StartIndex: 0,
					EndIndex:   3,
					Text:       "foo",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0, 1, 2, 3, 4}}},
				},
			},
		},
		{
			name:               "single citation, not start of answer",
			input:              "Answer: foo bar\nGrounded answer: foo <co: 0>bar</co: 0>",
			wantCompleteString: "foo bar",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
			},
		},
		{
			name:               "single citation, multiple words",
			input:              "Answer: foo bar\nGrounded answer:<co: 0>foo bar</co: 0>",
			wantCompleteString: "foo bar",
			wantCitations: []FilterCitation{
				{
					StartIndex: 0,
					EndIndex:   7,
					Text:       "foo bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
			},
		},
		{
			name:               "multiple citations, separated by whitespace",
			input:              "Answer: foo bar baz\nGrounded answer: foo <co: 0>bar</co: 0> <co: 0>baz</co: 0>",
			wantCompleteString: "foo bar baz",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
				{
					StartIndex: 8,
					EndIndex:   11,
					Text:       "baz",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
			},
		},
		{
			name:               "multiple citations, separated by character",
			input:              "Answer: foo bar-baz\nGrounded answer: foo <co: 0>bar</co: 0>-<co: 0>baz</co: 0>",
			wantCompleteString: "foo bar-baz",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
				{
					StartIndex: 8,
					EndIndex:   11,
					Text:       "baz",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
			},
		},
		{
			name:               "multiple citations, not separated by whitespace",
			input:              "Answer: foo barbaz\nGrounded answer: foo <co: 0>bar</co: 0><co: 0>baz</co: 0>",
			wantCompleteString: "foo barbaz",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
				{
					StartIndex: 7,
					EndIndex:   10,
					Text:       "baz",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
			},
		},
		{
			name:               "multiple citations, with 'Answer:' in output",
			input:              "Answer: foo Answer: bar Answer: baz\nGrounded answer: foo Answer: <co: 0>bar</co: 0> Answer: <co: 0>baz</co: 0>",
			wantCompleteString: "foo Answer: bar Answer: baz",
			wantCitations: []FilterCitation{
				{
					StartIndex: 12,
					EndIndex:   15,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
				{
					StartIndex: 24,
					EndIndex:   27,
					Text:       "baz",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
			},
		},
		{
			name:               "relevant documents",
			input:              "Relevant Documents: 0,1,3,4,5",
			wantCompleteString: "",
		},
		{
			name:               "cited documents",
			input:              "Cited Documents: 0,1,3,4,5",
			wantCompleteString: "",
		},
		{
			name:               "ungrounded answer",
			input:              "Answer: foo",
			wantCompleteString: "foo",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			var wg sync.WaitGroup
			defer wg.Wait()

			tkzr, err := tokenizers.GetTokenizer("50k")
			require.NoError(t, err)

			f := NewStreamFilter(
				zaptest.NewLogger(t),
				tkzr,
				[]FilterOption{
					HandleRag(),
					StreamNonGroundedAnswer(),
					WithLeftTrimmed(),
				}...)
			wg.Add(1)
			go func() {
				defer wg.Done()
				defer f.Close()
				tokens, err := tkzr.Encode(tt.input)
				require.NoError(t, err)
				for _, token := range tokens {
					err := f.Write(token, nil)
					require.NoError(t, err)
				}
			}()
			index := 0
			var got string
			var gotCitations []FilterCitation
			for s := range f.Read() {
				assert.NotEmpty(t, s)
				if !s.IsPostAnswer {
					got += s.Text
					if tt.wantFilterOutput != nil {
						assert.Equal(t, tt.wantFilterOutput[index], s)
					}
					index++
				}
				gotCitations = append(gotCitations, s.Citations...)
			}
			assert.Equal(t, tt.wantCompleteString, got)
			assert.Equal(t, tt.wantCitations, gotCitations)
		})
	}
}

func TestMessages_Citations_DifferentDocumentIndices(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name          string
		input         string
		wantCitations []FilterCitation
	}{
		{
			name:  "single document",
			input: "Grounded answer: foo <co: 0>bar</co: 0>",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
			},
		},
		{
			name:  "single document, trailing comma",
			input: "Grounded answer: foo <co: 0,>bar</co: 0,>",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
			},
		},
		{
			name:  "multiple documents",
			input: "Grounded answer: foo <co: 0,1>bar</co: 0,1>",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0, 1}}},
				},
			},
		},
		{
			name:  "multiple documents, different order",
			input: "Grounded answer: foo <co: 1,0>bar</co: 1,0>",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1, 0}}},
				},
			},
		},
		{
			name:  "no documents",
			input: "Grounded answer: foo <co: >bar</co: >",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
				},
			},
		},
		{
			name:  "not an index",
			input: "Grounded answer: foo <co: foo>bar</co: foo>",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
				},
			},
		},
		{
			name:  "part valid, part not an index",
			input: "Grounded answer: foo <co: foo,0>bar</co: foo,0>",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
			},
		},
		{
			name:  "index large, positive",
			input: "Grounded answer: foo <co: 999>bar</co: 999>",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{999}}},
				},
			},
		},
		{
			name:  "index not found, negative",
			input: "Grounded answer: foo <co: -1>bar</co: -1>",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
				},
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			var wg sync.WaitGroup
			defer wg.Wait()

			tkzr, err := tokenizers.GetTokenizer("50k")
			require.NoError(t, err)

			f := NewStreamFilter(
				zaptest.NewLogger(t),
				tkzr,
				[]FilterOption{
					HandleRag(),
					StreamNonGroundedAnswer(),
				}...)
			wg.Add(1)
			go func() {
				defer wg.Done()
				defer f.Close()
				tokens, err := tkzr.Encode(tt.input)
				require.NoError(t, err)
				for _, token := range tokens {
					err := f.Write(token, nil)
					require.NoError(t, err)
				}
			}()
			var got string
			var gotCitations []FilterCitation
			for s := range f.Read() {
				assert.NotEmpty(t, s)
				got += s.Text
				gotCitations = append(gotCitations, s.Citations...)
			}
			assert.Equal(t, tt.wantCitations, gotCitations)
		})
	}
}

func TestMessages_Citations_BadTag(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name               string
		input              string
		wantCompleteString string
		wantCitations      []FilterCitation
	}{
		{
			name:               "index not found, negative",
			input:              "Grounded answer: foo <co: -1>bar</co: -1>",
			wantCompleteString: "foo bar",
			wantCitations: []FilterCitation{
				{
					StartIndex: 4,
					EndIndex:   7,
					Text:       "bar",
				},
			},
		},
		{
			name:               "part open tag",
			input:              "Grounded answer: foo <",
			wantCompleteString: "foo <",
		},
		{
			name:               "part open co tag, shorter",
			input:              "Grounded answer: foo <c",
			wantCompleteString: "foo <c",
		},
		{
			name:               "part open co tag, longer",
			input:              "Grounded answer: foo <co",
			wantCompleteString: "foo <co",
		},
		{
			name:               "part closed tag",
			input:              "Grounded answer: foo </",
			wantCompleteString: "foo </",
		},
		{
			name:               "part closed co tag, shorter",
			input:              "Grounded answer: foo </c",
			wantCompleteString: "foo </c",
		},
		{
			name:               "part closed co tag, longer",
			input:              "Grounded answer: foo </co",
			wantCompleteString: "foo </co",
		},
		{
			name:               "Relevant documents",
			input:              "Relevant Documents: 0,1,3,4,5",
			wantCompleteString: "",
		},
		{
			name:               "Cited documents",
			input:              "Cited Documents: 0,1,3,4,5",
			wantCompleteString: "",
		},
		{
			name:               "Ungrounded answer",
			input:              "Answer: foo",
			wantCompleteString: "",
		},
		{
			name:               "Grounded answer",
			input:              "Grounded answer: foo",
			wantCompleteString: "foo",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			var wg sync.WaitGroup
			defer wg.Wait()

			tkzr, err := tokenizers.GetTokenizer("50k")
			require.NoError(t, err)

			f := NewStreamFilter(
				zaptest.NewLogger(t),
				tkzr,
				[]FilterOption{
					HandleRag(),
					WithLeftTrimmed(),
				}...)
			wg.Add(1)
			go func() {
				defer wg.Done()
				defer f.Close()
				tokens, err := tkzr.Encode(tt.input)
				require.NoError(t, err)
				for _, token := range tokens {
					err := f.Write(token, nil)
					require.NoError(t, err)
				}
			}()
			var got string
			var gotCitations []FilterCitation
			for s := range f.Read() {
				assert.NotEmpty(t, s)
				got += s.Text
				gotCitations = append(gotCitations, s.Citations...)
			}
			assert.Equal(t, tt.wantCompleteString, got)
			assert.Equal(t, tt.wantCitations, gotCitations)
		})
	}
}

func TestStreamFilter_StopSequencesWithCitations(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name               string
		input              string
		opts               []FilterOption
		wantCompleteString string
		wantCitations      []FilterCitation
	}{
		{
			name:  "exclusive stop with accurate citation, stop before answer",
			input: "Relevant Documents: 0\nCited Documents: 0\nAnswer: The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.Grounded answer: foo bar",
			opts: []FilterOption{
				HandleRag(),
				StreamNonGroundedAnswer(),
				WithExclusiveStops("emperor penguin"),
			},
			wantCompleteString: "The tallest penguin is the",
		},
		{
			name:  "inclusive stop with accurate citation, stop before answer",
			input: "Relevant Documents: 0\nCited Documents: 0\nAnswer: The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.Grounded answer: foo bar",
			opts: []FilterOption{
				HandleRag(),
				StreamNonGroundedAnswer(),
				WithInclusiveStops("emperor penguin"),
			},
			wantCompleteString: "The tallest penguin is the emperor penguin",
		},
		{
			name: "inclusive stop with fast citation, stop after citation",
			input: "Relevant Documents: 0\nCited Documents: 0\nGrounded answer:" +
				" The tallest penguin is the <co: 0>emperor penguin</co: 0>.\n They are <co: 1>1.2 feet</co: 1> tall.",
			opts: []FilterOption{
				HandleRag(),
				WithInclusiveStops("tall."),
			},
			wantCompleteString: " The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.",
			wantCitations: []FilterCitation{
				{
					StartIndex: 28,
					EndIndex:   43,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
					Text:       "emperor penguin",
				},
				{
					StartIndex: 55,
					EndIndex:   63,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
					Text:       "1.2 feet",
				},
			},
		},
		{
			name: "inclusive stop with fast citation, stop in citation",
			input: "Relevant Documents: 0\nCited Documents: 0\nGrounded answer:" +
				" The tallest penguin is the <co: 0>emperor penguin</co: 0>.\n They are <co: 1>1.2 feet</co: 1> tall.",
			opts: []FilterOption{
				HandleRag(),
				WithInclusiveStops("feet"),
			},
			wantCompleteString: " The tallest penguin is the emperor penguin.\n They are 1.2 feet",
			wantCitations: []FilterCitation{
				{
					StartIndex: 28,
					EndIndex:   43,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
					Text:       "emperor penguin",
				},
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			var wg sync.WaitGroup
			defer wg.Wait()

			tkzr, err := tokenizers.GetTokenizer("50k")
			require.NoError(t, err)

			f := NewStreamFilter(zaptest.NewLogger(t), tkzr, tt.opts...)
			wg.Add(1)
			go func() {
				defer wg.Done()
				defer f.Close()
				tokens, err := tkzr.Encode(tt.input)
				require.NoError(t, err)
				for _, token := range tokens {
					err := f.Write(token, nil)
					require.NoError(t, err)
				}
			}()
			var got string
			var gotCitations []FilterCitation
			for s := range f.Read() {
				assert.NotEmpty(t, s)
				got += s.Text
				gotCitations = append(gotCitations, s.Citations...)
			}
			assert.Equal(t, tt.wantCompleteString, got)
			assert.Equal(t, tt.wantCitations, gotCitations)
		})
	}
}

func TestMessages_Citations_Filter(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name                 string
		input                string
		filterOptions        []FilterOption
		wantCompleteString   string
		wantPostAnswerString string
		wantCitations        []FilterCitation
	}{
		{
			name: "For accurate answer",
			input: "Relevant Documents: 0\nCited Documents: 0\nAnswer: The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.Grounded answer:" +
				" The tallest penguin is the <co: 0>emperor penguin</co: 0>.\n They are <co: 1>1.2 feet</co: 1> tall.",
			wantCompleteString:   "The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.",
			wantPostAnswerString: "The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.",
			wantCitations: []FilterCitation{
				{
					StartIndex: 27,
					EndIndex:   42,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
					Text:       "emperor penguin",
				},
				{
					StartIndex: 54,
					EndIndex:   62,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
					Text:       "1.2 feet",
				},
			},
			filterOptions: []FilterOption{
				HandleRag(),
				StreamNonGroundedAnswer(),
				WithLeftTrimmed(),
			},
		},
		{
			name:                 "For answer with 'Answer:' in output",
			input:                "Answer: foo Answer: bar Answer: baz\nGrounded answer: foo Answer: <co: 0>bar</co: 0> Answer: <co: 0>baz</co: 0>",
			wantCompleteString:   "foo Answer: bar Answer: baz",
			wantPostAnswerString: "foo Answer: bar Answer: baz",
			wantCitations: []FilterCitation{
				{
					StartIndex: 12,
					EndIndex:   15,
					Text:       "bar",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
				{
					StartIndex: 24,
					EndIndex:   27,
					Text:       "baz",
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
				},
			},
			filterOptions: []FilterOption{
				HandleRag(),
				StreamNonGroundedAnswer(),
				WithLeftTrimmed(),
			},
		},
		{
			name: "For fast answer",
			input: "Relevant Documents: 0\nCited Documents: 0\nGrounded answer:" +
				" The tallest penguin is the <co: 0>emperor penguin</co: 0>.\n They are <co: 1>1.2 feet</co: 1> tall.",
			wantCompleteString:   "The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.",
			wantPostAnswerString: "",
			wantCitations: []FilterCitation{
				{
					StartIndex: 27,
					EndIndex:   42,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
					Text:       "emperor penguin",
				},
				{
					StartIndex: 54,
					EndIndex:   62,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
					Text:       "1.2 feet",
				},
			},
			filterOptions: []FilterOption{
				HandleRag(),
				WithLeftTrimmed(),
			},
		},
		{
			name: "For fast answer without left trimmed",
			input: "Relevant Documents: 0\nCited Documents: 0\nGrounded answer:" +
				" The tallest penguin is the <co: 0>emperor penguin</co: 0>.\n They are <co: 1>1.2 feet</co: 1> tall.",
			wantCompleteString:   " The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.",
			wantPostAnswerString: "",
			wantCitations: []FilterCitation{
				{
					StartIndex: 28,
					EndIndex:   43,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
					Text:       "emperor penguin",
				},
				{
					StartIndex: 55,
					EndIndex:   63,
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
					Text:       "1.2 feet",
				},
			},
			filterOptions: []FilterOption{
				HandleRag(),
			},
		},
		{
			name: "With no filter",
			input: "Relevant Documents: 0\nCited Documents: 0\nAnswer: The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.Grounded answer:" +
				" The tallest penguin is the <co: 0>emperor penguin</co: 0>.\n They are <co: 0>1.2 feet</co: 0> tall.",
			wantCompleteString: "Relevant Documents: 0\nCited Documents: 0\nAnswer: The tallest penguin is the emperor penguin.\n They are 1.2 feet tall.Grounded answer:" +
				" The tallest penguin is the <co: 0>emperor penguin</co: 0>.\n They are <co: 0>1.2 feet</co: 0> tall.",
			wantCitations: nil,
			filterOptions: []FilterOption{},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			var wg sync.WaitGroup
			defer wg.Wait()

			tkzr, err := tokenizers.GetTokenizer("50k")
			require.NoError(t, err)

			f := NewStreamFilter(zaptest.NewLogger(t), tkzr, tt.filterOptions...)
			wg.Add(1)
			go func() {
				defer wg.Done()
				defer f.Close()
				tokens, err := tkzr.Encode(tt.input)
				require.NoError(t, err)
				for _, token := range tokens {
					err := f.Write(token, nil)
					require.NoError(t, err)
				}
			}()
			var got string
			var gotPostAnswer string
			var gotCitations []FilterCitation
			for s := range f.Read() {
				assert.NotEmpty(t, s)
				if s.IsPostAnswer {
					gotPostAnswer += s.Text
				} else {
					got += s.Text
				}
				gotCitations = append(gotCitations, s.Citations...)
			}
			assert.Equal(t, tt.wantCompleteString, got)
			assert.Equal(t, tt.wantPostAnswerString, gotPostAnswer)
			assert.Equal(t, tt.wantCitations, gotCitations)
		})
	}
}

type MultiHopTestCase struct {
	name                        string
	completion                  string
	expectedText                string
	expectedPlan                string
	expected                    []testGeneratedToolInput
	expectedDeltaConcatenations []string
	expectedCitations           []FilterCitation
	completionTokens            []int64
	tokenizerID                 string
}

func (tt *MultiHopTestCase) RunTest(t *testing.T, cmd3 bool) {
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
	f := NewStreamFilter(zaptest.NewLogger(t), tkzr, filterOption, StreamToolActions(), StreamProcessedParams())
	fraw := NewStreamFilter(zaptest.NewLogger(t), tkzr, filterOption, StreamToolActions())
	tokens := tt.completionTokens
	if len(tokens) == 0 {
		tokens, err = tkzr.Encode(tt.completion)
		require.NoError(t, err)
	}
	wg.Add(1)
	go func() {
		defer wg.Done()
		defer f.Close()
		for _, token := range tokens {
			err := f.Write(token, nil)
			//nolint:testifylint
			require.NoError(t, err)
		}
	}()
	wg.Add(1)
	go func() {
		defer wg.Done()
		defer fraw.Close()
		for _, token := range tokens {
			err := fraw.Write(token, nil)
			//nolint:testifylint
			require.NoError(t, err)
		}
	}()
	var gotToolCalls []testGeneratedToolInput
	var gotText, gotPlan string
	var citations []FilterCitation
	for s := range f.Read() {
		assert.NotEmpty(t, s)
		if s.Text != "" {
			if s.IsToolsReason {
				gotPlan += s.Text
			} else {
				gotText += s.Text
			}
		}
		if s.ToolCalls != nil {
			gotToolCalls = MergeToolCalls(gotToolCalls, s.ToolCalls)
		}
		if s.Citations != nil {
			citations = append(citations, s.Citations...)
		}
	}
	assert.Equal(t, tt.expectedPlan, gotPlan)
	assert.Equal(t, tt.expectedText, gotText)
	assert.Equal(t, tt.expected, gotToolCalls)
	assert.Equal(t, tt.expectedCitations, citations)

	var gotToolCallDeltaConcatenations []string
	for s := range fraw.Read() {
		assert.NotEmpty(t, s)
		if s.ToolCalls != nil {
			i := s.ToolCalls.Index
			if i >= len(gotToolCallDeltaConcatenations) {
				gotToolCallDeltaConcatenations = append(gotToolCallDeltaConcatenations, s.ToolCalls.RawParamDelta)
			} else {
				gotToolCallDeltaConcatenations[i] += s.ToolCalls.RawParamDelta
			}
		}
	}
	assert.Equal(t, tt.expectedDeltaConcatenations, gotToolCallDeltaConcatenations)
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
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
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
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0, 1}}},
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
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
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
					DocIndices: []DocIndex{
						{ToolIndex: 0, ResultIndices: []int{1, 2}},
						{ToolIndex: 1, ResultIndices: []int{3, 4}},
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
					DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
					StartIndex: 18,
					EndIndex:   26,
					IsThinking: true,
				},
				{
					Text: "bar",
					DocIndices: []DocIndex{
						{ToolIndex: 0, ResultIndices: []int{1, 2}},
						{ToolIndex: 1, ResultIndices: []int{3, 4}},
					},
					StartIndex: 4,
					EndIndex:   7,
				},
			},
		},
		{
			name: "bad utf8 token test",
			completionTokens: []int64{ // Tokenized: "<|START_RESPONSE|>foo bar *invalid* <|END_RESPONSE|>"
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
			completionTokens: []int64{ // Tokenized: "<|START_RESPONSE|>foo bar *invalid* <|END_RESPONSE|>"
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

func TestRepetitionLimits(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name   string
		input  []int64
		filter []FilterOption

		err error
	}{
		{
			name:   "no_repetition_limit_set",
			input:  []int64{1, 2, 3, 3, 3, 3, 3},
			filter: []FilterOption{},
		},
		{
			name:  "repetition_limit_2_sequence_2_does_not_match_no_repetition",
			input: []int64{1, 2, 3, 4, 5},
			filter: []FilterOption{
				WithRepetitionLimit(2, 2),
			},
		},
		{
			name:  "repetition_limit_2_sequence_2_does_not_match_3_tokens",
			input: []int64{1, 2, 3, 1, 2, 3},
			filter: []FilterOption{
				WithRepetitionLimit(2, 2),
			},
		},
		{
			name:  "repetition_limit_2_sequence_2_matches_1_token",
			input: []int64{1, 2, 3, 1, 2, 3, 3},
			filter: []FilterOption{
				WithRepetitionLimit(2, 2),
			},
			err: errors.New("saw too many repeated tokens"),
		},
		{
			name:  "repetition_limit_2_sequence_2_matches_2_tokens",
			input: []int64{1, 2, 3, 1, 2, 3, 2, 3},
			filter: []FilterOption{
				WithRepetitionLimit(2, 2),
			},
			err: errors.New("saw too many repeated tokens"),
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			var wg sync.WaitGroup
			defer wg.Wait()

			tkzr, err := tokenizers.GetTokenizer("50k")
			require.NoError(t, err)

			f := NewStreamFilter(zaptest.NewLogger(t), tkzr, tt.filter...)
			wg.Add(1)
			go func() {
				defer wg.Done()
				defer f.Close()
				tokens := tt.input
				for _, token := range tokens {
					err := f.Write(token, nil)
					if err != nil {
						assert.Equal(t, tt.err, err)
						return
					}
				}
			}()
			for s := range f.Read() {
				assert.NotEmpty(t, s)
			}
		})
	}
}

func TestHasHitTokenRepetitionLimit(t *testing.T) {
	t.Parallel()

	generateRepeatedSequence := func(pattern []int64, repetitions int) []int64 {
		var result []int64
		for range repetitions {
			result = append(result, pattern...)
		}
		return result
	}

	tests := []struct {
		name              string
		tokens            []int64
		repetitionLimit   int
		maxSequenceLength int

		expected bool
	}{
		{
			name:              "2_repetitions_1_sequence_false",
			tokens:            []int64{1, 2, 3, 4, 5},
			repetitionLimit:   2,
			maxSequenceLength: 1,

			expected: false,
		},
		{
			name:              "2_repetitions_2_sequences_false",
			tokens:            []int64{1, 2, 3, 4, 5, 1, 2, 3, 4, 5},
			repetitionLimit:   2,
			maxSequenceLength: 2,

			expected: false,
		},
		{
			name:              "2_repetitions_1_sequence_true",
			tokens:            []int64{1, 2, 3, 3},
			repetitionLimit:   2,
			maxSequenceLength: 1,

			expected: true,
		},
		{
			name:              "2_repetitions_2_sequences_true",
			tokens:            []int64{1, 2, 3, 4, 1, 2, 1, 2},
			repetitionLimit:   2,
			maxSequenceLength: 2,

			expected: true,
		},
		{
			name:              "2_repetitions_3_sequence_not_enough_tokens",
			tokens:            []int64{1, 2, 3, 4, 5},
			repetitionLimit:   2,
			maxSequenceLength: 3,

			expected: false,
		},
		{
			name:              "2_repetitions_1_sequence_not_enough_tokens",
			tokens:            []int64{1},
			repetitionLimit:   2,
			maxSequenceLength: 1,

			expected: false,
		},
		{
			name:              "3_repetitions_3_sequence_matches_seq_len_1",
			tokens:            []int64{2, 1, 3, 5, 1, 1, 1},
			repetitionLimit:   3,
			maxSequenceLength: 3,

			expected: true,
		},
		{
			name:              "3_repetitions_3_sequence_matches_seq_len_2",
			tokens:            []int64{2, 1, 3, 5, 1, 1, 2, 1, 2, 1, 2},
			repetitionLimit:   3,
			maxSequenceLength: 3,

			expected: true,
		},
		{
			name:              "3_repetitions_3_sequence_matches_seq_len_3",
			tokens:            []int64{2, 1, 3, 5, 1, 2, 3, 1, 2, 3, 1, 2, 3},
			repetitionLimit:   3,
			maxSequenceLength: 3,

			expected: true,
		},
		{
			name:              "3_repetitions_3_sequence_matches_seq_len_15",
			tokens:            generateRepeatedSequence([]int64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15}, 45),
			repetitionLimit:   3,
			maxSequenceLength: 15,

			expected: true,
		},
		{
			name:              "3_repetitions_3_sequence_does_not_matches_seq_len_14",
			tokens:            generateRepeatedSequence([]int64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15}, 45),
			repetitionLimit:   3,
			maxSequenceLength: 14,

			expected: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			actual := HasHitTokenRepetitionLimit(tt.tokens, tt.repetitionLimit, tt.maxSequenceLength)
			assert.Equal(t, tt.expected, actual)
		})
	}
}

func BenchmarkHasHitTokenRepetitionLimit(b *testing.B) {
	generateRepeatedSequence := func(pattern []int64, repetitions int) []int64 {
		var result []int64
		for range repetitions {
			result = append(result, pattern...)
		}
		return result
	}

	randInt64Slice := func(n, maxVal int) []int64 {
		s := make([]int64, n)
		for i := range s {
			s[i] = rand.Int63n(int64(maxVal))
		}
		return s
	}

	tests := []struct {
		name              string
		seenTokens        []int64
		repetitionLimit   int
		maxSequenceLength int
	}{
		{
			name:              "Short 6 token sequence, non-repetitive",
			seenTokens:        []int64{1, 2, 3, 4, 5, 6},
			repetitionLimit:   3,
			maxSequenceLength: 2,
		},
		{
			name:              "Short 6 token sequence, repetitive",
			seenTokens:        []int64{1, 2, 3, 4, 4, 4},
			repetitionLimit:   3,
			maxSequenceLength: 1,
		},
		{
			name:              "Medium 500 token sequence, repetitive pairs",
			seenTokens:        generateRepeatedSequence([]int64{1, 2}, 500),
			repetitionLimit:   3,
			maxSequenceLength: 2,
		},
		{
			name:              "Large 10000 token sequence, non-repetitive",
			seenTokens:        randInt64Slice(10000, 1000),
			repetitionLimit:   5,
			maxSequenceLength: 3,
		},
		{
			name:              "Large 10000 token sequence, 1 token repetitive pattern",
			seenTokens:        generateRepeatedSequence([]int64{42}, 10000),
			repetitionLimit:   10,
			maxSequenceLength: 1,
		},
		{
			name:              "Large 10000 token sequence, 15 token repetitive pattern",
			seenTokens:        generateRepeatedSequence([]int64{33, 4679, 3190, 1719, 1690, 5816, 77096, 99, 53, 7706, 95777, 13021, 255022, 18229, 18326}, 10000),
			repetitionLimit:   10,
			maxSequenceLength: 15,
		},
	}

	for _, tt := range tests {
		b.Run(tt.name, func(b *testing.B) {
			// Enable memory profiling
			b.ReportAllocs()
			for b.Loop() {
				_ = HasHitTokenRepetitionLimit(tt.seenTokens, tt.repetitionLimit, tt.maxSequenceLength)
			}
		})
	}
}

// Ensure that HandleMultiHopCmd3 doesn't overwrite the specialTokensMap.
func TestStreamFilterSpecialTokenMapMerging(t *testing.T) {
	t.Parallel()
	var wg sync.WaitGroup
	defer wg.Wait()
	tkzr, err := tokenizers.GetTokenizer("50k")
	require.NoError(t, err)
	opts := []FilterOption{
		WithInclusiveStops("title"),
		HandleMultiHopCmd3(),
	}
	f := NewStreamFilter(zaptest.NewLogger(t), tkzr, opts...)

	input := "should see title should not see"
	tokens, err := tkzr.Encode(input)
	require.NoError(t, err)
	wg.Add(1)
	go func() {
		defer wg.Done()
		defer f.Close()
		for _, token := range tokens {
			err := f.Write(token, nil)
			assert.NoError(t, err)
		}
	}()
	actual := ""
	for s := range f.Read() {
		actual += s.Text
	}
	require.Equal(t, "should see title", actual)
}

func TestStreamFilterChunkSize(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name            string
		filterChunkSize int
		inclusiveStops  []string
		input           string
		want            []string
	}{
		{
			name:            "chunk size 1",
			filterChunkSize: 1,
			input:           "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
			want: []string{
				"L",
				"orem",
				" ipsum",
				" dolor",
				" sit",
				" amet",
				",",
				" consectetur",
				" adip",
				"iscing",
				" elit",
				"."},
		},
		{
			name:            "chunk size 3",
			filterChunkSize: 3,
			input:           "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
			want: []string{
				"Lorem ipsum",
				" dolor sit amet",
				", consectetur adip",
				"iscing elit."},
		},
		{
			name:            "chunk size 5",
			filterChunkSize: 5,
			input:           "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
			want: []string{
				"Lorem ipsum dolor sit",
				" amet, consectetur adipiscing",
				" elit."},
		},
		{
			name:            "chunk size 5 with special token at end",
			filterChunkSize: 5,
			inclusiveStops:  []string{"<LONG_END_SPECIAL_TOKEN>"},
			input:           "Lorem ipsum dolor sit amet, consectetur adipiscing elit<LONG_END_SPECIAL_TOKEN>.",
			want: []string{
				"Lorem ipsum dolor sit",
				" amet, consectetur adipiscing",
				" elit<LONG_END_SPECIAL_TOKEN>"},
		},
		{
			name:            "chunk size 5 with long partial special token at end",
			filterChunkSize: 5,
			inclusiveStops:  []string{"<LONG_END_SPECIAL_TOKEN>"},
			input:           "Lorem ipsum dolor sit amet, consectetur adipiscing elit<LONG_END_SPECIAL_",
			want: []string{
				"Lorem ipsum dolor sit",
				" amet, consectetur adipiscing",
				" elit<LONG_END_SPECIAL_"},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			var wg sync.WaitGroup
			defer wg.Wait()
			tokenizer, err := tokenizers.GetTokenizer("")
			require.NoError(t, err)

			f := NewStreamFilter(
				zaptest.NewLogger(t),
				tokenizer,
				WithChunkSize(tt.filterChunkSize),
				WithInclusiveStops(tt.inclusiveStops...),
			)
			wg.Add(1)
			go func() {
				defer wg.Done()
				defer f.Close()
				tokens, err := tokenizer.Encode(tt.input, tokenizers.NoSpecialTokens())
				assert.NoError(t, err)
				for _, token := range tokens {
					err := f.Write(token, nil)
					assert.NoError(t, err)
				}
			}()
			var got []string
			for s := range f.Read() {
				got = append(got, s.Text)
			}
			assert.ElementsMatch(t, got, tt.want)
		})
	}
}
