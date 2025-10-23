/*
 * Copyright 2022, Pygolo Project contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package py

// #include "pygolo.h"
import "C"
import "math"

func (Py Py) long_FromLong(v int32) (Object, error) {
	return Py.wrap(C.PyLong_FromLong(C.long(v)))
}

func (Py Py) long_FromUnsignedLong(v uint32) (Object, error) {
	return Py.wrap(C.PyLong_FromUnsignedLong(C.ulong(v)))
}

func (Py Py) long_AsLong(o Object) (int32, error) {
	v := C.PyLong_AsLong(o.o)
	if v == -1 && Py.Err_Occurred() != (Object{}) {
		return 0, Py.GoCatchError()
	}
	return int32(v), nil
}

func (Py Py) long_AsUnsignedLong(o Object) (uint32, error) {
	v := C.PyLong_AsUnsignedLong(o.o)
	if C.long(v) == -1 && Py.Err_Occurred() != (Object{}) {
		return 0, Py.GoCatchError()
	}
	return uint32(v), nil
}

// Long_FromInt returns a new long object from int v.
func (Py Py) Long_FromInt(v int) (Object, error) {
	return Py.long_FromLongLong(int64(v))
}

// Long_FromInt8 returns a new long object from int8 v.
func (Py Py) Long_FromInt8(v int8) (Object, error) {
	return Py.long_FromLong(int32(v))
}

// Long_FromInt16 returns a new long object from int16 v.
func (Py Py) Long_FromInt16(v int16) (Object, error) {
	return Py.long_FromLong(int32(v))
}

// Long_FromInt32 returns a new long object from int32 v.
func (Py Py) Long_FromInt32(v int32) (Object, error) {
	return Py.long_FromLong(v)
}

// Long_FromInt64 returns a new long object from int64 v.
func (Py Py) Long_FromInt64(v int64) (Object, error) {
	return Py.long_FromLongLong(v)
}

// Long_FromUint returns a new long object from uint v.
func (Py Py) Long_FromUint(v uint) (Object, error) {
	return Py.long_FromUnsignedLongLong(uint64(v))
}

// Long_FromUint8 returns a new long object from uint8 v.
func (Py Py) Long_FromUint8(v uint8) (Object, error) {
	return Py.long_FromUnsignedLong(uint32(v))
}

// Long_FromUint16 returns a new long object from uint16 v.
func (Py Py) Long_FromUint16(v uint16) (Object, error) {
	return Py.long_FromUnsignedLong(uint32(v))
}

// Long_FromUint32 returns a new long object from uint32 v.
func (Py Py) Long_FromUint32(v uint32) (Object, error) {
	return Py.long_FromUnsignedLong(v)
}

// Long_FromUint64 returns a new long object from uint64 v.
func (Py Py) Long_FromUint64(v uint64) (Object, error) {
	return Py.long_FromUnsignedLongLong(v)
}

// Long_FromUintptr returns a new long object from uintptr v.
func (Py Py) Long_FromUintptr(v uintptr) (Object, error) {
	return Py.long_FromUnsignedLongLong(uint64(v))
}

// Long_AsInt returns an int representation of o.
//
// If the Python int doesn't fit in the Go int, an error is returned.
func (Py Py) Long_AsInt(o Object) (int, error) {
	v, e := Py.long_AsLongLong(o)
	if e != nil {
		return 0, long_error(int(v), e)
	}
	return int(v), nil
}

// Long_AsInt8 returns an int8 representation of o.
//
// If the Python int doesn't fit in the Go int8, an error is returned.
func (Py Py) Long_AsInt8(o Object) (int8, error) {
	v, e := Py.long_AsLong(o)
	if e != nil || v < math.MinInt8 || math.MaxInt8 < v {
		return 0, long_error(int8(v), e)
	}
	return int8(v), nil
}

// Long_AsInt16 returns an int16 representation of o.
//
// If the Python int doesn't fit in the Go int16, an error is returned.
func (Py Py) Long_AsInt16(o Object) (int16, error) {
	v, e := Py.long_AsLong(o)
	if e != nil || v < math.MinInt16 || math.MaxInt16 < v {
		return 0, long_error(int16(v), e)
	}
	return int16(v), nil
}

// Long_AsInt32 returns an int32 representation of o.
//
// If the Python int doesn't fit in the Go int32, an error is returned.
func (Py Py) Long_AsInt32(o Object) (int32, error) {
	v, e := Py.long_AsLong(o)
	if e != nil || v < math.MinInt32 || math.MaxInt32 < v {
		return 0, long_error(int32(v), e)
	}
	return int32(v), nil
}

// Long_AsInt64 returns an int64 representation of o.
//
// If the Python int doesn't fit in the Go int64, an error is returned.
func (Py Py) Long_AsInt64(o Object) (int64, error) {
	v, e := Py.long_AsLongLong(o)
	if e != nil {
		return 0, long_error(v, e)
	}
	return v, nil
}

// Long_AsUint returns a uint representation of o.
//
// If the Python int doesn't fit in the Go uint, an error is returned.
func (Py Py) Long_AsUint(o Object) (uint, error) {
	v, e := Py.long_AsUnsignedLongLong(o)
	if e != nil {
		return 0, long_error(uint(v), e)
	}
	return uint(v), nil
}

// Long_AsUint8 returns a uint8 representation of o.
//
// If the Python int doesn't fit in the Go uint8, an error is returned.
func (Py Py) Long_AsUint8(o Object) (uint8, error) {
	v, e := Py.long_AsUnsignedLong(o)
	if e != nil || math.MaxUint8 < v {
		return 0, long_error(uint8(v), e)
	}
	return uint8(v), nil
}

// Long_AsUint16 returns a uint16 representation of o.
//
// If the Python int doesn't fit in the Go uint16, an error is returned.
func (Py Py) Long_AsUint16(o Object) (uint16, error) {
	v, e := Py.long_AsUnsignedLong(o)
	if e != nil || math.MaxUint16 < v {
		return 0, long_error(uint16(v), e)
	}
	return uint16(v), nil
}

// Long_AsUint32 returns a uint32 representation of o.
//
// If the Python int doesn't fit in the Go uint32, an error is returned.
func (Py Py) Long_AsUint32(o Object) (uint32, error) {
	v, e := Py.long_AsUnsignedLong(o)
	if e != nil || math.MaxUint32 < v {
		return 0, long_error(uint32(v), e)
	}
	return uint32(v), nil
}

// Long_AsUint64 returns a uint64 representation of o.
//
// If the Python int doesn't fit in the Go uint64, an error is returned.
func (Py Py) Long_AsUint64(o Object) (uint64, error) {
	v, e := Py.long_AsUnsignedLongLong(o)
	if e != nil {
		return 0, long_error(v, e)
	}
	return v, nil
}

// Long_AsUintptr returns a uintptr representation of o.
//
// If the Python int doesn't fit in the Go uintptr, an error is returned.
func (Py Py) Long_AsUintptr(o Object) (uintptr, error) {
	v, e := Py.long_AsUnsignedLongLong(o)
	if e != nil {
		return 0, long_error(uintptr(v), e)
	}
	return uintptr(v), nil
}
