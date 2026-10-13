#!/usr/bin/env bash
# Run a focused cargo command with this machine's toolchain on PATH.
#
# Usage: bash scripts/run.sh cargo test --test history
set -u
export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH
PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT" || exit 1
"$@"
