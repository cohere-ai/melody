package orderedjson

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"iter"
	"strconv"
	"strings"
	"unicode/utf8"

	"github.com/buger/jsonparser"
	"github.com/mailru/easyjson/jwriter"
)

type Pair struct {
	Key   string
	Value any
}

type Object struct {
	pairs map[string]Pair
	order []string
}

type InitOption func(*Object)

func WithInitialData(pairs ...Pair) InitOption {
	return func(o *Object) {
		for _, pair := range pairs {
			o.Set(pair.Key, pair.Value)
		}
	}
}

func New(opts ...InitOption) Object {
	obj := &Object{
		pairs: make(map[string]Pair),
		order: make([]string, 0),
	}
	for _, opt := range opts {
		opt(obj)
	}
	return *obj
}

func (o *Object) Pairs() iter.Seq2[string, any] {
	return func(yield func(string, any) bool) {
		for _, key := range o.order {
			pair := o.pairs[key]
			if !yield(key, pair.Value) {
				return
			}
		}
	}
}

func (o *Object) Keys() []string {
	return o.order
}

func (o *Object) Len() int {
	if o == nil || o.pairs == nil {
		return 0
	}
	return len(o.order)
}

func (o *Object) Contains(key string) bool {
	if o.pairs == nil {
		return false
	}
	_, present := o.pairs[key]
	return present
}

func (o *Object) Get(key string) (any, bool) {
	if o.pairs == nil {
		return nil, false
	}
	pair, present := o.pairs[key]
	return pair.Value, present
}

func (o *Object) Delete(key string) {
	if !o.Contains(key) {
		return
	}
	delete(o.pairs, key)
	for i, k := range o.order {
		if k == key {
			o.order = append(o.order[:i], o.order[i+1:]...)
			break
		}
	}
}

func (o *Object) Set(key string, value any) {
	if o.pairs == nil {
		o.pairs = make(map[string]Pair)
	}
	if o.Contains(key) {
		o.pairs[key] = Pair{
			Key:   key,
			Value: value,
		}
	} else {
		p := Pair{
			Key:   key,
			Value: value,
		}
		o.pairs[key] = p
		o.order = append(o.order, key)
	}
}

func (o *Object) ToMap() map[string]any {
	m := make(map[string]any)
	for _, key := range o.order {
		val := o.pairs[key].Value
		if obj, ok := val.(Object); ok {
			m[key] = obj.ToMap()
			continue
		}
		m[key] = o.pairs[key].Value
	}
	return m
}

var (
	_ json.Marshaler   = &Object{}
	_ json.Unmarshaler = &Object{}
)

func (o Object) MarshalJSON() ([]byte, error) {
	if o.pairs == nil {
		return []byte("{}"), nil
	}

	writer := jwriter.Writer{
		NoEscapeHTML: true,
	}
	writer.RawByte('{')

	firstIteration := true
	for _, key := range o.order {
		if firstIteration {
			firstIteration = false
		} else {
			writer.RawByte(',')
		}

		pair := o.pairs[key]
		writer.String(key)
		writer.RawByte(':')
		switch vt := pair.Value.(type) {
		case float64:
			formatFloat(&writer, vt)
		case float32:
			formatFloat(&writer, float64(vt))
		default:
			// disable html escaping
			var buf bytes.Buffer
			enc := json.NewEncoder(&buf)
			enc.SetEscapeHTML(false)
			err := enc.Encode(pair.Value)
			res := bytes.TrimSuffix(buf.Bytes(), []byte{'\n'})
			writer.Raw(res, err)
		}
	}
	writer.RawByte('}')

	return dumpWriter(&writer)
}

func formatFloat(writer *jwriter.Writer, f float64) {
	fStr := strconv.FormatFloat(f, 'g', -1, 64)
	// if no decimal add .0
	if !strings.Contains(fStr, ".") && !strings.Contains(fStr, "e") {
		fStr += ".0"
	}
	writer.RawString(fStr)
}

func dumpWriter(writer *jwriter.Writer) ([]byte, error) {
	if writer.Error != nil {
		return nil, writer.Error
	}

	var buf bytes.Buffer
	buf.Grow(writer.Size())
	if _, err := writer.DumpTo(&buf); err != nil {
		return nil, err
	}

	return buf.Bytes(), nil
}

func quoteBytes(b []byte) []byte {
	var buf bytes.Buffer
	buf.WriteByte('"')
	buf.Write(b)
	buf.WriteByte('"')
	return buf.Bytes()
}

func parseArray(arrData []byte) ([]any, error) {
	res := make([]any, 0)
	var firstErr error
	_, err := jsonparser.ArrayEach(arrData, func(elemData []byte, elemType jsonparser.ValueType, offset int, cbErr error) {
		if firstErr != nil {
			return
		}
		if cbErr != nil {
			firstErr = cbErr
			return
		}
		// restore enclosing quotes for strings to make valid JSON
		if elemType == jsonparser.String {
			elemData = quoteBytes(elemData)
		}
		v, e := parseValue(elemData, elemType)
		if e != nil {
			firstErr = e
			return
		}
		res = append(res, v)
	})
	if err != nil {
		return nil, err
	}
	if firstErr != nil {
		return nil, firstErr
	}
	return res, nil
}

func parseValue(vData []byte, vType jsonparser.ValueType) (any, error) {
	switch vType {
	case jsonparser.Object:
		obj := New()
		if err := obj.UnmarshalJSON(vData); err != nil {
			return nil, err
		}
		return obj, nil
	case jsonparser.Array:
		return parseArray(vData)
	case jsonparser.Number:
		if intVal, err := jsonparser.ParseInt(vData); err == nil {
			return intVal, nil
		}
		if floatVal, err := jsonparser.ParseFloat(vData); err == nil {
			return floatVal, nil
		}
		return nil, errors.New("invalid numeric value")
	default:
		var value interface{}
		if err := json.Unmarshal(vData, &value); err != nil {
			return nil, err
		}
		return value, nil
	}
}

func (o *Object) UnmarshalJSON(data []byte) error {
	if o == nil {
		return errors.New("orderedjson.Object: UnmarshalJSON on nil pointer")
	}
	if o.pairs == nil {
		*o = New()
	}
	if bytes.Equal(bytes.TrimSpace(data), []byte("null")) {
		*o = Object{}
		return nil
	}
	{
		var m map[string]any
		if err := json.Unmarshal(data, &m); err != nil {
			return err
		}
	}
	return jsonparser.ObjectEach(data,
		func(keyData []byte, valueData []byte, dataType jsonparser.ValueType, offset int) error {
			if dataType == jsonparser.String {
				// jsonparser removes the enclosing quotes; we need to restore them to make a valid JSON
				valueData = quoteBytes(valueData)
			}

			key, err := decodeUTF8(keyData)
			if err != nil {
				return err
			}

			val, err := parseValue(valueData, dataType)
			if err != nil {
				return err
			}
			o.Set(key, val)
			return nil
		})
}

func decodeUTF8(input []byte) (string, error) {
	remaining, offset := input, 0
	runes := make([]rune, 0, len(remaining))

	for len(remaining) > 0 {
		r, size := utf8.DecodeRune(remaining)
		if r == utf8.RuneError && size <= 1 {
			return "", fmt.Errorf("not a valid UTF-8 string (at position %d): %s", offset, string(input))
		}

		runes = append(runes, r)
		remaining = remaining[size:]
		offset += size
	}

	return string(runes), nil
}
