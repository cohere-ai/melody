package main

import (
	"fmt"

	"gitlab.com/pygolo/py"

	"github.com/cohere-ai/melody"
	"github.com/cohere-ai/melody/lib/orderedjson"
	"github.com/cohere-ai/melody/parsing"
)

func pythonFilterToObject(Py py.Py, o any) (py.Object, error) {
	if pf, ok := o.(PythonFilter); ok {
		return Py.GoToObject(pf.ID)
	}
	return py.Object{}, fmt.Errorf("trying to convert non-PythonFilter Go Object to a PythonFilter")
}
func pythonFilterFromObject(Py py.Py, o py.Object, a any) error {
	v := a.(*PythonFilter)
	var s string
	if err := Py.GoFromObject(o, &s); err != nil {
		return fmt.Errorf("PythonFilter must be a string ID: %w", err)
	}
	*v = PythonFilter{ID: s}
	return nil
}

var pythonFilterConversion = py.GoConvConf{
	TypeOf:     PythonFilter{},
	ToObject:   pythonFilterToObject,
	FromObject: pythonFilterFromObject,
}

var roleConversion = py.GoConvConf{
	TypeOf:     melody.Role{},
	ToObject:   roleToObject,
	FromObject: roleFromObject,
}

func roleToObject(Py py.Py, o any) (py.Object, error) {
	if r, ok := o.(melody.Role); ok {
		return Py.GoToObject(r.String())
	}
	return py.Object{}, fmt.Errorf("trying to convert non-Role Go Object to a Python Role")
}
func roleFromObject(Py py.Py, o py.Object, a any) error {
	v := a.(*melody.Role)
	var s string
	if err := Py.GoFromObject(o, &s); err != nil {
		return fmt.Errorf("role must be a string: %w", err)
	}
	r, err := melody.RoleFromString(s)
	if err != nil {
		return err
	}
	*v = r
	return nil
}

var contentTypeConversion = py.GoConvConf{
	TypeOf:     melody.ContentType{},
	ToObject:   contentTypeToObject,
	FromObject: contentTypeFromObject,
}

func contentTypeToObject(Py py.Py, o any) (py.Object, error) {
	if ct, ok := o.(melody.ContentType); ok {
		return Py.GoToObject(ct.String())
	}
	return py.Object{}, fmt.Errorf("trying to convert non-ContentType Go Object to a Python ContentType")
}
func contentTypeFromObject(Py py.Py, o py.Object, a any) error {
	v := a.(*melody.ContentType)
	var s string
	if err := Py.GoFromObject(o, &s); err != nil {
		return fmt.Errorf("content type must be a string: %w", err)
	}
	ct, err := melody.ContentTypeFromString(s)
	if err != nil {
		return err
	}
	*v = ct
	return nil
}

var citationQualityConversion = py.GoConvConf{
	TypeOf:     melody.CitationQuality{},
	ToObject:   citationQualityToObject,
	FromObject: citationQualityFromObject,
}

func citationQualityToObject(Py py.Py, o any) (py.Object, error) {
	if cq, ok := o.(melody.CitationQuality); ok {
		return Py.GoToObject(cq.String())
	}
	return py.Object{}, fmt.Errorf("trying to convert non-CitationQuality Go Object to a Python CitationQuality")
}
func citationQualityFromObject(Py py.Py, o py.Object, a any) error {
	v := a.(*melody.CitationQuality)
	var s string
	if err := Py.GoFromObject(o, &s); err != nil {
		return fmt.Errorf("citation quality must be a string: %w", err)
	}
	cq, err := melody.CitationQualityFromString(s)
	if err != nil {
		return err
	}
	*v = cq
	return nil
}

var groundingConversion = py.GoConvConf{
	TypeOf:     melody.Grounding{},
	ToObject:   groundingToObject,
	FromObject: groundingFromObject,
}

func groundingToObject(Py py.Py, o any) (py.Object, error) {
	if g, ok := o.(melody.Grounding); ok {
		return Py.GoToObject(g.String())
	}
	return py.Object{}, fmt.Errorf("trying to convert non-Grounding Go Object to a Python Grounding")
}
func groundingFromObject(Py py.Py, o py.Object, a any) error {
	v := a.(*melody.Grounding)
	var s string
	if err := Py.GoFromObject(o, &s); err != nil {
		return fmt.Errorf("grounding must be a string: %w", err)
	}
	g, err := melody.GroundingFromString(s)
	if err != nil {
		return err
	}
	*v = g
	return nil
}

var safetyModeConversion = py.GoConvConf{
	TypeOf:     melody.SafetyMode{},
	ToObject:   safetyModeToObject,
	FromObject: safetyModeFromObject,
}

func safetyModeToObject(Py py.Py, o any) (py.Object, error) {
	if sm, ok := o.(melody.SafetyMode); ok {
		return Py.GoToObject(sm.String())
	}
	return py.Object{}, fmt.Errorf("trying to convert non-SafetyMode Go Object to a Python SafetyMode")
}
func safetyModeFromObject(Py py.Py, o py.Object, a any) error {
	v := a.(*melody.SafetyMode)
	var s string
	if err := Py.GoFromObject(o, &s); err != nil {
		return fmt.Errorf("safety mode must be a string: %w", err)
	}
	sm, err := melody.SafetyModeFromString(s)
	if err != nil {
		return err
	}
	*v = sm
	return nil
}

var reasoningTypeConversion = py.GoConvConf{
	TypeOf:     melody.ReasoningType{},
	ToObject:   reasoningTypeToObject,
	FromObject: reasoningTypeFromObject,
}

func reasoningTypeToObject(Py py.Py, o any) (py.Object, error) {
	if rt, ok := o.(melody.ReasoningType); ok {
		return Py.GoToObject(rt.String())
	}
	return py.Object{}, fmt.Errorf("trying to convert non-ReasoningType Go Object to a Python ReasoningType")
}
func reasoningTypeFromObject(Py py.Py, o py.Object, a any) error {
	v := a.(*melody.ReasoningType)
	var s string
	if err := Py.GoFromObject(o, &s); err != nil {
		return fmt.Errorf("reasoning type must be a string: %w", err)
	}
	rt, err := melody.ReasoningTypeFromString(s)
	if err != nil {
		return err
	}
	*v = rt
	return nil
}

var filterModeConversion = py.GoConvConf{
	TypeOf:     parsing.FilterMode{},
	ToObject:   filterModeToObject,
	FromObject: filterModeFromObject,
}

func filterModeToObject(Py py.Py, o any) (py.Object, error) {
	if rt, ok := o.(parsing.FilterMode); ok {
		return Py.GoToObject(rt.String())
	}
	return py.Object{}, fmt.Errorf("trying to convert non-FilterMode Go Object to a Python FilterMode")
}
func filterModeFromObject(Py py.Py, o py.Object, a any) error {
	v := a.(*parsing.FilterMode)
	var s string
	if err := Py.GoFromObject(o, &s); err != nil {
		return fmt.Errorf("filter mode must be a string: %w", err)
	}
	rt, err := parsing.FilterModeFromString(s)
	if err != nil {
		return err
	}
	*v = rt
	return nil
}

var orderedJSONObjectConversion = py.GoConvConf{
	TypeOf:     orderedjson.Object{},
	ToObject:   orderedJSONObjectToObject,
	FromObject: orderedJSONObjectFromObject,
}

func orderedJSONObjectToObject(Py py.Py, o any) (py.Object, error) {
	// per https://gitlab.com/pygolo/py/-/blob/main/docs/HOWTO-EMBED.md#creating-a-dictionary
	if rt, ok := o.(orderedjson.Object); ok {
		obj, err := Py.Dict_New()
		defer Py.DecRef(obj)
		if err != nil {
			return py.Object{}, err
		}
		for key, value := range rt.Pairs() {
			err = Py.Dict_SetItem(obj, key, value)
			if err != nil {
				return py.Object{}, err
			}

		}
		return Py.NewRef(obj), nil // because we are creating the object via CPython we are responsible for incrementing the refcount
	}
	return py.Object{}, fmt.Errorf("trying to convert non-OrderedJSON Go Object to a Python Dictionary")
}

func orderedJSONObjectFromObject(Py py.Py, o py.Object, a any) error {
	// copied mostly out of gitlab.com/pygolo/py/dict.go#dictFromObject
	if !(o.Type() == py.Dict_Type) {
		return fmt.Errorf("orderedJSON Object must be a dictionary")
	}
	v := a.(*orderedjson.Object)
	*v = orderedjson.New()

	o_items, err := Py.Dict_Items(o)
	defer Py.DecRef(o_items)
	if err != nil {
		return err
	}

	length, err := Py.Object_Length(o_items)
	if err != nil {
		return err
	}
	for i := 0; i < length; i++ {
		o_item, err := Py.List_GetItem(o_items, i)
		if err != nil {
			return err
		}
		o_key, err := Py.Tuple_GetItem(o_item, 0)
		if err != nil {
			return err
		}
		o_value, err := Py.Tuple_GetItem(o_item, 1)
		if err != nil {
			return err
		}

		var key string
		if err := Py.GoFromObject(o_key, &key); err != nil {
			return fmt.Errorf("orderedjson key (%#v) must be a string: %w", o_key, err)
		}
		var value any
		if err := Py.GoFromObject(o_value, &value); err != nil {
			var str string
			if e := Py.GoFromObject(o_value, &str); e == nil {
				return fmt.Errorf("orderedjson key (%s) error converting value %#v: %s", key, str, err)
			}
			return fmt.Errorf("orderedjson key (%s) error converting value %#v: %s", key, o_value, err)
		}
		v.Set(key, value)
	}
	return nil
}

var filterToolCallDeltaPointerConversion = py.GoConvConf{
	TypeOf:     &parsing.FilterToolCallDelta{},
	ToObject:   filterToolCallDeltaToObject,
	FromObject: filterToolCallDeltaFromObject,
}

func filterToolCallDeltaToObject(Py py.Py, o any) (py.Object, error) {
	if fo, ok := o.(*parsing.FilterToolCallDelta); ok {
		if fo == nil {
			return py.None, nil
		}
		return Py.GoToObject(*fo)
	}
	return py.Object{}, fmt.Errorf("trying to convert non-FilterToolCallDelta Go Object to a Python FilterToolCallDelta")
}

func filterToolCallDeltaFromObject(Py py.Py, o py.Object, a any) error {
	v := a.(**parsing.FilterToolCallDelta)
	if o == py.None {
		*v = nil
		return nil
	}
	var fo parsing.FilterToolCallDelta
	if err := Py.GoFromObject(o, &fo); err != nil {
		return fmt.Errorf("FilterToolCallDelta must be a FilterToolCallDelta: %w", err)
	}
	*v = &fo
	return nil
}

var filterToolParameterPointerConversion = py.GoConvConf{
	TypeOf:     &parsing.FilterToolParameter{},
	ToObject:   filterToolParameterToObject,
	FromObject: filterToolParameterFromObject,
}

func filterToolParameterToObject(Py py.Py, o any) (py.Object, error) {
	if fo, ok := o.(*parsing.FilterToolParameter); ok {
		if fo == nil {
			return py.None, nil
		}
		return Py.GoToObject(*fo)
	}
	return py.Object{}, fmt.Errorf("trying to convert non-FilterOutput Go Object to a Python FilterOutput")
}

func filterToolParameterFromObject(Py py.Py, o py.Object, a any) error {
	v := a.(**parsing.FilterToolParameter)
	if o == py.None {
		*v = nil
		return nil
	}
	var fo parsing.FilterToolParameter
	if err := Py.GoFromObject(o, &fo); err != nil {
		return fmt.Errorf("FilterToolParameter must be a FilterToolParameter: %w", err)
	}
	*v = &fo
	return nil
}
