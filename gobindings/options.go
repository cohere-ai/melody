package gobindings

// FilterOption is a function that configures a filter
type FilterOption func(*filterConfig)

// filterConfig holds the configuration for creating a filter
type filterConfig struct {
	multiHopCmd3            bool
	multiHopCmd4            bool
	rag                     bool
	searchQuery             bool
	multiHop                bool
	streamToolActions       bool
	streamNonGroundedAnswer bool
	streamProcessedParams   bool
	leftTrimmed             bool
	rightTrimmed            bool
	prefixTrim              string
	chunkSize               int
	inclusiveStops          []string
	exclusiveStops          []string
	removeTokens            []string
}

// apply applies the configuration to the FilterOptions builder
func (cfg *filterConfig) apply(opts *FilterOptions) {
	// Handle format types
	if cfg.multiHopCmd3 {
		opts.Cmd3()
	}
	if cfg.multiHopCmd4 {
		opts.Cmd4()
	}
	if cfg.rag {
		opts.HandleRAG()
	}
	if cfg.searchQuery {
		opts.HandleSearchQuery()
	}
	if cfg.multiHop {
		opts.HandleMultiHop()
	}

	// Handle streaming options
	if cfg.streamToolActions {
		opts.StreamToolActions()
	}
	if cfg.streamNonGroundedAnswer {
		opts.StreamNonGroundedAnswer()
	}
	if cfg.streamProcessedParams {
		opts.StreamProcessedParams()
	}

	// Handle trimming options
	if cfg.leftTrimmed {
		opts.WithLeftTrimmed()
	}
	if cfg.rightTrimmed {
		opts.WithRightTrimmed()
	}

	// Handle size and limit options
	if cfg.chunkSize > 0 {
		opts.WithChunkSize(cfg.chunkSize)
	}

	// Handle stop sequences
	if len(cfg.inclusiveStops) > 0 {
		opts.WithInclusiveStops(cfg.inclusiveStops)
	}
	if len(cfg.exclusiveStops) > 0 {
		opts.WithExclusiveStops(cfg.exclusiveStops)
	}

	// Handle token removal
	for _, token := range cfg.removeTokens {
		opts.RemoveToken(token)
	}
}

// HandleMultiHopCmd3 configures the filter to handle multi-hop CMD3 format
func HandleMultiHopCmd3() FilterOption {
	return func(cfg *filterConfig) {
		cfg.multiHopCmd3 = true
	}
}

// HandleMultiHopCmd4 configures the filter to handle multi-hop CMD4 format
func HandleMultiHopCmd4() FilterOption {
	return func(cfg *filterConfig) {
		cfg.multiHopCmd4 = true
	}
}

// HandleRAG configures the filter to handle RAG (Retrieval Augmented Generation) format
func HandleRAG() FilterOption {
	return func(cfg *filterConfig) {
		cfg.rag = true
	}
}

// StreamToolActions enables streaming of tool actions
func StreamToolActions() FilterOption {
	return func(cfg *filterConfig) {
		cfg.streamToolActions = true
	}
}

// HandleSearchQuery configures the filter to handle search query format
func HandleSearchQuery() FilterOption {
	return func(cfg *filterConfig) {
		cfg.searchQuery = true
	}
}

// HandleMultiHop configures the filter to handle multi-hop format
func HandleMultiHop() FilterOption {
	return func(cfg *filterConfig) {
		cfg.multiHop = true
	}
}

// StreamNonGroundedAnswer enables streaming of non-grounded answer
func StreamNonGroundedAnswer() FilterOption {
	return func(cfg *filterConfig) {
		cfg.streamNonGroundedAnswer = true
	}
}

// StreamProcessedParams enables streaming of processed parameters
func StreamProcessedParams() FilterOption {
	return func(cfg *filterConfig) {
		cfg.streamProcessedParams = true
	}
}

// WithLeftTrimmed enables left trimming
func WithLeftTrimmed() FilterOption {
	return func(cfg *filterConfig) {
		cfg.leftTrimmed = true
	}
}

// WithRightTrimmed enables right trimming
func WithRightTrimmed() FilterOption {
	return func(cfg *filterConfig) {
		cfg.rightTrimmed = true
	}
}

// WithChunkSize sets the chunk size
func WithChunkSize(size int) FilterOption {
	return func(cfg *filterConfig) {
		cfg.chunkSize = size
	}
}

// WithInclusiveStops sets inclusive stop sequences
func WithInclusiveStops(stops []string) FilterOption {
	return func(cfg *filterConfig) {
		cfg.inclusiveStops = stops
	}
}

// WithExclusiveStops sets exclusive stop sequences
func WithExclusiveStops(stops []string) FilterOption {
	return func(cfg *filterConfig) {
		cfg.exclusiveStops = stops
	}
}

// RemoveToken removes a specific token from the output
func RemoveToken(token string) FilterOption {
	return func(cfg *filterConfig) {
		cfg.removeTokens = append(cfg.removeTokens, token)
	}
}
