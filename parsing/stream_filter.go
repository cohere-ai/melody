package parsing

import (
	"go.uber.org/zap"
)

// StreamFilter accepts raw bytes and outputs a filtered stream of strings.
type StreamFilter interface { //nolint:revive
	Read() <-chan FilterOutput
	Write(token int64, likelihood *float32) error
	WriteDecoded(decodedToken string)
	Close()
	GetRawTokens() []int64
}

func NewStreamFilter(logger *zap.Logger, tokenizer Decoder, opts ...FilterOption) StreamFilter {
	s := &streamFilter{
		filter: *newF(logger, tokenizer, opts...),
		in:     make(chan fulltextwithlogprobs, 1),
		out:    make(chan FilterOutput, 1),
	}
	go s.run()
	return s
}

var _ StreamFilter = (*streamFilter)(nil)

type streamFilter struct {
	filter filter
	in     chan fulltextwithlogprobs
	out    chan FilterOutput
}

func (s *streamFilter) run() {
	defer close(s.out)
	defer func() {
		for range s.in { //nolint:revive
			// drain input channel if caller this hasn't noticed out channel is closed
		}
	}()
	for t := range s.in {
		filterOutputs := s.filter.writeText(t.Text, t.Logprobs)
		for _, output := range filterOutputs {
			s.out <- output
		}
	}
	o := s.filter.flushPartials()
	for _, output := range o {
		s.out <- output
	}
}

func (s *streamFilter) Read() <-chan FilterOutput {
	return s.out
}

func (s *streamFilter) Write(token int64, likelihood *float32) error {
	t, err := s.filter.getFullTextWithLogProbs(token, likelihood)
	if err != nil {
		return err
	}
	s.in <- t
	return nil
}

func (s *streamFilter) WriteDecoded(decodedToken string) {
	s.in <- fulltextwithlogprobs{
		Text: []byte(decodedToken),
	}
}

func (s *streamFilter) Close() {
	close(s.in)
}

func (s *streamFilter) GetRawTokens() []int64 {
	return s.filter.GetRawTokens()
}
