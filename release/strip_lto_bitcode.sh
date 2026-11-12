#!/usr/bin/env bash
# Strip LLVM LTO sections from libcohere_melody.a.
#
# Go 1.27.4 rejects any archive named in #cgo LDFLAGS whose members carry LTO
# sections (cmd/go/internal/work/security.go):
#
#     disallowed LTO section ".llvmbc" in #cgo LDFLAGS: .../libcohere_melody.a
#
# rustc bundles the precompiled Rust standard library -- bitcode included --
# into a staticlib, so every published archive has to be stripped before it is
# packaged. The release Dockerfile's Go 1.27.4 smoke test links the result, so
# a strip that does nothing fails the build.
#
# Usage: strip_lto_bitcode.sh <path to libcohere_melody.a>
set -euo pipefail

lib="${1:?usage: $0 <path to libcohere_melody.a>}"
if [ ! -f "$lib" ]; then
	echo "error: $lib does not exist" >&2
	exit 1
fi

# GNU objcopy cannot read these archives (some members are raw LLVM bitcode),
# so require the LLVM objcopy that ships with the Rust toolchain.
objcopy=""
objcopy_libdir=""
if sysroot="$(rustc --print sysroot 2>/dev/null)"; then
	host="$(rustc -vV | sed -n 's/^host: //p')"
	candidate="$sysroot/lib/rustlib/$host/bin/rust-objcopy"
	if [ -x "$candidate" ]; then
		objcopy="$candidate"
		# The sysroot binary links against the toolchain's libLLVM, which is
		# not on the default loader path.
		objcopy_libdir="$sysroot/lib"
	fi
fi
if [ -z "$objcopy" ] && command -v llvm-objcopy >/dev/null 2>&1; then
	objcopy="$(command -v llvm-objcopy)"
fi
if [ -z "$objcopy" ]; then
	echo "error: no LLVM objcopy found (looked for rust-objcopy in the Rust sysroot and llvm-objcopy on PATH)" >&2
	exit 1
fi
if [ -n "$objcopy_libdir" ]; then
	if [ "$(uname -s)" = Darwin ]; then
		DYLD_FALLBACK_LIBRARY_PATH="$objcopy_libdir${DYLD_FALLBACK_LIBRARY_PATH:+:$DYLD_FALLBACK_LIBRARY_PATH}"
		export DYLD_FALLBACK_LIBRARY_PATH
	else
		LD_LIBRARY_PATH="$objcopy_libdir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
		export LD_LIBRARY_PATH
	fi
fi

# ELF keeps the bitcode in sections, Mach-O in an __LLVM segment. Naming a
# section the archive does not have is a no-op, so pass both sets and let the
# archive format decide; that also keeps the script idempotent.
sections=(.llvmbc .llvmcmd __LLVM,__bitcode __LLVM,__cmdline)

args=()
for section in "${sections[@]}"; do
	args+=("--remove-section=$section")
done

"$objcopy" "${args[@]}" "$lib"
