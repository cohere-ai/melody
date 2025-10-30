package melody

import (
	"strings"

	"github.com/cohere-ai/melody/_internal/vectormath"
)

type Decoder interface {
	DecodeUint32(tokens []uint32, skipSpecialTokens bool) (string, error)
}

type fulltextwithlogprobs struct {
	Text     []byte
	Logprobs TokenIDsWithLogProb
}

// StreamFilter is the interface for parsing a cohere model generation through channels.
type StreamFilter interface {
	Read() <-chan FilterOutput
	Write(token int64, likelihood *float32) error
	WriteDecoded(decodedToken string, prob TokenIDsWithLogProb)
	Close()
}

// NewStreamFilter creates a new StreamFilter with the given options.
func NewStreamFilter(tokenizer Decoder, opts ...FilterOption) StreamFilter {
	s := &streamFilter{
		filter:    NewFilter(opts...),
		tokenizer: tokenizer,
		in:        make(chan fulltextwithlogprobs, 1),
		out:       make(chan FilterOutput, 1),
	}
	go s.run()
	return s
}

var _ StreamFilter = (*streamFilter)(nil)

type streamFilter struct {
	filter    Filter
	tokenizer Decoder
	in        chan fulltextwithlogprobs
	out       chan FilterOutput

	tokenBuf   []uint32
	logProbBuf []float32
}

func (s *streamFilter) run() {
	defer close(s.out)
	defer func() {
		for range s.in { //nolint:revive
			// drain input channel if caller this hasn't noticed out channel is closed
		}
	}()
	for t := range s.in {
		filterOutputs := s.filter.WriteDecoded(string(t.Text), &t.Logprobs)
		for _, output := range filterOutputs {
			s.out <- output
		}
	}
	o := s.filter.FlushPartials()
	for _, output := range o {
		s.out <- output
	}
}

func (s *streamFilter) Read() <-chan FilterOutput {
	return s.out
}

func (s *streamFilter) Write(token int64, likelihood *float32) error {
	t, err := s.getFullTextWithLogProbs(token, likelihood)
	if err != nil {
		return err
	}
	s.in <- t
	return nil
}

func (s *streamFilter) WriteDecoded(decodedToken string, l TokenIDsWithLogProb) {
	s.in <- fulltextwithlogprobs{
		Text:     []byte(decodedToken),
		Logprobs: l,
	}
}

func (s *streamFilter) Close() {
	close(s.in)
}

func (f *streamFilter) getFullTextWithLogProbs(token int64, tokenLogProb *float32) (fulltextwithlogprobs, error) {
	text, err := f.decodeToken(token, tokenLogProb)
	if err != nil {
		return fulltextwithlogprobs{}, err
	}
	// multi token characters will decode into this string
	// more on the string: https://www.fileformat.info/info/unicode/char/fffd/index.htm
	if strings.HasSuffix(text, "\ufffd") {
		return fulltextwithlogprobs{}, nil
	}

	ret := fulltextwithlogprobs{
		Text: []byte(text),
		Logprobs: TokenIDsWithLogProb{
			TokenIDs: vectormath.Convert[uint32](f.tokenBuf),
			Logprobs: vectormath.Convert[float32](f.logProbBuf),
		},
	}
	f.tokenBuf = nil
	f.logProbBuf = nil
	return ret, nil
}

func (f *streamFilter) decodeToken(token int64, tokenLogProb *float32) (string, error) {
	f.tokenBuf = append(f.tokenBuf, uint32(token))
	text, err := f.tokenizer.DecodeUint32(f.tokenBuf, false)
	if err != nil {
		return "", err
	}

	if tokenLogProb == nil {
		return text, nil
	}

	f.logProbBuf = append(f.logProbBuf, *tokenLogProb)
	return text, nil
}
