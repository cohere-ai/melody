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
