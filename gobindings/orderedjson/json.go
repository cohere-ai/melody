package orderedjson

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
