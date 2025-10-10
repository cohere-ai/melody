package parsing

import "go.uber.org/zap"

type FilterOption func(*filter)

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

func WithRepetitionLimit(limit int, maxSequenceLength int) FilterOption {
	// Set a hard limit to how many repetitions of the same sequence we allow before sending an error.
	return func(f *filter) {
		f.maxRepetitionLimit = limit
		f.maxRepetitionSequenceLength = maxSequenceLength
	}
}

func WithInclusiveStops(inclusiveStops ...string) FilterOption {
	return func(f *filter) {
		for _, stop := range inclusiveStops {
			if _, ok := f.specialTokenMap[stop]; !ok {
				// Only add if it's not already in the special token map
				f.specialTokenMap[stop] = InclusiveStop
			} else {
				f.logger.Warn("ignoring inclusive stop: already exists in specialTokenMap", zap.String("stop", stop))
			}
		}
	}
}

func WithExclusiveStops(exclusiveStops ...string) FilterOption {
	return func(f *filter) {
		for _, stop := range exclusiveStops {
			if _, ok := f.specialTokenMap[stop]; !ok {
				// Only add if it's not already in the special token map
				f.specialTokenMap[stop] = ExclusiveStop
			} else {
				f.logger.Warn("ignoring exclusive stop: already exists in specialTokenMap", zap.String("stop", stop))
			}
		}
	}
}

func WithLeftTrimmed() FilterOption {
	return func(f *filter) {
		f.leftTrimmed = true
	}
}

func WithRightTrimmed() FilterOption {
	return func(f *filter) {
		f.rightTrimmed = true
	}
}

func WithPrefixTrim(prefix string) FilterOption {
	return func(f *filter) {
		f.trimPrefix = prefix
	}
}

func HandleRag() FilterOption {
	return func(f *filter) {
		f.defaultMode = Ignore
		f.rightTrimmed = true
		// Need to create a copy as we will modify the map
		f.mergeIntoSpecialTokenMap(ragTokenMap)
	}
}

func HandleSearchQuery() FilterOption {
	return func(f *filter) {
		f.defaultMode = Ignore
		f.rightTrimmed = true
		f.mergeIntoSpecialTokenMap(searchQueryTokenMap)
	}
}

// HandleMultiHop Temporary until the reader handles actions
func HandleMultiHop() FilterOption {
	return func(f *filter) {
		f.defaultMode = Ignore
		f.rightTrimmed = true
		f.mergeIntoSpecialTokenMap(multiHopTokenMap)
	}
}

func HandleMultiHopCmd3() FilterOption {
	return func(f *filter) {
		// default needs to be grounded since response sometimes does not start with <|START_RESPONSE|>
		f.defaultMode = GroundedAnswer
		f.rightTrimmed = true
		f.hasToolCallID = true
		f.mergeIntoSpecialTokenMap(multiHopTokenMapCmd3)
		f.cmd3Citations = true
	}
}

func HandleMultiHopCmd4() FilterOption {
	return func(f *filter) {
		// default needs to be grounded since response sometimes does not start with <|START_RESPONSE|>
		f.defaultMode = GroundedAnswer
		f.rightTrimmed = true
		f.hasToolCallID = true
		f.mergeIntoSpecialTokenMap(multiHopTokenMapCmd4)
		f.cmd3Citations = true
	}
}

func HandleLlama() FilterOption {
	return func(f *filter) {
		f.defaultMode = GroundedAnswer
		f.rightTrimmed = true
		f.mergeIntoSpecialTokenMap(llamaTokenMap)
		f.llamaToolParsing = true
	}
}

func StreamNonGroundedAnswer() FilterOption { //nolint:revive
	return func(f *filter) {
		f.streamNonGroundedAnswer = true
	}
}

func StreamToolActions() FilterOption { //nolint:revive
	return func(f *filter) {
		f.streamToolActions = true
	}
}

func StreamProcessedParams() FilterOption { //nolint:revive
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

func (f *filter) mergeIntoSpecialTokenMap(toMerge map[string]FilterMode) {
	for k, v := range toMerge {
		if _, ok := f.specialTokenMap[k]; ok {
			f.logger.Warn("Special token already exists in stream filter specialTokenMap", zap.String("key", k))
			continue
		}
		f.specialTokenMap[k] = v
	}
}
