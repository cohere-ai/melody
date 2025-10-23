package main

import (
	"fmt"

	"gitlab.com/pygolo/py"

	"github.com/cohere-ai/melody"
)

var roleConversion = py.GoConvConf{
	TypeOf:     melody.Role{},
	ToObject:   roleToObject,
	FromObject: roleFromObject,
}

func roleToObject(Py py.Py, o any) (py.Object, error) {
	if r, ok := o.(melody.Role); ok {
		return Py.GoToObject(r.String())
	}
	return py.None, nil
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
	return py.None, nil
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
	return py.None, nil
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
	return py.None, nil
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
	return py.None, nil
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
	return py.None, nil
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
