#!/usr/bin/env bash
# Helper: build bpmc with this machine's MSYS2 clang64 Rust toolchain.
set -u

# clang64 comes first so the Rust toolchain and its linker win over mingw64;
# mingw64 is needed only for binutils' dlltool, which the GNU link step calls.
export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH

PROJECT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$PROJECT" || exit 1
echo "== rustc: $(rustc --version)"
echo "== target: $(rustc -vV | grep host)"
echo "== cargo build"
cargo build 2>&1 | tail -40
echo "== build exit: ${PIPESTATUS[0]}"
