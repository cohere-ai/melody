package parsing

import (
	"bytes"
	"errors"
	"strings"
	"unicode"
	"unicode/utf8"

	"go.uber.org/zap"
)

// Filter is the interface used to parse the output of a cohere model
type Filter interface {
	Write(token int64, likelihood *float32) ([]FilterOutput, error)
	WriteDecoded(decodedToken string) []FilterOutput
	FlushPartials() []FilterOutput
	GetRawTokens() []int64
}

func newF(logger *zap.Logger, tokenizer Decoder, opts ...FilterOption) *filter {
	if logger == nil {
		logger = zap.NewNop()
	}
	f := &filter{
		logger:               logger,
		tokenizer:            tokenizer,
		specialTokenMap:      make(map[string]filterMode),
		defaultMode:          plainText,
		curCitationByteIndex: -1,
		chunkSize:            1,
	}
	for _, opt := range opts {
		opt(f)
	}
	f.mode = f.defaultMode
	f.specialTokenKeys = keys(f.specialTokenMap)
	return f
}
func keys[T any](m map[string]T) []string {
	res := make([]string, 0, len(m))
	for k := range m {
		res = append(res, k)
	}
	return res
}

// NewFilter creates a new Filter with the given options.
func NewFilter(logger *zap.Logger, tokenizer Decoder, opts ...FilterOption) Filter {
	return newF(logger, tokenizer, opts...)
}

type filter struct {
	logger *zap.Logger

	tokenizer  Decoder
	tokenBuf   []int64
	logProbBuf []float32
	rawTokens  []int64 // raw tokens that have been written to the filter (so far)

	leftTrimmed                 bool
	rightTrimmed                bool
	trimPrefix                  string
	maxRepetitionLimit          int
	maxRepetitionSequenceLength int

	defaultMode             filterMode
	specialTokenMap         map[string]filterMode
	streamNonGroundedAnswer bool
	streamToolActions       bool
	streamProcessedParams   bool

	// rawParamIndentLengthRemoved is used to keep track how much indentation we have removed for the current
	// line of a tool call's raw parameter generation. Since `parameters` of the tool call generation
	// is always indented two levels deeper than the end user should see, we use this to remove the extra indentation.
	rawParamIndentLengthRemoved   int
	sawNonWhitespaceInCurrentline bool

	// The curTextIndex marks where in the text we are without the citation tags
	// For example "<co: 1>hello</co: 1> world" if we have sent "hello w" the index would be 6
	// This doesn't include the citation tags, note we have to use the utf8.RuneCountInString to
	// get the correct index for the end user BUT we also need curTextByteIndex so we can correctly index the string still.
	curTextIndex     int
	curTextByteIndex int
	// The curCitationByteIndex marks where in the text we are with the citation tags
	// For example "boo hoo<co: 1>hello</co: 1> world" if we have sent "boo hoo hel" the index would be 10
	// As it is "<co: 1>hel", we use it to keep track of what we have sent and what we will need to send
	// After each citation it is reset to -1
	curCitationByteIndex int
	actionMetaData       filterAction

	// The current search query index
	currSearchQueryIdx int
	sentCurrIndex      bool

	// Flag to indicate whether to look for tool_call_id before tool_name
	hasToolCallID bool

	// Flag to indicate whether using Cmd3 citations
	cmd3Citations bool

	llamaToolParsing bool

	chunkSize        int                 // number of tokens to chunk together before emitting FilterOutputs
	numTokensInChunk int                 // current number of tokens seen
	chunkLogProbs    TokenIDsWithLogProb // logprobs for the current chunk

	// filter state
	buf                        bytes.Buffer
	partialSpecialTokenLogProb TokenIDsWithLogProb
	mode                       filterMode
	specialTokenKeys           []string
	done                       bool
}

// GetRawTokens returns the raw tokens that have been written to the filter (so far).
func (f *filter) GetRawTokens() []int64 {
	return f.rawTokens
}

func (f *filter) DecodeToken(token int64, tokenLogProb *float32) (string, error) {
	f.tokenBuf = append(f.tokenBuf, token)
	text, err := f.tokenizer.Decode(f.tokenBuf, false)
	if err != nil {
		return "", err
	}
	if text == "" {
		f.logger.Warn("empty text from tokenizer", zap.Int64("token", token), zap.Int64s("tokens", f.tokenBuf))
	}

	if tokenLogProb == nil {
		return text, nil
	}

	f.logProbBuf = append(f.logProbBuf, *tokenLogProb)
	return text, nil
}

func (f *filter) getFullTextWithLogProbs(token int64, tokenLogProb *float32) (fulltextwithlogprobs, error) {
	f.rawTokens = append(f.rawTokens, token)
	// Check if the token is repeated too many times
	hasRepetitionLimits := f.maxRepetitionLimit > 0 && f.maxRepetitionSequenceLength > 0
	if hasRepetitionLimits && hasHitTokenRepetitionLimit(f.rawTokens, f.maxRepetitionLimit, f.maxRepetitionSequenceLength) {
		f.logger.Error(
			"too many repeated tokens in strict (guided generation) mode",
			zap.Int("maxRepetitionLimit", f.maxRepetitionLimit),
			zap.Int("maxRepetitionSequenceLength", f.maxRepetitionSequenceLength),
			zap.Int("raw_tokens_length", len(f.rawTokens)),
		)
		return fulltextwithlogprobs{}, errors.New("saw too many repeated tokens")
	}
	text, err := f.DecodeToken(token, tokenLogProb)
	if err != nil {
		return fulltextwithlogprobs{}, err
	}
	// multi token characters will decode into this string
	// more on the string: https://www.fileformat.info/info/unicode/char/fffd/index.htm
	if strings.HasSuffix(text, "\ufffd") {
		return fulltextwithlogprobs{}, nil
	}

	tokenBufCopy := make([]int64, len(f.tokenBuf))
	copy(tokenBufCopy, f.tokenBuf)
	f.tokenBuf = nil

	var logProbsCopy []float32
	if len(f.logProbBuf) != 0 {
		logProbsCopy = make([]float32, len(f.logProbBuf))
		copy(logProbsCopy, f.logProbBuf)
		f.logProbBuf = nil
	}
	return fulltextwithlogprobs{
		Text:     []byte(text),
		Logprobs: TokenIDsWithLogProb{TokenIDs: tokenBufCopy, Logprobs: logProbsCopy},
	}, nil
}

func (f *filter) Write(token int64, tokenLogProb *float32) ([]FilterOutput, error) {
	t, err := f.getFullTextWithLogProbs(token, tokenLogProb)
	if err != nil {
		return nil, err
	}
	return f.writeText(t.Text, t.Logprobs), nil
}
func (f *filter) WriteDecoded(decodedToken string) []FilterOutput {
	return f.writeText([]byte(decodedToken), TokenIDsWithLogProb{TokenIDs: []int64{}, Logprobs: []float32{}}) // no logprobs for decoded tokens
}

func (f *filter) writeText(text []byte, logprobs TokenIDsWithLogProb) (out []FilterOutput) {
	if f.done {
		return nil
	}
	f.buf.Write(text)
	str := f.buf.String()
	// If is a partial special token, we need to wait for the next token.
	specialTokenIdx, foundSeq := findPartial(str, f.specialTokenKeys)
	if specialTokenIdx != -1 && foundSeq == "" {
		f.partialSpecialTokenLogProb = logprobs
		return nil
	}

	// If it is a whole special token, change the mode, remove the tokens and continue
	if specialTokenIdx != -1 && foundSeq != "" {
		// Get the new mode based on the special token
		o, newMode, stop, validSpecial := f.handleSpecialToken(str, specialTokenIdx, foundSeq, f.mode)
		out = append(out, o...)
		if validSpecial {
			if stop {
				f.buf.Reset()
				f.done = true
				return out
			}
			// Before the special token, process the buffer with the old mode (there could have been a partial special token)
			preSpecialToken := str[:specialTokenIdx]
			if preSpecialToken != "" {
				o, _ = f.handleToken(f.mode, []byte(preSpecialToken), false, f.partialSpecialTokenLogProb)
				out = append(out, o...)
			}
			// Remove the special token and the text before
			_, _ = f.buf.Read(make([]byte, len(preSpecialToken)+len(foundSeq)))
			// Change mode
			f.mode = newMode
		}
	}
	// Process buffer by mode
	if f.buf.Len() > 0 {
		f.numTokensInChunk++
		f.chunkLogProbs.append(logprobs)
		if f.chunkSize > 1 {
			if f.numTokensInChunk < f.chunkSize {
				return out
			}
		}
		o, remove := f.handleToken(f.mode, f.buf.Bytes(), false, f.chunkLogProbs)
		out = append(out, o...)
		_, _ = f.buf.Read(make([]byte, remove))
		f.numTokensInChunk = 0
		f.chunkLogProbs = TokenIDsWithLogProb{}
	}
	return out
}

// FlushPartials implies that no more tokens will be written to the filter
// and it is safe to flush any partial special tokens we have buffered.
// This only has implications if the stream does not end with a EOS token.
func (f *filter) FlushPartials() []FilterOutput {
	f.done = true
	// If there was a partial special token at the end, process it
	if f.buf.Len() > 0 && f.mode != inclusiveStop && f.mode != exclusiveStop {
		o, remove := f.handleToken(f.mode, f.buf.Bytes(), true, f.partialSpecialTokenLogProb)
		_, _ = f.buf.Read(make([]byte, remove))
		return o
	}
	return nil
}

// handleToken processes the token based on the mode
// Each helper function sends the results to the out channel in the filter
// And returns the number of bytes to remove from the buffer
func (f *filter) handleToken(
	mode filterMode,
	bstr []byte,
	afterLastToken bool,
	tokenLogProbs TokenIDsWithLogProb) ([]FilterOutput, int) {
	switch mode {
	case inclusiveStop, exclusiveStop:
		f.logger.Error("in stop mode but we should have already stopped")
		return nil, 0
	case ignore:
		return nil, 0
	case toolAction:
		return f.ParseActions(string(bstr))
	case groundedAnswer, toolReason:
		return f.processGroundedText(bstr, afterLastToken, mode, &tokenLogProbs)
	case searchQuery:
		return f.processSearchQuery(bstr)
	case answer:
		if f.streamNonGroundedAnswer {
			return f.processText(bstr, nil)
		}
		// If we don't stream the answer just remove all the bytes
		return nil, len(bstr)
	case plainText:
		return f.processText(bstr, &tokenLogProbs)
	}
	return nil, 0
}

func (f *filter) handleInclusiveStop(str string, idx int, token string) []FilterOutput {
	// Stop with the inclusive stop token
	if idx != -1 && str[:idx+len(token)] != "" {
		if f.curCitationByteIndex != -1 {
			// don't resend what has been sent
			return []FilterOutput{{Text: str[f.curCitationByteIndex : idx+len(token)]}}
		}
		return []FilterOutput{{Text: str[:idx+len(token)]}}
	}
	return nil
}

func (f *filter) handleExclusiveStop(str string, idx int) []FilterOutput {
	// Stop without the exclusive stop token
	if idx != -1 && str[:idx] != "" {
		var text string
		if f.curCitationByteIndex != -1 {
			// don't resend what has been sent
			text, _ = f.trimSpace(str[f.curCitationByteIndex:idx])
		} else {
			text, _ = f.trimSpace(str[:idx])
		}
		return []FilterOutput{{Text: text}}
	}
	return nil
}

func (f *filter) handleSpecialToken(str string, idx int, token string, curMode filterMode) ([]FilterOutput, filterMode, bool, bool) {
	newMode := f.specialTokenMap[token]
	// Disable mode change if in grounded answer or answer mode and see "answer:" in the text
	notSpecial := (curMode == groundedAnswer || curMode == answer) && newMode == answer
	if notSpecial {
		return nil, curMode, false, false
	}
	switch newMode {
	case inclusiveStop:
		out := f.handleInclusiveStop(str, idx, token)
		return out, newMode, true, true
	case exclusiveStop:
		out := f.handleExclusiveStop(str, idx)
		return out, newMode, true, true
	case groundedAnswer:
		f.curTextIndex = 0 // Reset the curTextIndex so citations aren't offset by the length of plans/reflections
		if f.streamNonGroundedAnswer {
			// Reset from the "answer" to left trim the grounded answer
			f.leftTrimmed = true
		}
	case toolReason:
		f.leftTrimmed = true
		f.rightTrimmed = true
	case answer:
		f.leftTrimmed = true
	case searchQuery:
		f.leftTrimmed = true
	case nextSearchQuery:
		f.leftTrimmed = true
		if f.sentCurrIndex {
			f.currSearchQueryIdx++
			f.sentCurrIndex = false
		}
		return nil, searchQuery, false, true
	}

	return nil, newMode, false, true
}

func (f *filter) utf8ValidOrLimit(bstr []byte) bool {
	limit := 4 // utf-8 is up to 4 bytes
	valid := utf8.Valid(bstr)
	if len(bstr) >= limit && !valid {
		f.logger.Warn("emitting invalid utf8", zap.Binary("text", bstr))
	}
	return valid || len(bstr) >= limit
}

func (f *filter) processSearchQuery(bstr []byte) ([]FilterOutput, int) {
	// This should be handled in StreamFilter.Write , but left as safety
	if !f.utf8ValidOrLimit(bstr) {
		return nil, 0
	}

	// Trim space and send text
	send, remRight := f.trimSpace(string(bstr))
	var out []FilterOutput
	if send != "" {
		out = []FilterOutput{
			{SearchQuery: &FilterSearchQueryDelta{
				Index: f.currSearchQueryIdx,
				Text:  send,
			}}}
		f.sentCurrIndex = true
	}
	// We don't remove the remRight incase we aren't at the end
	return out, len(bstr) - remRight
}

// processGroundedText processes the grounded text and sends it to the out channel
// grounded text can continue citations such as <co: 1>
// If we see a partial citation, we will wait to get the whole citation
// However if we have seen all tokens e.g. afterLastToken then we will not find a whole citation so send all the text
func (f *filter) processGroundedText(
	bstr []byte,
	afterLastToken bool,
	mode filterMode,
	tokenLogProbs *TokenIDsWithLogProb) ([]FilterOutput, int) {
	// This should be handled in StreamFilter.Write , but left as safety
	if !f.utf8ValidOrLimit(bstr) {
		return nil, 0
	}
	send := string(bstr)
	send, remRight := f.trimSpace(send)
	remove := len(bstr) - len(send) - remRight
	resOut, removeCit := f.ParseCitations(send, mode)
	if resOut == nil || (resOut.Text == "" && resOut.Citations == nil) {
		// If it is after the last token, and we don't find citations then
		//  we want to send all the text as there won't be the end of the citation
		if send == "" || !afterLastToken {
			return nil, remove + removeCit
		}
		resOut = &FilterOutput{Text: send}
	}
	resOut.IsPostAnswer = f.streamNonGroundedAnswer && mode != toolReason
	resOut.IsToolsReason = mode == toolReason

	// Don't send logprobs for citations if there's no corresponding text.
	if tokenLogProbs != nil && (resOut.Citations == nil || resOut.Text != "") {
		resOut.Logprobs = *tokenLogProbs
	}

	var out []FilterOutput
	if f.streamToolActions && resOut.IsToolsReason || !resOut.IsToolsReason {
		out = []FilterOutput{*resOut}
	}
	return out, remove + removeCit
}

func (f *filter) processText(bstr []byte, tokenLogProbs *TokenIDsWithLogProb) ([]FilterOutput, int) {
	// This should be handled in StreamFilter.Write , but left as safety
	if !f.utf8ValidOrLimit(bstr) {
		return nil, 0
	}
	// Trim space and send text
	send, remRight := f.trimSpace(string(bstr))
	var out []FilterOutput
	if send != "" {
		if tokenLogProbs != nil {
			out = []FilterOutput{{Text: send, Logprobs: *tokenLogProbs}}
		} else {
			out = []FilterOutput{{Text: send}}
		}
	}
	// We don't remove the remRight incase we aren't at the end
	return out, len(bstr) - remRight
}

// returns trimmed string and number of trimmed right characters
func (f *filter) trimSpace(s string) (string, int) {
	rem := 0
	if f.rightTrimmed {
		rem = len(s)
		s = strings.TrimRightFunc(s, unicode.IsSpace)
		rem -= len(s)
	}

	if f.leftTrimmed {
		s = strings.TrimLeftFunc(s, unicode.IsSpace)
		if s != "" {
			// remember if left is already trimmed
			f.leftTrimmed = false
		}
	}

	if f.trimPrefix != "" {
		// if prefix longer shorten it
		prefix := f.trimPrefix
		if len(s) < len(f.trimPrefix) {
			prefix = f.trimPrefix[:len(s)]
		}
		if strings.HasPrefix(s, prefix) {
			if len(prefix) == len(f.trimPrefix) {
				// full match, forget the prefix
				f.trimPrefix = ""
				return s[len(prefix):], rem
			}
			// partial match, keep the prefix
			return "", len(s) + rem
		}
		// no match at all, forget the prefix
		f.trimPrefix = ""
	}
	return s, rem
}

// findPartial returns first index in str that might match one of stop sequences.
// If the string in the response is filled, then we found the whole stop sequence
// If the string is empty but the int is not -1, we found a partial stop sequence
func findPartial(str string, stops []string) (int, string) {
	minIdx := -1
	// Go through all the stops
	for _, stop := range stops {
		// If we find the stop sequence, return the index and the stop sequence
		if idx := strings.Index(str, stop); idx >= 0 {
			return idx, stop
		}
		// Go through the substrings of the stop sequence
		for i := 0; i < len(str); i++ {
			suffix := stop
			if len(stop) > len(str)-i {
				suffix = stop[:len(str)-i]
			}
			// If we find the partial stop sequence, return the index
			if strings.HasSuffix(str, suffix) {
				idx := len(str) - len(suffix)
				if minIdx < 0 || minIdx > idx {
					minIdx = idx
				}
				break
			}
		}
	}
	return minIdx, ""
}

// hashTokensForRepetitionCheck is essentially a DJB2 hash function
// very fast, zero allocations, and simple, but not cryptographically secure
// source: https://gist.github.com/lmas/664afa94f922c1e58d5c3d73aed98f3f
func hashTokensForRepetitionCheck(seq []int64) uint64 {
	var hash uint64 = 5381
	for _, v := range seq {
		hash = hash*33 + uint64(v)
	}
	return hash
}

func hasHitTokenRepetitionLimit(seenTokens []int64, repetitionLimit int, maxSequenceLength int) bool {
	/*
		Checks if the last repetitionLimit tokens, or any combination of number of tokens up to maxSequenceLength tokens are the same.
		Examples:
		- If repetitionLimit is 3, and maxSequenceLength is 1:
			- [1, 2, 3, 4, 5, 6] -> false
			- [1, 2, 3, 4, 4, 4] -> true
			- [1, 2, 1, 2, 1, 2] -> false (because maxSequenceLength is only 1)
		- If repetitionLimit is 3, and maxSequenceLength is 2:
			- [1, 2, 3, 4, 5, 6] -> false
			- [1, 2, 3, 4, 4, 4] -> true
			- [1, 2, 1, 2, 1, 2] -> true (because maxSequenceLength here is 2, so the pair (1, 2) is repeated)
	*/
	if len(seenTokens) <= repetitionLimit {
		return false
	}
	// Check the largest sequence length we can support based on the number of tokens we have seen
	maxPossibleSeqLen := len(seenTokens) / repetitionLimit
	if maxSequenceLength > maxPossibleSeqLen {
		maxSequenceLength = maxPossibleSeqLen
	}

	// Check all possible sequence lengths up to maxSequenceLength
	for seqLen := 1; seqLen <= maxSequenceLength; seqLen++ {
		start := len(seenTokens) - repetitionLimit*seqLen
		tokens := seenTokens[start:]

		var firstHash uint64
		mismatch := false

		for i := range repetitionLimit {
			offset := i * seqLen
			h := hashTokensForRepetitionCheck(tokens[offset : offset+seqLen])
			if i == 0 {
				firstHash = h
			} else if h != firstHash {
				mismatch = true
				break
			}
		}

		if !mismatch {
			return true
		}
	}

	return false
}
