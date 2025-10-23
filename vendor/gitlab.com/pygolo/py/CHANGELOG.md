## [0.3.1] - 2024-08-21

### 🐛 Bug fixes

- Added a missing header in a Windows specific C file that was resulting
  in this error when building with LLVM:

```
# gitlab.com/pygolo/py
go-file_windows.go:78:33: could not determine kind of name for C._errno
```

## [0.3.0] - 2024-07-20

### In brief

This release sees the expansion of the tested systems to Windows, OpenBSD
and NetBSD: your Go code is portable to all of them.

Windows, where C long is 32 bits wide also on 64 bit systems, forced us to
rethink how integers are converted; now the API is focused on Go integer
types and the implementation adjusts to the actual target system. This
change is totally transparent to users of `Py.GoToObject` and
`Py.GoFromObject`.

Windows also suggested a new type conversion for files; UNIX file
descriptors are not portable enough. You can read more at [Sharing a
file](docs/HOWTO-EMBED.md#sharing-a-file).  Check also the
[console.go](examples/console.go) example that shows how the Go
application can take control of the embedded Python console.

### Added

#### 🚀 Features

##### For the user

- Build support for Python 3.13 and 3.14 dev releases.
- Allow exported functions to be bound to Python objects.
- Add conversion of file handle/descriptor.
- Add conversion of Go `Uintptr`.
- Add conversion of Go arrays.
- Add conversion of Go structs.
- New API `Object.AsTypeObject()`
- Make `TypeObject.Name()` safe on zero-TypeObject.

##### For the maintainer

- Incorporate the GoHandle size in the basic object size and
  tidy up the object creation.
- Per-interpreter `GOCACHE`, switch to different Python versions without
  having to clear the Go build cache.
- Add Alpine 3.19, 3.20, Fedora 40, Ubuntu 24.04 to the CI pipeline.
- Support overriding the Go compiler
- Run tests also with Python 3.13 free-threading.  
- Run tests with Go data race checks enabled, where possible.
- Allow passing flags to Pytest makefile targets.
- Add makefile target `test-embed-debug` to run tests under gdb so to
  investigate low-level C API crashes.

#### 🐛 Bug fixes

- `GoSetError` now sets the exception type according to the passed `GoError`,
  previously it was always `RuntimeError`.

### Modified

- Replaced API:
  - `Py.Long_From*Long` and `Py.Long_As*Long`

  with:
  - `Py.Long_FromInt*` and `Py.Long_AsInt*` 
  - `Py.Long_FromUint*` and `Py.Long_AsUint*`

  `*` is 8, 16, 32, 64 or simply nothing

- Rename API:
  - `Py.Type(Object)` is now `Object.Type()`
  - `TypeObject.Object()` is now `TypeObject.AsObject()`

- Reworked the converters registration/deregistration, multiple converters
  can be registered in one go and either all or none are added:
  - `ConvConf.Register` is now `GoRegisterConversions(ConvConf...)`
  - `ConvConf.Unregister` is now `GoDeregisterConversions(ConvConf...)`

## [0.2.0] - 2023-12-04

### Added

#### 🚀 Features

- Extend the Python interpreter. See [How to extend the Python interpreter](docs/HOWTO-EXTEND.md).
- Concurrency primitives:
  - `Py.GoEnterPython`, `Py.GoLeavePython`: see [Accessing the interpreter](docs/HOWTO-EXTEND.md#accessing-the-interpreter).
  - `Py.GoNewFlow`: see [Concurrency](docs/HOWTO-EXTEND.md#concurrency).
  - `Py.GoClosureBuddy`: see [GoClosure and its buddy](docs/HOWTO-EXTEND.md#goclosure-and-its-buddy).
- Execute CI tests also with Python debug versions so to catch more bugs.
- Augment tests with property-based testing.
- Support for Python 3.12.

#### 🐛 Bug fixes

- Goroutines are now pinned to their OS threads, Python uses thread-local
  storage and does not tolerate goroutines migration to different threads.
- Do not reuse threads that accessed the Python interpreter, Go runtime
  needs absolute control of its threads and Python could modify them in
  unexpected ways.

### Modified

- All the Go APIs now start with the `Go` prefix, ex. `py.Args` and `py.KwArgs`
  are now named `py.GoArgs` and `py.GoKwArgs`.
- `Py.Object_Length` now returns error by a proper `error` value
  instead of just `-1`.
- `*py.GoError` (not `py.GoError`) now implements the `error` interface,
  incorrect type casting is now detected during compilation.
- Python context for the embedded interpreter is now created by
  `py.GoEmbed`, `py.Py{}` is an invalid and unusable context. See
  [Initialization and finalization](docs/HOWTO-EMBED.md#initialization-and-finalization)
  for the updates.

### Removed

- `Py.Tuple_New` and `Py.Tuple_SetItem` ([#16](https://gitlab.com/pygolo/py/-/issues/16)),
  use `Py.Tuple_Pack` instead.
- Support for Go 1.9, it does not prevent thread reuse when a
  pinned goroutine returns. Scientific Linux 7 is dropped from the test matrix.

## [0.1.1] - 2023-08-28

#### 🐛 Bug fixes

- Fix build issue with Go 1.21

## [0.1.0] - 2023-07-04

💥 First release ever! 💥

This release focused on readying basic life support and solid ground for growth.

### Added

#### 🚀 Features

- Embed the Python interpreter
- Convert basic types to/from Python
- Handle Python exceptions as regular Go errors
- Import modules
- Call functions and objects
- Build with pyenv provided interpreters
- Run in venv environments

#### 📚 Documentation

- [Contributing](CONTRIBUTING.md)
- [How to embed](docs/HOWTO-EMBED.md)
- [Advanced topics](docs/ADVANCED-TOPICS.md)

### It Could Work!

<img src="http://www.frankensteinjunior.it/download/foto/1/big/FJ_015.jpg" alt="Gene Wilder exclaiming 'It Could Work!'" width=256 height=144>
