package parsing

var ragTokenMap = map[string]FilterMode{
	"Grounded answer:": groundedAnswer,
	"Answer:":          answer,
}

var multiHopTokenMap = map[string]FilterMode{
	"Grounded answer:": groundedAnswer,
	"Answer:":          answer,
	"Plan:":            toolReason,
	"Reflection:":      toolReason,
	"Action:":          toolAction,
	// We ignore Relevant & Cited documents because we do not want to stream them to the user.
	// They are present in the generation for model performance.
	"Relevant Documents:": ignore,
	"Cited Documents:":    ignore,
}

// TODO add ticket for token change
var multiHopTokenMapCmd3 = map[string]FilterMode{
	"<|START_RESPONSE|>": groundedAnswer,
	"<|END_RESPONSE|>":   ignore,
	"<|START_THINKING|>": toolReason,
	"<|END_THINKING|>":   ignore,
	"<|START_ACTION|>":   toolAction,
	"<|END_ACTION|>":     ignore,
}

var multiHopTokenMapCmd4 = map[string]FilterMode{
	"<|START_TEXT|>":     groundedAnswer,
	"<|END_TEXT|>":       ignore,
	"<|START_THINKING|>": toolReason,
	"<|END_THINKING|>":   ignore,
	"<|START_ACTION|>":   toolAction,
	"<|END_ACTION|>":     ignore,
}

var searchQueryTokenMap = map[string]FilterMode{
	"Search:": searchQuery,
	"|||":     nextSearchQuery,
	"\n":      nextSearchQuery,
}

var llamaTokenMap = map[string]FilterMode{
	"\n\n":           groundedAnswer,
	"<|python_tag|>": toolAction,
	"<eom_id>":       exclusiveStop,
}
