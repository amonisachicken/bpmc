#!/usr/bin/env bash
# Full verification for bpmc: tests, clippy, release build, render demo.
#
# Built for this machine's MSYS2 setup: the Rust toolchain lives in clang64,
# while binutils' dlltool (needed for the windows-gnu link step) lives in
# mingw64, so both prefixes must be on PATH.
set -u

export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH
PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${TMPDIR:-/tmp}/bpmc-check"
mkdir -p "$OUT"

cd "$PROJECT" || exit 1

echo "===== 1/4 cargo test ====="
cargo test 2>&1 | tail -25
echo "test exit: ${PIPESTATUS[0]}"

echo
echo "===== 2/4 cargo clippy ====="
cargo clippy --all-targets -- -D warnings 2>&1 | tail -6
echo "clippy exit: ${PIPESTATUS[0]}"

echo
echo "===== 3/4 release build ====="
cargo build --release 2>&1 | tail -4
echo "release exit: ${PIPESTATUS[0]}"

echo
echo "===== 4/4 render demo ====="
cargo run --quiet --example render_demo > "$OUT/render_en.txt" 2>&1
echo "render en exit: $?"
cargo run --quiet --example render_demo -- zh > "$OUT/render_zh.txt" 2>&1
echo "render zh exit: $?"
cargo run --quiet --example render_demo -- --check
echo "render check exit: $?"

ls -la target/release/bpmc.exe 2>/dev/null
