package tokenizers

import (
	"fmt"
	"sync"

	hftokenizers "github.com/cohere-ai/tokenizers"

	"github.com/cohere-ai/melody/_internal/vectormath"
)

// Same as custom_tokenizer but calling the tokenizer natively
// instead of over network.
// Both still exist to be able to flight the new implementation.
type hfTokenizer struct {
	cfg         []byte
	tokenizerID string
	tokenizer   *hftokenizers.Tokenizer

	truncTkzrMux sync.RWMutex
	truncTkzrs   map[string]*hftokenizers.Tokenizer
}

var _ Tokenizer = (*hfTokenizer)(nil)

func newHuggingFaceTokenizer(tokenizerID string, cfgData []byte) (Tokenizer, error) {
	tk, err := hftokenizers.FromBytes(cfgData)
	return &hfTokenizer{
		cfg:         cfgData,
		tokenizerID: tokenizerID,
		tokenizer:   tk,

		truncTkzrMux: sync.RWMutex{},
		truncTkzrs:   make(map[string]*hftokenizers.Tokenizer),
	}, err
}

func (t *hfTokenizer) truncatingTokenizer(params *encodeParams) (*hftokenizers.Tokenizer, error) {
	mapID := fmt.Sprintf("%s-%d-%v", t.tokenizerID, *params.truncationLength, *params.truncationMode)
	t.truncTkzrMux.RLock()
	tk, ok := t.truncTkzrs[mapID]
	t.truncTkzrMux.RUnlock()
	if ok {
		return tk, nil
	}
	t.truncTkzrMux.Lock()
	defer t.truncTkzrMux.Unlock()
	// handle 2nd goroutine to get the lock
	if tk, ok = t.truncTkzrs[mapID]; ok {
		return tk, nil
	}
	tk, err := hftokenizers.FromBytesWithTruncation(t.cfg, uint32(*params.truncationLength), *params.truncationMode)
	if err != nil {
		return nil, fmt.Errorf("failed to create truncated tokenizer: %w", err)
	}
	t.truncTkzrs[mapID] = tk
	return tk, nil
}

func (t *hfTokenizer) ID() string {
	return t.tokenizerID
}

func (t *hfTokenizer) Close() {
	// This package caches tokenizers so we don't want to close it
	// because it's expensive to create new instance of tokenizer.
	// Callers should not call Close() at all, but that still required
	// for CustomTokenizer implementation.
}

func (t *hfTokenizer) getTokenizerWithParams(opts ...EncodeOption) (*hftokenizers.Tokenizer, bool, error) {
	params := &encodeParams{
		addSpecialTokens: true,
	}
	for _, opt := range opts {
		opt(params)
	}
	tokenizer := t.tokenizer
	if params.truncationLength != nil && params.truncationMode != nil {
		var err error
		tokenizer, err = t.truncatingTokenizer(params)
		if err != nil {
			return nil, false, fmt.Errorf("failed to create truncated tokenizer: %w", err)
		}
	}
	return tokenizer, params.addSpecialTokens, nil
}

func (t *hfTokenizer) Encode(text string, opts ...EncodeOption) ([]int64, error) {
	tokenizer, addSpecialTokens, err := t.getTokenizerWithParams(opts...)
	if err != nil {
		return nil, err
	}
	enc := tokenizer.EncodeWithOptions(text, addSpecialTokens)
	return vectormath.Convert[int64](enc.IDs), nil
}

func (t *hfTokenizer) EncodeUint32(text string, opts ...EncodeOption) ([]uint32, error) {
	tokenizer, addSpecialTokens, err := t.getTokenizerWithParams(opts...)
	if err != nil {
		return nil, err
	}
	// important! use tokenizer.EncodeWithOptions to avoid allocating superfluous data
	enc := tokenizer.EncodeWithOptions(text, addSpecialTokens)
	return enc.IDs, nil
}

func (t *hfTokenizer) EncodeBatch(texts []string, opts ...EncodeOption) ([][]int64, error) {
	tokenizer, addSpecialTokens, err := t.getTokenizerWithParams(opts...)
	if err != nil {
		return nil, err
	}
	encs := make([][]int64, len(texts))
	for i, text := range texts {
		// important! use tokenizer.EncodeWithOptions to avoid allocating superfluous data
		enc := tokenizer.EncodeWithOptions(text, addSpecialTokens)
		encs[i] = vectormath.Convert[int64](enc.IDs)
	}
	return encs, nil
}

func (t *hfTokenizer) EncodeBatchUint32(texts []string, opts ...EncodeOption) ([][]uint32, error) {
	tokenizer, addSpecialTokens, err := t.getTokenizerWithParams(opts...)
	if err != nil {
		return nil, err
	}
	encs := make([][]uint32, len(texts))
	for i, text := range texts {
		// important! use tokenizer.EncodeWithOptions to avoid allocating superfluous data
		enc := tokenizer.EncodeWithOptions(text, addSpecialTokens)
		encs[i] = enc.IDs
	}
	return encs, nil
}

func (t *hfTokenizer) Decode(ids []int64, skipSpecialTokens bool) (string, error) {
	return t.tokenizer.Decode(vectormath.Convert[uint32](ids), skipSpecialTokens), nil
}

func (t *hfTokenizer) DecodeUint32(ids []uint32, skipSpecialTokens bool) (string, error) {
	return t.tokenizer.Decode(ids, skipSpecialTokens), nil
}

func (t *hfTokenizer) DecodeBatch(ids [][]int64, skipSpecialTokens bool) ([]string, error) {
	decs := make([]string, len(ids))
	for i, id := range ids {
		decs[i] = t.tokenizer.Decode(vectormath.Convert[uint32](id), skipSpecialTokens)
	}
	return decs, nil
}

func (t *hfTokenizer) DecodeBatchUint32(ids [][]uint32, skipSpecialTokens bool) ([]string, error) {
	decs := make([]string, len(ids))
	for i, id := range ids {
		decs[i] = t.tokenizer.Decode(id, skipSpecialTokens)
	}
	return decs, nil
}

func (t *hfTokenizer) VocabSize() (int64, error) {
	return int64(t.tokenizer.VocabSize()), nil
}
