package parsing

import (
	"testing"

	"github.com/stretchr/testify/require"
	"go.uber.org/zap/zapcore"
	"go.uber.org/zap/zaptest"
)

func Test_HandleCitations(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name   string
		input  string
		output *FilterOutput
		remove int
		filter filter
	}{
		{
			name: "standard case",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 2,1>foo</co: 2,1>",
			output: &FilterOutput{
				Text: "hello foo",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
					},
				},
			},
			remove: 28,
		},
		{
			name: "standard case with handleCitationsDuringAnswer ",
			filter: filter{
				streamNonGroundedAnswer: false,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 2,1>foo</co: 2,1>",
			output: &FilterOutput{
				Text: "hello foo",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
					},
				},
			},
			remove: 28,
		},
		{
			name: "no document",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: >foo</co: >",
			output: &FilterOutput{
				Text: "hello foo",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
					},
				},
			},
			remove: 22,
		},
		{
			name: "non int document",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 2, foo>foo</co: 2, foo>",
			output: &FilterOutput{
				Text: "hello foo",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{2}}},
					},
				},
			},
			remove: 34,
		},

		{
			name: "different documents",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 1,2>foo</co: 3,4>",
			output: &FilterOutput{
				Text: "hello foo",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{3, 4}}},
					},
				},
			},
			remove: 28,
		},
		{
			name: "no citation",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input: "hello coo",
			output: &FilterOutput{
				Text: "hello coo",
			},
			remove: 9,
		},
		{
			name: "incomplete first citation",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input:  "<",
			output: nil,
			remove: 0,
		},
		{
			name: "incomplete first citation more",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input:  "hello coo <co: 2",
			output: nil,
			remove: 0,
		},
		{
			name: "incomplete first citation more - with handleCitationsDuringAnswer",
			filter: filter{
				streamNonGroundedAnswer: false,
				curCitationByteIndex:    -1,
			},
			input:  "hello coo <co: 2",
			output: nil,
			remove: 0,
		},
		{
			name: "only first citation element ",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input:  "hello coo <co: 2,1>fo",
			output: nil,
			remove: 0,
		},
		{
			name: "only first citation element  - with handleCitationsDuringAnswer",
			filter: filter{
				streamNonGroundedAnswer: false,
				curCitationByteIndex:    -1,
			},
			input: "hello coo <co: 2,1>fo",
			output: &FilterOutput{
				Text: "hello coo fo",
			},
			remove: 10,
		},
		{
			name: "only first citation element  - with handleCitationsDuringAnswer, already streamed some",
			filter: filter{
				streamNonGroundedAnswer: false,
				curCitationByteIndex:    14,
			},
			input: "<co: 2,1>foo bar",
			output: &FilterOutput{
				Text: "ar",
			},
			remove: 0,
		},
		{
			name: "incomplete end citation element ",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input:  "hello <co: 2,1>foo<",
			output: nil,
			remove: 0,
		},
		{
			name: "incomplete end citation element  - with handleCitationsDuringAnswer",
			filter: filter{
				streamNonGroundedAnswer: false,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 2,1>foo<",
			output: &FilterOutput{
				Text: "hello foo",
			},
			remove: 6,
		},
		{
			name: "incomplete end citation element more",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input:  "hello <co: 2,1>foo</co: 2,1",
			output: nil,
			remove: 0,
		},
		{
			name: "multiple citations",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 2,1>foo</co: 2,1> hi <co: 0>barber</co: 0>",
			output: &FilterOutput{
				Text: "hello foo hi barber",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
					},
					{
						StartIndex: 13,
						EndIndex:   19,
						Text:       "barber",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{0}}},
					},
				},
			},
			remove: 53,
		},
		{
			name: "multiple citations - second no citation",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 2,1>foo</co: 2,1> hi",
			output: &FilterOutput{
				Text: "hello foo hi",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
					},
				},
			},
			remove: 31,
		},
		{
			name: "multiple citations - second incomplete first citation",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 2,1>foo</co: 2,1> hi <",
			output: &FilterOutput{
				Text: "hello foo",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
					},
				},
			},
			remove: 28,
		},
		{
			name: "multiple citations - only first citation",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 2,1>foo</co: 2,1> hi <co: 2,1>",
			output: &FilterOutput{
				Text: "hello foo",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
					},
				},
			},
			remove: 28,
		},
		{
			name: "multiple citations - incomplete end citation",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 2,1>foo</co: 2,1> hi <co: 2,1>barber<",
			output: &FilterOutput{
				Text: "hello foo",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
					},
				},
			},
			remove: 28,
		},
		{
			name: "multiple citations - incomplete end citation - handleCitationsDuringAnswer",
			filter: filter{
				streamNonGroundedAnswer: false,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 2,1>foo</co: 2,1> hi <co: 2,1>barber<",
			output: &FilterOutput{
				Text: "hello foo hi barber",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
					},
				},
			},
			remove: 32,
		},
		{
			name: "multiple citations - incomplete end citation more",
			filter: filter{
				streamNonGroundedAnswer: true,
				curCitationByteIndex:    -1,
			},
			input: "hello <co: 2,1>foo</co: 2,1> hi <co: 2,1>barber</co: 2,1",
			output: &FilterOutput{
				Text: "hello foo",
				Citations: []FilterCitation{
					{
						StartIndex: 6,
						EndIndex:   9,
						Text:       "foo",
						DocIndices: []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
					},
				},
			},
			remove: 28,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			output, remove := tt.filter.ParseCitations(tt.input, groundedAnswer)
			require.Equal(t, tt.output, output)
			require.Equal(t, tt.remove, remove)
		})
	}
}

func Test_FindAnElement(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name               string
		input              string
		inputStartElement  string
		inputEndElement    string
		expectedStartIndex int
		expectedEndIndex   int
		expectedDocs       []DocIndex
		cmd3Citations      bool
	}{
		{
			name:               "standard case - find start",
			input:              "hello <co: 2,1> foo </co: 2,1>",
			inputStartElement:  "<co: ",
			inputEndElement:    ">",
			expectedStartIndex: 6,
			expectedEndIndex:   14,
			expectedDocs:       []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
		},
		{
			name:               "no citation",
			input:              "hello",
			inputStartElement:  "<co: ",
			inputEndElement:    ">",
			expectedStartIndex: -1,
			expectedEndIndex:   -1,
			expectedDocs:       nil,
		},
		{
			name:               "partial start element",
			input:              "hello <c",
			inputStartElement:  "<co: ",
			inputEndElement:    ">",
			expectedStartIndex: 6,
			expectedEndIndex:   -1,
			expectedDocs:       nil,
		},
		{
			name:               "partial start element without space",
			input:              "hello <co:",
			inputStartElement:  "<co: ",
			inputEndElement:    ">",
			expectedStartIndex: 6,
			expectedEndIndex:   -1,
			expectedDocs:       nil,
		},
		{
			name:               "no end element",
			input:              "hello <co: 0,",
			inputStartElement:  "<co: ",
			inputEndElement:    ">",
			expectedStartIndex: 6,
			expectedEndIndex:   -1,
			expectedDocs:       nil,
		},
		{
			name:               "no documents",
			input:              "hello <co: >",
			inputStartElement:  "<co: ",
			inputEndElement:    ">",
			expectedStartIndex: 6,
			expectedEndIndex:   11,
			expectedDocs:       nil,
		},
		{
			name:               "one documents",
			input:              "hello <co: 1>",
			inputStartElement:  "<co: ",
			inputEndElement:    ">",
			expectedStartIndex: 6,
			expectedEndIndex:   12,
			expectedDocs:       []DocIndex{{ToolIndex: 0, ResultIndices: []int{1}}},
		},
		{
			name:               "two documents",
			input:              "hello <co: 2,1>",
			inputStartElement:  "<co: ",
			inputEndElement:    ">",
			expectedStartIndex: 6,
			expectedEndIndex:   14,
			expectedDocs:       []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
		},
		{
			name:               "trailing comma",
			input:              "hello <co: 2,>",
			inputStartElement:  "<co: ",
			inputEndElement:    ">",
			expectedStartIndex: 6,
			expectedEndIndex:   13,
			expectedDocs:       []DocIndex{{ToolIndex: 0, ResultIndices: []int{2}}},
		},
		{
			name:               "incorrect documents",
			input:              "hello <co: 2,foo>",
			inputStartElement:  "<co: ",
			inputEndElement:    ">",
			expectedStartIndex: 6,
			expectedEndIndex:   16,
			expectedDocs:       []DocIndex{{ToolIndex: 0, ResultIndices: []int{2}}},
		},
		{
			name:               "loads of stuff between start and end",
			input:              "hello <co: 2,1 foo </co: 2,1>",
			inputStartElement:  "<co: ",
			inputEndElement:    ">",
			expectedStartIndex: 6,
			expectedEndIndex:   28,
			expectedDocs:       []DocIndex{{ToolIndex: 0, ResultIndices: []int{2, 1}}},
		},
		{
			name:               "two tools cmd3",
			input:              "<co> hello </co: 0:[1,2],1:[0]>",
			inputStartElement:  "</co: ",
			inputEndElement:    ">",
			expectedStartIndex: 11,
			expectedEndIndex:   30,
			expectedDocs:       []DocIndex{{ToolIndex: 0, ResultIndices: []int{1, 2}}, {ToolIndex: 1, ResultIndices: []int{0}}},
			cmd3Citations:      true,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			f := filter{logger: zaptest.NewLogger(t, zaptest.Level(zapcore.ErrorLevel))}
			startIndex, endIndex, docs := f.findAnElement(tt.input, tt.inputStartElement, tt.inputEndElement, tt.cmd3Citations)
			require.Equal(t, tt.expectedStartIndex, startIndex)
			require.Equal(t, tt.expectedEndIndex, endIndex)
			require.ElementsMatch(t, tt.expectedDocs, docs)
		})
	}
}

func Test_ConvertStringToIntList(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name   string
		input  string
		output []int
	}{
		{
			name:   "single document",
			input:  "0",
			output: []int{0},
		},
		{
			name:   "single document, trailing comma",
			input:  "0,",
			output: []int{0},
		},
		{
			name:   "multiple documents",
			input:  "0,1",
			output: []int{0, 1},
		},
		{
			name:   "multiple documents, different order",
			input:  "1,0",
			output: []int{1, 0},
		},
		{
			name:   "no documents",
			input:  "",
			output: []int{},
		},
		{
			name:   "not an index",
			input:  "foo",
			output: []int{},
		},
		{
			name:   "part valid, part not an index",
			input:  "foo,0",
			output: []int{0},
		},
		{
			name:   "index large, positive",
			input:  "999",
			output: []int{999},
		},
		{
			name:   "index not found, negative",
			input:  "-1",
			output: []int{},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			output := convertStringToIntList(tt.input)
			require.Equal(t, tt.output, output)
		})
	}
}
