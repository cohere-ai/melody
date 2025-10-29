package parsing

import "go.uber.org/zap"

// FilterOption is an "option" type for configuring the filter.
type FilterOption func(*filter)

// WithZapLogger sets a custom zap logger for the filter.
func WithZapLogger(logger *zap.Logger) FilterOption {
	return func(f *filter) {
		f.logger = logger
	}
}

// WithChunkSize Warning: This option is not well tested. We rolled out quickly to unblock evals and it is only used internally.
func WithChunkSize(size int) FilterOption {
	return func(f *filter) {
		f.chunkSize = size
	}
}

// WithRepetitionLimit sets a limit on how many times a token can be repeated in the output before erroring.
func WithRepetitionLimit(limit int, maxSequenceLength int) FilterOption {
	// Set a hard limit to how many repetitions of the same sequence we allow before sending an error.
	return func(f *filter) {
		f.maxRepetitionLimit = limit
		f.maxRepetitionSequenceLength = maxSequenceLength
	}
}

// WithInclusiveStops adds inclusive stops to the filter.
func WithInclusiveStops(inclusiveStops ...string) FilterOption {
	return func(f *filter) {
		for _, stop := range inclusiveStops {
			if _, ok := f.specialTokenMap[stop]; !ok {
				// Only add if it's not already in the special token map
				f.specialTokenMap[stop] = inclusiveStop
			} else {
				f.logger.Warn("ignoring inclusive stop: already exists in specialTokenMap", zap.String("stop", stop))
			}
		}
	}
}

// WithExclusiveStops adds exclusive stops to the filter
func WithExclusiveStops(exclusiveStops ...string) FilterOption {
	return func(f *filter) {
		for _, stop := range exclusiveStops {
			if _, ok := f.specialTokenMap[stop]; !ok {
				// Only add if it's not already in the special token map
				f.specialTokenMap[stop] = exclusiveStop
			} else {
				f.logger.Warn("ignoring exclusive stop: already exists in specialTokenMap", zap.String("stop", stop))
			}
		}
	}
}

// WithLeftTrimmed will trim whitespace from the start of the response
func WithLeftTrimmed() FilterOption {
	return func(f *filter) {
		f.leftTrimmed = true
	}
}

// WithRightTrimmed will trim whitespace from the end of the response
func WithRightTrimmed() FilterOption {
	return func(f *filter) {
		f.rightTrimmed = true
	}
}

// WithPrefixTrim will trim the given prefix from the start of the response
func WithPrefixTrim(prefix string) FilterOption {
	return func(f *filter) {
		f.trimPrefix = prefix
	}
}

// HandleRag enables parsing for the RAG formats (cmd2?)
func HandleRag() FilterOption {
	return func(f *filter) {
		f.defaultMode = ignore
		f.rightTrimmed = true
		// Need to create a copy as we will modify the map
		f.mergeIntoSpecialTokenMap(ragTokenMap)
	}
}

// HandleSearchQuery enables parsing for the search query generation formats (cmd2)
func HandleSearchQuery() FilterOption {
	return func(f *filter) {
		f.defaultMode = ignore
		f.rightTrimmed = true
		f.mergeIntoSpecialTokenMap(searchQueryTokenMap)
	}
}

// HandleMultiHop Temporary until the reader handles actions
func HandleMultiHop() FilterOption {
	return func(f *filter) {
		f.defaultMode = ignore
		f.rightTrimmed = true
		f.mergeIntoSpecialTokenMap(multiHopTokenMap)
	}
}

// HandleMultiHopCmd3 enables parsing for the format used in command-3 models.
func HandleMultiHopCmd3() FilterOption {
	return func(f *filter) {
		// default needs to be grounded since response sometimes does not start with <|START_RESPONSE|>
		f.defaultMode = groundedAnswer
		f.rightTrimmed = true
		f.hasToolCallID = true
		f.mergeIntoSpecialTokenMap(multiHopTokenMapCmd3)
		f.cmd3Citations = true
	}
}

// HandleMultiHopCmd4 enables parsing for the format used in command-4 models.
func HandleMultiHopCmd4() FilterOption {
	return func(f *filter) {
		// default needs to be grounded since response sometimes does not start with <|START_RESPONSE|>
		f.defaultMode = groundedAnswer
		f.rightTrimmed = true
		f.hasToolCallID = true
		f.mergeIntoSpecialTokenMap(multiHopTokenMapCmd4)
		f.cmd3Citations = true
	}
}

// HandleLlama enables parsing for the format used in LLaMA models.
func HandleLlama() FilterOption {
	return func(f *filter) {
		f.defaultMode = groundedAnswer
		f.rightTrimmed = true
		f.mergeIntoSpecialTokenMap(llamaTokenMap)
		f.llamaToolParsing = true
	}
}

func StreamNonGroundedAnswer() FilterOption {
	return func(f *filter) {
		f.streamNonGroundedAnswer = true
	}
}

func StreamToolActions() FilterOption {
	return func(f *filter) {
		f.streamToolActions = true
	}
}

func StreamProcessedParams() FilterOption {
	return func(f *filter) {
		f.streamProcessedParams = true
	}
}

// RemoveToken is used for disableEOS to remove the END_RESPONSE/END_TEXT token
func RemoveToken(token string) FilterOption {
	return func(f *filter) {
		delete(f.specialTokenMap, token)
	}
}

func (f *filter) mergeIntoSpecialTokenMap(toMerge map[string]filterMode) {
	for k, v := range toMerge {
		if _, ok := f.specialTokenMap[k]; ok {
			f.logger.Warn("Special token already exists in stream filter specialTokenMap", zap.String("key", k))
			continue
		}
		f.specialTokenMap[k] = v
	}
}
