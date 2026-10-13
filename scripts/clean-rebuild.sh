#!/usr/bin/env bash
# Clean rebuild + smoke test after the project moved to its final path.
set -u
export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH
PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT" || exit 1

echo "== pwd: $(pwd)"
echo "== cargo clean"
cargo clean
echo "== cargo build --release"
cargo build --release 2>&1 | tail -4
echo "build exit: ${PIPESTATUS[0]}"

echo
echo "== artifact"
ls -la target/release/bpmc.exe target/release/bpmc 2>/dev/null

echo
echo "== smoke test with no TTY (stdin is a pipe here)"
./target/release/bpmc.exe < /dev/null
echo "no-TTY exit: $?"
