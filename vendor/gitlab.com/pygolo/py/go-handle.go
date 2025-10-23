//go:build go1.21
// +build go1.21

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

import "runtime"

// GoHandle makes a Go value referrable by the Python interpreter.
//
// The GoHandle is usually allocated by malloc or equivalent, its
// pointer can then be safely used in C structures and later
// dereferenced by Go code again.
//
// It's important to close the handle before its storage is freed.
type GoHandle struct {
	pinner *runtime.Pinner
	value  *interface{}
}

// Get gets the handle value
func (h *GoHandle) Get() interface{} {
	return *h.value
}

// Set sets the handle value
func (h *GoHandle) Set(v interface{}) {
	h.Close()
	pinner := &runtime.Pinner{}
	pinner.Pin(pinner)
	pinner.Pin(&v)
	h.pinner = pinner
	h.value = &v
}

// Close stops the Go runtime tracking of the referred Go value.
//
// If the Go runtime does not manage the GoHandle storage it cannot detect
// when the referred value becomes inaccessible. Before deallocating the
// GoHandle is necessary to close it so to prevent memory leaks.
func (h *GoHandle) Close() {
	if h.pinner != nil {
		h.pinner.Unpin()
		h.pinner = nil
		h.value = nil
	}
}
