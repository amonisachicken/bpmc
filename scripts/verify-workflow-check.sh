#!/usr/bin/env bash
# Verify both branches of check-workflows.sh: the shallow one used locally and
# the PyYAML "deep" one used on the Ubuntu runner.
set -u
export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH
PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT" || exit 1

echo "=== shallow branch (no PyYAML, as on this machine)"
bash scripts/check-workflows.sh
echo "exit: $?"

echo
echo "=== deep branch (fixture-driven, as on Ubuntu)"
bash scripts/repro-deep-check.sh 2>&1 | grep -E 'exited with|PROBLEMS|structurally fine|run step|matrix ' | head -20
