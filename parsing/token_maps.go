package parsing

var ragTokenMap = map[string]FilterMode{
	"Grounded answer:": GroundedAnswer,
	"Answer:":          Answer,
}

var multiHopTokenMap = map[string]FilterMode{
	"Grounded answer:": GroundedAnswer,
	"Answer:":          Answer,
	"Plan:":            ToolReason,
	"Reflection:":      ToolReason,
	"Action:":          ToolAction,
	// We ignore Relevant & Cited documents because we do not want to stream them to the user.
	// They are present in the generation for model performance.
	"Relevant Documents:": Ignore,
	"Cited Documents:":    Ignore,
}

// TODO add ticket for token change
var multiHopTokenMapCmd3 = map[string]FilterMode{
	"<|START_RESPONSE|>": GroundedAnswer,
	"<|END_RESPONSE|>":   Ignore,
	"<|START_THINKING|>": ToolReason,
	"<|END_THINKING|>":   Ignore,
	"<|START_ACTION|>":   ToolAction,
	"<|END_ACTION|>":     Ignore,
}

var multiHopTokenMapCmd4 = map[string]FilterMode{
	"<|START_TEXT|>":     GroundedAnswer,
	"<|END_TEXT|>":       Ignore,
	"<|START_THINKING|>": ToolReason,
	"<|END_THINKING|>":   Ignore,
	"<|START_ACTION|>":   ToolAction,
	"<|END_ACTION|>":     Ignore,
}

var searchQueryTokenMap = map[string]FilterMode{
	"Search:": SearchQuery,
	"|||":     NextSearchQuery,
	"\n":      NextSearchQuery,
}

var llamaTokenMap = map[string]FilterMode{
	"\n\n":           GroundedAnswer,
	"<|python_tag|>": ToolAction,
	"<eom_id>":       ExclusiveStop,
}
