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

import "sync"

// GoClosure helps in handling Go closures of Python objects.
type GoClosure struct {
	wg       *sync.WaitGroup
	captured []Object
}

// GoClosureBuddy helps in tracking Go closures of Python objects.
type GoClosureBuddy struct {
	wg       sync.WaitGroup
	closures []*GoClosure
	py       Py
}

// GoNewClosureBuddy creates a GoClosureBuddy.
func (Py Py) GoNewClosureBuddy() GoClosureBuddy {
	return GoClosureBuddy{py: Py}
}

// Close handles the reference counting of the rejected closures.
//
// It blocks until all the associated closures are either accepted or rejected.
func (cb *GoClosureBuddy) Close() {
	cb.wg.Wait()
	for _, cl := range cb.closures {
		cb.py.DecRef(cl.captured...)
	}
}

// Capture creates a GoClosure reporting to buddy cb.
//
// Shall be invoked from the same goroutine of its buddy and with
// access to the interpreter.
//
//	func ExportedFn(Py py.Py, o py.Object) {
//	        buddy := Py.GoNewClosureBuddy()
//	        defer buddy.Close()
//
//	        Py.GoEnterPython()
//	        defer Py.GoLeavePython()
//
//	        go func(closure *py.GoClosure) {
//	                Py, err := Py.GoNewFlow(closure)
//	                defer Py.Close()
//	                // handle error err
//
//	                Py.GoEnterPython()
//	                defer Py.GoLeavePython()
//
//	                // use object o
//	        }(buddy.Capture(o))
//	}
func (cb *GoClosureBuddy) Capture(oo ...Object) *GoClosure {
	cl := &GoClosure{wg: &cb.wg, captured: oo}
	cb.closures = append(cb.closures, cl)
	cb.py.IncRef(cl.captured...)
	cb.wg.Add(1)
	return cl
}

// Accept signals to the buddy the commitment to handle the captured objects.
//
// It returns the objects captured by the closure.
//
// It's responsibility of the caller to decrement the refcount
// of the captured objects when not needed any more.
func (cl *GoClosure) Accept() []Object {
	var captured []Object
	if cl != nil && cl.wg != nil {
		captured = cl.captured
		cl.captured = nil
		cl.wg.Done()
		cl.wg = nil
	}
	return captured
}

// Reject signals to the buddy the refusal to handle the captured objects.
func (cl *GoClosure) Reject() {
	if cl != nil && cl.wg != nil {
		cl.wg.Done()
		cl.wg = nil
	}
}
