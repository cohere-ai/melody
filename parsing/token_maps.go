package parsing

var ragTokenMap = map[string]filterMode{
	"Grounded answer:": groundedAnswer,
	"answer:":          answer,
}

var multiHopTokenMap = map[string]filterMode{
	"Grounded answer:": groundedAnswer,
	"answer:":          answer,
	"Plan:":            toolReason,
	"Reflection:":      toolReason,
	"Action:":          toolAction,
	// We ignore Relevant & Cited documents because we do not want to stream them to the user.
	// They are present in the generation for model performance.
	"Relevant Documents:": ignore,
	"Cited Documents:":    ignore,
}

// TODO add ticket for token change
var multiHopTokenMapCmd3 = map[string]filterMode{
	"<|START_RESPONSE|>": groundedAnswer,
	"<|END_RESPONSE|>":   ignore,
	"<|START_THINKING|>": toolReason,
	"<|END_THINKING|>":   ignore,
	"<|START_ACTION|>":   toolAction,
	"<|END_ACTION|>":     ignore,
}

var multiHopTokenMapCmd4 = map[string]filterMode{
	"<|START_TEXT|>":     groundedAnswer,
	"<|END_TEXT|>":       ignore,
	"<|START_THINKING|>": toolReason,
	"<|END_THINKING|>":   ignore,
	"<|START_ACTION|>":   toolAction,
	"<|END_ACTION|>":     ignore,
}

var searchQueryTokenMap = map[string]filterMode{
	"Search:": searchQuery,
	"|||":     nextSearchQuery,
	"\n":      nextSearchQuery,
}

var llamaTokenMap = map[string]filterMode{
	"\n\n":           groundedAnswer,
	"<|python_tag|>": toolAction,
	"<eom_id>":       exclusiveStop,
}
