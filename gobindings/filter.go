package gobindings

// Filter is the interface used to parse the output of a cohere model
type Filter interface {
	// WriteDecoded writes a decoded token string to the filter
	// For raw text processing
	WriteDecoded(decodedToken string, logprob *TokenIDsWithLogProb) ([]FilterOutput, error)

	// FlushPartials flushes any partial outputs
	FlushPartials() ([]FilterOutput, error)
}

// SyncFilter is a synchronous filter implementation
type SyncFilter struct {
	cfilter *cFilter
}

// NewFilter creates a new synchronous filter
func NewFilter(options ...FilterOption) Filter {
	cfg := &filterConfig{}
	for _, opt := range options {
		opt(cfg)
	}

	// Build FilterOptions using the builder pattern
	opts := NewFilterOptions()
	if opts == nil {
		return nil
	}

	// Apply configuration
	cfg.apply(opts)

	// Create filter with configured options
	cfilter := newCFilter(opts)
	if cfilter == nil {
		return nil
	}

	return &SyncFilter{
		cfilter: cfilter,
	}
}

// WriteDecoded writes a decoded token string to the filter
func (f *SyncFilter) WriteDecoded(decodedToken string, logprob *TokenIDsWithLogProb) ([]FilterOutput, error) {
	if f.cfilter == nil {
		return nil, nil
	}

	var lp TokenIDsWithLogProb
	if logprob != nil {
		lp = *logprob
	}

	return f.cfilter.writeDecoded(decodedToken, lp)
}

// FlushPartials flushes any partial outputs
func (f *SyncFilter) FlushPartials() ([]FilterOutput, error) {
	if f.cfilter == nil {
		return nil, nil
	}

	return f.cfilter.flushPartials()
}
