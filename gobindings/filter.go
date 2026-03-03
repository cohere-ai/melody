package gobindings

// Filter is the interface used to parse the output of a cohere model
type Filter interface {
	// WriteDecoded writes a decoded token string to the filter
	WriteDecoded(decodedToken string) (*AggregatedResult, error)

	// FlushPartials flushes any partial outputs
	FlushPartials() (*AggregatedResult, error)
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
func (f *SyncFilter) WriteDecoded(decodedToken string) (*AggregatedResult, error) {
	if f.cfilter == nil {
		return nil, nil
	}

	return f.cfilter.writeDecoded(decodedToken)
}

// FlushPartials flushes any partial outputs
func (f *SyncFilter) FlushPartials() (*AggregatedResult, error) {
	if f.cfilter == nil {
		return nil, nil
	}

	return f.cfilter.flushPartials()
}
