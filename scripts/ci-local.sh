#!/usr/bin/env bash
# Mirror of the GitHub Actions CI steps, run locally.
set -u
export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH
PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT" || exit 1

echo "== cargo fmt --all   (apply)"
cargo fmt --all
echo "fmt exit: $?"

echo
echo "== cargo fmt --all -- --check"
cargo fmt --all -- --check
echo "fmt check exit: $?"

echo
echo "== cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings 2>&1 | tail -5
echo "clippy exit: ${PIPESTATUS[0]}"

echo
echo "== cargo test --all-targets"
cargo test --all-targets 2>&1 | grep -E '^(test result|error)' | head -10
echo "test exit: ${PIPESTATUS[0]}"

echo
echo "== cargo run --example render_demo -- --check"
cargo run --quiet --example render_demo -- --check
echo "render exit: $?"

echo
echo "== cargo build --release"
cargo build --release 2>&1 | tail -3
echo "release exit: ${PIPESTATUS[0]}"

echo
echo "== artifact"
ls -la target/release/bpmc.exe target/release/bpmc 2>/dev/null
