package melody

// Filter is the interface used to parse the output of a cohere model
type Filter interface {
	// WriteDecoded writes a decoded token string to the filter
	// For raw text processing
	WriteDecoded(decodedToken string, logprob *TokenIDsWithLogProb) []FilterOutput

	// FlushPartials flushes any partial outputs
	FlushPartials() []FilterOutput
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

	var cfilter *cFilter
	if cfg.multiHopCmd3 {
		cfilter = newCFilterMultiHopCmd3(cfg.streamToolActions)
	} else if cfg.multiHopCmd4 {
		cfilter = newCFilterMultiHopCmd4(cfg.streamToolActions)
	} else if cfg.rag {
		cfilter = newCFilterRAG()
	} else {
		cfilter = newCFilter()
	}

	if cfilter == nil {
		return nil
	}

	return &SyncFilter{
		cfilter: cfilter,
	}
}

// WriteDecoded writes a decoded token string to the filter
func (f *SyncFilter) WriteDecoded(decodedToken string, logprob *TokenIDsWithLogProb) []FilterOutput {
	if f.cfilter == nil {
		return nil
	}

	var lp TokenIDsWithLogProb
	if logprob != nil {
		lp = *logprob
	}

	return f.cfilter.writeDecoded(decodedToken, lp)
}

// FlushPartials flushes any partial outputs
func (f *SyncFilter) FlushPartials() []FilterOutput {
	if f.cfilter == nil {
		return nil
	}

	return f.cfilter.flushPartials()
}
