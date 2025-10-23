# <img width="96px" height="96px" src="./images/logo-small.png" alt="Go mascot wearing a hat with Python logo colors" /> &nbsp; pygolo - embed or extend the Python interpreter with Go

[![Go Reference](https://pkg.go.dev/badge/gitlab.com/pygolo/py.svg)](https://pkg.go.dev/gitlab.com/pygolo/py)
[![Gitter](https://img.shields.io/gitter/room/pygolo/py)](https://gitter.im/pygolo/py)
[![Reddit](https://img.shields.io/badge/share-on%20reddit-red)](https://www.reddit.com/r/pygolo)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE-OF-CONDUCT.md)

## Overview

This project assists you in crossing the Python-Go boundary: extending
Python with Go and vice versa.

It also helps you to:

- reduce the friction between such different environments
- evaluate if mixing them does resonate with your needs
- fail fast, cheap and safe when such mixing is not best for you

## Project status

There is a significant core of functionality for embedding the
interpreter:

- type conversion
- error handling
- module importing
- object calling

Have a look at [How to embed the Python interpreter](docs/HOWTO-EMBED.md).

You can extend the interpreter with an increasing set of features:

- export Go values (integers, floats, strings, etc)
- export Go function
- concurrency

See how it works at [How to extend the Python interpreter](docs/HOWTO-EXTEND.md).

Coverage of the C API is expanded as needed, see what is available at
[![Go Reference](https://pkg.go.dev/badge/gitlab.com/pygolo/py.svg)](https://pkg.go.dev/gitlab.com/pygolo/py#pkg-index).
Feel free to open a MR to add what you miss.

## Supported versions

We'd like to not leave anybody alone and support every version of Go and
Python but this would require quite some effort and maybe the sacrifice of
some features, therefore there are limits it's unlikely we'll ever cross.

The good news is that we currently cover a good range of systems and we
are open to new entries; if we can test it, we can support it. Have a look
at our [Test matrix](docs/TEST-MATRIX.md).

## Community

We have a chat at [https://gitter.im/pygolo/py](https://gitter.im/pygolo/py),
you can also join it with your favourite [&lsqb;matrix&rsqb;](https://matrix.org)
client at [#pygolo:gitter.im](https://matrix.to/#/#pygolo:gitter.im). Feel
free also to join us on Reddit at [r/pygolo](https://www.reddit.com/r/pygolo),
check what others do, share your know-how.

Observe our [Code of conduct](CODE-OF-CONDUCT.md), make questions, get
involved, feel at home.

## Code of conduct

Come as you are. Our community depends on the respectful, safe, and
collaborative environment we build together.

Take some time to read the [Code of conduct](CODE-OF-CONDUCT.md).

## Contributing

You are welcomed to contribute to this project. Read
[Contributing](CONTRIBUTING.md) and you'll be able to get your hands dirty
straight away.

## License

This is a free software project under Apache License 2.0 and every
contributor is a copyright holder. As plain as we can imagine, everything
is of everybody who worked at the project.

## About the name

In establishing a tradition of not taking ourselves too seriously, we just
glued the Python and Go names together with a salt of craziness (👉 loco).
