package parsing

// FilterOption is an "option" type for configuring the filter.
type FilterOption func(*Options)

// WithChunkSize Warning: This option is not well tested. We rolled out quickly to unblock evals and it is only used internally.
func WithChunkSize(size int) FilterOption {
	return func(f *Options) {
		f.ChunkSize = size
	}
}

// WithRepetitionLimit sets a limit on how many times a token can be repeated in the output before erroring.
func WithRepetitionLimit(limit int, maxSequenceLength int) FilterOption {
	// Set a hard limit to how many repetitions of the same sequence we allow before sending an error.
	return func(f *Options) {
		f.MaxRepetitionLimit = limit
		f.MaxRepetitionSequenceLength = maxSequenceLength
	}
}

// WithInclusiveStops adds inclusive stops to the filter.
func WithInclusiveStops(inclusiveStops ...string) FilterOption {
	return func(f *Options) {
		for _, stop := range inclusiveStops {
			if _, ok := f.SpecialTokenMap[stop]; !ok {
				// Only add if it's not already in the special token map
				f.SpecialTokenMap[stop] = inclusiveStop
			}
		}
	}
}

// WithExclusiveStops adds exclusive stops to the filter
func WithExclusiveStops(exclusiveStops ...string) FilterOption {
	return func(f *Options) {
		for _, stop := range exclusiveStops {
			if _, ok := f.SpecialTokenMap[stop]; !ok {
				// Only add if it's not already in the special token map
				f.SpecialTokenMap[stop] = exclusiveStop
			}
		}
	}
}

// WithLeftTrimmed will trim whitespace from the start of the response
func WithLeftTrimmed() FilterOption {
	return func(f *Options) {
		f.LeftTrimmed = true
	}
}

// WithRightTrimmed will trim whitespace from the end of the response
func WithRightTrimmed() FilterOption {
	return func(f *Options) {
		f.RightTrimmed = true
	}
}

// WithPrefixTrim will trim the given prefix from the start of the response
func WithPrefixTrim(prefix string) FilterOption {
	return func(f *Options) {
		f.TrimPrefix = prefix
	}
}

// HandleRag enables parsing for the RAG formats (cmd2?)
func HandleRag() FilterOption {
	return func(f *Options) {
		f.DefaultMode = ignore
		f.RightTrimmed = true
		// Need to create a copy as we will modify the map
		f.mergeIntoSpecialTokenMap(ragTokenMap)
	}
}

// HandleSearchQuery enables parsing for the search query generation formats (cmd2)
func HandleSearchQuery() FilterOption {
	return func(f *Options) {
		f.DefaultMode = ignore
		f.RightTrimmed = true
		f.mergeIntoSpecialTokenMap(searchQueryTokenMap)
	}
}

// HandleMultiHop Temporary until the reader handles actions
func HandleMultiHop() FilterOption {
	return func(f *Options) {
		f.DefaultMode = ignore
		f.RightTrimmed = true
		f.mergeIntoSpecialTokenMap(multiHopTokenMap)
	}
}

// HandleMultiHopCmd3 enables parsing for the format used in command-3 models.
func HandleMultiHopCmd3() FilterOption {
	return func(f *Options) {
		// default needs to be grounded since response sometimes does not start with <|START_RESPONSE|>
		f.DefaultMode = groundedAnswer
		f.RightTrimmed = true
		f.HasToolCallID = true
		f.mergeIntoSpecialTokenMap(multiHopTokenMapCmd3)
		f.Cmd3Citations = true
	}
}

// HandleMultiHopCmd4 enables parsing for the format used in command-4 models.
func HandleMultiHopCmd4() FilterOption {
	return func(f *Options) {
		// default needs to be grounded since response sometimes does not start with <|START_RESPONSE|>
		f.DefaultMode = groundedAnswer
		f.RightTrimmed = true
		f.HasToolCallID = true
		f.mergeIntoSpecialTokenMap(multiHopTokenMapCmd4)
		f.Cmd3Citations = true
	}
}

// HandleLlama enables parsing for the format used in LLaMA models.
func HandleLlama() FilterOption {
	return func(f *Options) {
		f.DefaultMode = groundedAnswer
		f.RightTrimmed = true
		f.mergeIntoSpecialTokenMap(llamaTokenMap)
		f.LlamaToolParsing = true
	}
}

func StreamNonGroundedAnswer() FilterOption {
	return func(f *Options) {
		f.StreamNonGroundedAnswer = true
	}
}

func StreamToolActions() FilterOption {
	return func(f *Options) {
		f.StreamToolActions = true
	}
}

func StreamProcessedParams() FilterOption {
	return func(f *Options) {
		f.StreamProcessedParams = true
	}
}

// RemoveToken is used for disableEOS to remove the END_RESPONSE/END_TEXT token
func RemoveToken(token string) FilterOption {
	return func(f *Options) {
		delete(f.SpecialTokenMap, token)
	}
}

func (f *Options) mergeIntoSpecialTokenMap(toMerge map[string]FilterMode) {
	for k, v := range toMerge {
		if _, ok := f.SpecialTokenMap[k]; ok {
			continue
		}
		f.SpecialTokenMap[k] = v
	}
}
