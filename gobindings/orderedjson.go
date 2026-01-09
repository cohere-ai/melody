package gobindings

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
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

// Pairs returns an iterator, enable when we upgrade to Go 1.23
// func (o *Object) Pairs() iter.Seq2[string, any] {
//	return func(yield func(string, any) bool) {
//		for _, key := range o.order {
//			pair := o.pairs[key]
//			if !yield(key, pair.Value) {
//				return
//			}
//		}
//	}
// }

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
	_, present := o.pairs[key]
	return present
}

func (o *Object) Get(key string) (any, bool) {
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
		return []byte("null"), nil
	}

	writer := jwriter.Writer{}
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
			writer.Raw(json.Marshal(pair.Value))
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

func (o *Object) UnmarshalJSON(data []byte) error {
	if o == nil || o.pairs == nil {
		*o = New()
	}

	return jsonparser.ObjectEach(data,
		func(keyData []byte, valueData []byte, dataType jsonparser.ValueType, offset int) error {
			if dataType == jsonparser.String {
				// jsonparser removes the enclosing quotes; we need to restore them to make a valid JSON
				valueData = data[offset-len(valueData)-2 : offset]
			}

			key, err := decodeUTF8(keyData)
			if err != nil {
				return err
			}
			// if value is an object, unmarshal it into a nested Object
			switch dataType {
			case jsonparser.Object:
				obj := New()
				if err := obj.UnmarshalJSON(valueData); err != nil {
					return err
				}
				o.Set(key, obj)
			case jsonparser.Number:
				if intVal, err := jsonparser.ParseInt(valueData); err == nil {
					o.Set(key, intVal)
					return nil
				}
				if floatVal, err := jsonparser.ParseFloat(valueData); err == nil {
					o.Set(key, floatVal)
					return nil
				}
				// neither float nor int, dafuq?
				return errors.New("invalid numeric value")
			default:
				var value interface{}
				if err := json.Unmarshal(valueData, &value); err != nil {
					return err
				}
				o.Set(key, value)
			}
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
