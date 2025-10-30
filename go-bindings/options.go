package melody

// FilterOption is a function that configures a filter
type FilterOption func(*filterConfig)

// filterConfig holds the configuration for creating a filter
type filterConfig struct {
	multiHopCmd3      bool
	multiHopCmd4      bool
	rag               bool
	streamToolActions bool
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
