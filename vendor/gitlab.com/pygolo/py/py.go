//go:build go1.10
// +build go1.10

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

// Package py allows you to embed or extend the Python interpreter with Go.
package py

// #include "pygolo.h"
//
// void pyEval_InitThreads(void)
// {
//   #if PY_VERSION_HEX < 0x03070000
//     PyEval_InitThreads();
//   #endif
// }
import "C"
import (
	"fmt"
	"runtime"
	"unsafe"
)

// Py is the context construct that the Python C API lacks.
type Py struct {
	_      [0]func() // disallow comparisons
	state  *GoHandle
	tstate *C.PyThreadState
	closer func()
}

// pyPy is an useful alias in scopes where Py is shadowed
type pyPy = Py

// GoEmbed returns the Py context for an embedded interpreter.
//
// Use Close() to dispose the context once it's not needed any more.
//
// Access to the interpreter is allowed after this call.
func GoEmbed() (Py, error) {
	p := Py{}
	if C.Py_IsInitialized() != 0 {
		return Py{}, fmt.Errorf("the interpreter is already initialized")
	}
	p.state = (*GoHandle)(C.PyMem_RawCalloc(1, C.size_t(unsafe.Sizeof(GoHandle{}))))
	if p.state == nil {
		return Py{}, fmt.Errorf("cannot allocate state handle")
	}
	// Python uses thread-local storage and has free hands to modify the threads state,
	// let's play safe and:
	//  - pin this goroutine to the current thread
	//  - do not run any other goroutine on it
	//  - do not reuse the thread once the pinned goroutine exits, it will be
	//    killed by the Go runtime
	runtime.LockOSThread()
	C.Py_Initialize()
	C.pyEval_InitThreads()
	p.state.Set(&state{})
	p.tstate = C.PyThreadState_Get()
	p.closer = func() {
		p.deinitTypes()
		C.Py_Finalize()
		p.state.Close()
		C.PyMem_RawFree(unsafe.Pointer(p.state))
	}
	err := p.initTypes(Object{})
	if err != nil {
		p.closer()
		return Py{}, err
	}
	return p, nil
}

func extendModule(m Object) Py {
	p := Py{}
	p.state = p.getStateFromModule(m)
	p.tstate = C.PyThreadState_Get()
	return p
}

func extendObject(o Object) (Py, error) {
	p := Py{}
	p.state = p.getStateFromObject(o)
	p.tstate = C.PyThreadState_Get()
	if p.state == nil || p.state.Get() == nil {
		return Py{}, fmt.Errorf("the state is not initialized")
	}
	return p, nil
}

// GoNewFlow returns the context for a new execution flow.
//
// If closure cl is passed, GoNewFlow will take care of the captured
// objects lifetime.
//
// Use Close() to dispose the context once it's not needed any more.
//
// Access to the interpreter is not needed or modified by this call
// therefore it's not allowed from the returned context until explicitly
// enabled with GoEnterPython().
func (p Py) GoNewFlow(cl *GoClosure) (Py, error) {
	defer cl.Reject()
	if p.state == nil || p.state.Get() == nil || p.tstate == nil {
		return Py{}, fmt.Errorf("the state is not initialized")
	}
	runtime.LockOSThread()
	if C.PyGILState_GetThisThreadState() != nil {
		runtime.UnlockOSThread()
		return Py{}, fmt.Errorf("this goroutine already owns a Py context")
	}
	p.tstate = C.PyThreadState_New(p.tstate.interp)
	if p.tstate == nil {
		runtime.UnlockOSThread()
		return Py{}, fmt.Errorf("could not create Python thread state")
	}
	captured := cl.Accept()
	p.closer = func() {
		p.GoEnterPython()
		p.DecRef(captured...)
		C.PyThreadState_Clear(p.tstate)
		p.GoLeavePython()
		C.PyThreadState_Delete(p.tstate)
	}
	return p, nil
}

// Close disposes the Py context as needed.
//
// If the Py context was returned by Embed then Close expects
// to already have access to the interpreter.
//
// If the Py context was returned by NewFlow then Close will
// enable the access possibly blocking until it becomes available.
func (p *Py) Close() error {
	if p.state == nil || p.state.Get() == nil || p.tstate == nil {
		return fmt.Errorf("the state is not initialized")
	}
	if p.closer != nil {
		p.closer()
		p.closer = nil
	}
	p.state = nil
	p.tstate = nil
	return nil
}

// GoEnterPython enables access to the interpreter.
//
// It may block until such access is availalble.
//
// Calls to the interpreter are allowed after this call, they all need
// to happen from the same goroutine.
//
// Other Py contexts may or may not be allowed to access the
// interpreter at the same time.
//
// Don't assume any synchronization to be in place while allowed to
// access the interpreter, keep concurrent access to data structures
// well in order independently from the interpreter access state.
//
// It's important to disable the access before slow or blocking
// operations, if they don't require it, so to give way to other
// flows that may be waiting for it.
//
// Calling GoEnterPython while the access is already enabled may lead to
// a deadlock.
func (p Py) GoEnterPython() {
	C.PyEval_AcquireThread(p.tstate)
}

// GoLeavePython disables access to the interpreter.
//
// Calls to the interpreter are not allowed after this call.
//
// GoLeavePython must be invoked from the same goroutine that
// previously invoked GoEnterPython on the given Py context.
//
// Calling GoLeavePython while the access is already disabled may
// cause a fatal error.
func (p Py) GoLeavePython() {
	C.PyEval_ReleaseThread(p.tstate)
}

func (p Py) getState() *state {
	return p.state.Get().(*state)
}
