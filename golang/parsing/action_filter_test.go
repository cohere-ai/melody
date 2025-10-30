package parsing

import (
	"testing"

	"github.com/stretchr/testify/require"
)

func Test_ParseActions(t *testing.T) {
	t.Parallel()
	startingMetadata := filterAction{
		mode: notStarted,
	}
	testcases := []struct {
		name           string
		completion     string
		metadata       filterAction
		expected       []FilterOutput
		expectedRemove int
		cmd3           bool
	}{
		{
			name: "no tool name",
			completion: `Action: ` + "```" + `json
		[
		   {"`,
			expectedRemove: 0,
			metadata:       startingMetadata,
		},
		{
			name: "no tool name cmd3",
			completion: `<|START_ACTION|>
		[
		   {"`,
			expectedRemove: 0,
			metadata:       startingMetadata,
			cmd3:           true,
		},
		{
			name: "just tool name",
			completion: `Action: ` + "```" + `json
		[
		   {
		       "tool_name": "`,
			expectedRemove: 50,
			metadata:       startingMetadata,
		},
		{
			name: "just tool call id key cmd3",
			completion: `<|START_ACTION|>
		[
		   {
		       "tool_call_id": "`,
			expectedRemove: 54,
			metadata:       startingMetadata,
			cmd3:           true,
		},
		{
			name:       "just tool name",
			completion: `int"`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						Name:  "int",
					}},
			},
			expectedRemove: 4,
			metadata: filterAction{
				curToolCallIndex: 0,
				mode:             toolName,
			},
		},
		{
			name: "action till tool name",
			completion: `Action: ` + "```" + `json
			[
			   {
				   "tool_name": "internet_search",`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						Name:  "internet_search",
					}},
			},
			expectedRemove: 66,
			metadata:       startingMetadata,
		},
		{
			name: "action till tool call id cmd3",
			completion: `Action:
			[
			   {
				   "tool_call_id": "0",`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ID:    "0",
					}},
			},
			expectedRemove: 47,
			metadata:       startingMetadata,
			cmd3:           true,
		},
		{
			name:       "just param name",
			completion: `query2"`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name: "query2",
						}},
				}},
			expectedRemove: 7,
			metadata: filterAction{
				curToolCallIndex: 0,
				mode:             paramName,
			},
		},
		{
			name:       "param name with escaped quote",
			completion: `que\"ry2"`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name: "que\\\"ry2",
						}},
				}},
			expectedRemove: 9,
			metadata: filterAction{
				curToolCallIndex: 0,
				mode:             paramName,
			},
		},
		{
			name:       "just param value",
			completion: `query2`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name:       "param_name",
							ValueDelta: "query2",
						}},
				}},
			expectedRemove: 6,
			metadata: filterAction{
				curParamName:     "param_name",
				curToolCallIndex: 0,
				mode:             paramValue,
			},
		},
		{
			name:       "param value with escaped quote",
			completion: "que\\\"ry2",
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name:       "param_name",
							ValueDelta: "que\\\"ry2",
						}},
				}},
			expectedRemove: 8,
			metadata: filterAction{
				curParamName:     "param_name",
				curToolCallIndex: 0,
				mode:             paramValue,
			},
		},
		{
			name:       "action till tool name with escaped quotes",
			completion: `Action: ` + "```" + "json\n[\n{\n\"tool_name\": \"internet_\\\"search\",\"",
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						Name:  "internet_\\\"search",
					}},
			},
			expectedRemove: 52,
			metadata:       startingMetadata,
		},
		{
			name: "action till first parameter which isn't a string",
			completion: `Action: ` + "```" + `json
			[
			   {
				   "tool_name": "internet_search",
				   "parameters": {
					   "query": 10,`,
			expected: []FilterOutput{
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					Name:  "internet_search",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name: "query",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name:       "query",
						ValueDelta: "10",
					},
				}},
			},
			expectedRemove: 112,
			metadata:       startingMetadata,
		},
		{
			name: "whole thing one tool one parameter",
			completion: `Action: ` + "```" + `json
			[
			   {
				   "tool_name": "internet_search",
				   "parameters": {
					   "query": "query1"
				   }
			   }
			]` + "```",
			expected: []FilterOutput{
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					Name:  "internet_search",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name: "query",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name:       "query",
						ValueDelta: "\"query1\"",
					},
				}},
			},
			expectedRemove: 118,
			metadata:       startingMetadata,
		},
		{
			name: "whole thing two tools one parameter and two parameters",
			completion: `Action: ` + "```" + `json
			[
			   {
				   "tool_name": "foo",
				   "parameters": {
					   "query1": "value1",
					   "query3": 10
				   }
			   },
			   {
					"tool_name": "bar",
					"parameters": {
					   "query2": "value2"
					}
			   }
			]` + "```",
			expected: []FilterOutput{
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					Name:  "foo",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name: "query1",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name:       "query1",
						ValueDelta: "\"value1\"",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name: "query3",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name:       "query3",
						ValueDelta: "10",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					Name:  "bar",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					ParamDelta: &FilterToolParameter{
						Name: "query2",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					ParamDelta: &FilterToolParameter{
						Name:       "query2",
						ValueDelta: "\"value2\"",
					},
				}},
			},
			expectedRemove: 229,
			metadata:       startingMetadata,
		},
		{
			name: "whole thing two tools one parameter and two parameters for cmd3",
			completion: `<|START_ACTION|>
			[
			   {
				   "tool_call_id": "1",
				   "tool_name": "foo",
				   "parameters": {
					   "query1": "value1",
					   "query3": 10
				   }
			   },
			   {
				    "tool_call_id": "2",
					"tool_name": "bar",
					"parameters": {
					   "query2": "value2"
					}
			   }
			]<|END_ACTION|>`,
			expected: []FilterOutput{
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ID:    "1",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					Name:  "foo",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name: "query1",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name:       "query1",
						ValueDelta: "\"value1\"",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name: "query3",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name:       "query3",
						ValueDelta: "10",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					ID:    "2",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					Name:  "bar",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					ParamDelta: &FilterToolParameter{
						Name: "query2",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					ParamDelta: &FilterToolParameter{
						Name:       "query2",
						ValueDelta: "\"value2\"",
					},
				}},
			},
			expectedRemove: 289,
			metadata:       startingMetadata,
			cmd3:           true,
		},
		{
			name: "just param value",
			completion: `import matplotlib.pyplot as plt\n\n# Data for\n",
			"code2"
			`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name:       "code",
							ValueDelta: "import matplotlib.pyplot as plt\\n\\n# Data for\\n\"",
						}},
				},
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name: "code2",
						}},
				},
			},
			expectedRemove: 60,
			metadata: filterAction{
				curToolCallIndex: 0,
				curParamName:     "code",
				mode:             paramValue,
				curParamState:    complexType,
				paramValueBuffer: "\"",
			},
		},
	}

	for _, tt := range testcases {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			f := filter{
				actionMetaData:        tt.metadata,
				streamToolActions:     true,
				streamProcessedParams: true,
			}
			if tt.cmd3 {
				f.hasToolCallID = true
			}
			out, actualRemove := f.ParseActions(tt.completion)
			require.Equal(t, tt.expectedRemove, actualRemove)
			require.Equal(t, tt.expected, out)
		})
	}
}

func Test_ParseRawActions(t *testing.T) {
	t.Parallel()
	startingMetadata := filterAction{
		mode: notStarted,
	}
	testcases := []struct {
		name           string
		completion     string
		metadata       filterAction
		expected       []FilterOutput
		expectedRemove int
		cmd3           bool
	}{
		{
			name: "no tool name",
			completion: `Action: ` + "```" + `json
		[
		   {"`,
			expectedRemove: 0,
			metadata:       startingMetadata,
		},
		{
			name: "no tool name cmd3",
			completion: `<|START_ACTION|>
		[
		   {"`,
			expectedRemove: 0,
			metadata:       startingMetadata,
			cmd3:           true,
		},
		{
			name: "just tool name",
			completion: `Action: ` + "```" + `json
		[
		   {
		       "tool_name": "`,
			expectedRemove: 50,
			metadata:       startingMetadata,
		},
		{
			name:       "just tool name",
			completion: `int"`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						Name:  "int",
					}},
			},
			expectedRemove: 4,
			metadata: filterAction{
				curToolCallIndex: 0,
				mode:             toolName,
			},
		},
		{
			name: "action till tool name",
			completion: `Action: ` + "```" + `json
			[
			   {
				   "tool_name": "internet_search",`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						Name:  "internet_search",
					}},
			},
			expectedRemove: 66,
			metadata:       startingMetadata,
		},
		{
			name:       "just param name",
			completion: `query2"`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name: "query2",
						}},
				}},
			expectedRemove: 7,
			metadata: filterAction{
				curToolCallIndex: 0,
				mode:             paramName,
			},
		},
		{
			name:       "param name with escaped quote",
			completion: `que\"ry2"`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name: "que\\\"ry2",
						}},
				}},
			expectedRemove: 9,
			metadata: filterAction{
				curToolCallIndex: 0,
				mode:             paramName,
			},
		},
		{
			name:       "just param value",
			completion: `query2`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name:       "param_name",
							ValueDelta: "query2",
						}},
				}},
			expectedRemove: 6,
			metadata: filterAction{
				curParamName:     "param_name",
				curToolCallIndex: 0,
				mode:             paramValue,
			},
		},
		{
			name:       "param value with escaped quote",
			completion: "que\\\"ry2",
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name:       "param_name",
							ValueDelta: "que\\\"ry2",
						}},
				}},
			expectedRemove: 8,
			metadata: filterAction{
				curParamName:     "param_name",
				curToolCallIndex: 0,
				mode:             paramValue,
			},
		},
		{
			name:       "action till tool name with escaped quotes",
			completion: `Action: ` + "```" + "json\n[\n{\n\"tool_name\": \"internet_\\\"search\",\"",
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						Name:  "internet_\\\"search",
					}},
			},
			expectedRemove: 52,
			metadata:       startingMetadata,
		},

		{
			name: "action till first parameter which isn't a string",
			completion: `Action: ` + "```" + `json
			[
			   {
				   "tool_name": "internet_search",
				   "parameters": {
					   "query": 10,`,
			expected: []FilterOutput{
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					Name:  "internet_search",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index:         0,
					RawParamDelta: "{\n\"query\": 10,",
				}},
			},
			expectedRemove: 112,
			metadata:       startingMetadata,
		},
		{
			name: "whole thing one tool one parameter",
			completion: `Action: ` + "```" + `json
			[
			   {
				   "tool_name": "internet_search",
				   "parameters": {
					   "query": "query1"
				   }
			   }
			]` + "```",
			expected: []FilterOutput{
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					Name:  "internet_search",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index:         0,
					RawParamDelta: "{\n\"query\": \"query1\"\n}",
				}},
			},
			expectedRemove: 126,
			metadata:       startingMetadata,
		},
		{
			name: "whole thing two tools one parameter and two parameters",
			completion: `Action: ` + "```" + `json
			[
			   {
				   "tool_name": "foo",
				   "parameters": {
					   "query1": "value1",
					   "query3": 10
				   }
			   },
			   {
					"tool_name": "bar",
					"parameters": {
					   "query2": "value2"
					}
			   }
			]` + "```",
			expected: []FilterOutput{
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					Name:  "foo",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index:         0,
					RawParamDelta: "{\n\"query1\": \"value1\",\n\"query3\": 10\n}",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					Name:  "bar",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index:         1,
					RawParamDelta: "{\n\"query2\": \"value2\"\n}",
				}},
			},
			expectedRemove: 235,
			metadata:       startingMetadata,
		},
		{
			name: "whole thing two tools one parameter and two parameters cmd3",
			completion: `<|START_ACTION|>
			[
			   {
				   "tool_call_id": "0",
				   "tool_name": "foo",
				   "parameters": {
					   "query1": "value1",
					   "query3": 10
				   }
			   },
			   {
			   	    "tool_call_id": "1",
					"tool_name": "bar",
					"parameters": {
					   "query2": "value2"
					}
			   }
			]<|END_ACTION|>`,
			expected: []FilterOutput{
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ID:    "0",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					Name:  "foo",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index:         0,
					RawParamDelta: "{\n\"query1\": \"value1\",\n\"query3\": 10\n}",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					ID:    "1",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					Name:  "bar",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index:         1,
					RawParamDelta: "{\n\"query2\": \"value2\"\n}",
				}},
			},
			expectedRemove: 298,
			metadata:       startingMetadata,
			cmd3:           true,
		}, {
			name: "cmd3 missing a closing }",
			completion: `<|START_ACTION|>
			[
			   {
				   "tool_call_id": "0",
				   "tool_name": "foo",
				   "parameters": {
					   "query1": "value1",
					   "query3": 10
				   }
			   },
			   {
					"tool_call_id": "1",
					"tool_name": "bar",
					"parameters": {
					   "query2": "value2"
					}
			]<|END_ACTION|>`,
			expected: []FilterOutput{
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ID:    "0",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					Name:  "foo",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index:         0,
					RawParamDelta: "{\n\"query1\": \"value1\",\n\"query3\": 10\n}",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					ID:    "1",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 1,
					Name:  "bar",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index:         1,
					RawParamDelta: "{\n\"query2\": \"value2\"\n}",
				}},
			},
			expectedRemove: 292,
			metadata:       startingMetadata,
			cmd3:           true,
		},
		{
			name: "just param value",
			completion: `import matplotlib.pyplot as plt\n\n# Data for\n",
			"code2"
			`,
			expected: []FilterOutput{
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name:       "code",
							ValueDelta: "import matplotlib.pyplot as plt\\n\\n# Data for\\n\"",
						}},
				},
				{
					ToolCalls: &FilterToolCallDelta{
						Index: 0,
						ParamDelta: &FilterToolParameter{
							Name: "code2",
						}},
				},
			},
			expectedRemove: 60,
			metadata: filterAction{
				curToolCallIndex: 0,
				curParamName:     "code",
				mode:             paramValue,
				curParamState:    complexType,
				paramValueBuffer: "\"",
			},
		},
	}

	for _, tt := range testcases {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()

			f := filter{
				actionMetaData:    tt.metadata,
				streamToolActions: true,
			}
			if tt.cmd3 {
				f.hasToolCallID = true
			}
			out, actualRemove := f.ParseActions(tt.completion)
			require.Equal(t, tt.expectedRemove, actualRemove)
			require.Equal(t, tt.expected, out)
		})
	}
}

func TestHandleLlamaTools(t *testing.T) {
	t.Parallel()
	startingMetadata := filterAction{
		mode: notStarted,
	}

	tests := []struct {
		name           string
		completion     string
		metadata       filterAction
		expected       []FilterOutput
		expectedRemove int
	}{
		{
			name:           "tool generation",
			completion:     `\n\n<|python_tag|>{"name": "internet_search", "parameters": {"query": "Sound of Music company S&P 500 year"}}<|eom_id|>`,
			metadata:       startingMetadata,
			expectedRemove: 109,
			expected: []FilterOutput{
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					Name:  "internet_search",
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name: "query",
					},
				}},
				{ToolCalls: &FilterToolCallDelta{
					Index: 0,
					ParamDelta: &FilterToolParameter{
						Name:       "query",
						ValueDelta: `"Sound of Music company S&P 500 year"`,
					},
				}},
			},
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()

			f := filter{
				actionMetaData:        tc.metadata,
				streamToolActions:     true,
				streamProcessedParams: true,
				specialTokenMap:       map[string]filterMode{},
			}
			HandleLlama()(&f)
			out, actualRemove := f.ParseActions(tc.completion)

			require.Equal(t, tc.expected, out)
			require.Equal(t, tc.expectedRemove, actualRemove)
		})
	}
}
