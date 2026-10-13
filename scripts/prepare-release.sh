#!/usr/bin/env bash
# Prepare a version bump for release: sync Cargo.lock, then verify everything
# the release workflow will check.
set -u
export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH
PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT" || exit 1

VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
echo "== Cargo.toml version: $VERSION"

echo
echo "== cargo check (also refreshes the version recorded in Cargo.lock)"
cargo check --quiet --locked 2>&1 | tail -5
echo "locked check exit (expected non-zero only if the lock file is stale): ${PIPESTATUS[0]}"
cargo check --quiet 2>&1 | tail -3
echo "check exit: ${PIPESTATUS[0]}"

echo
echo "== Cargo.lock entry for this package"
awk -v pkg='name = "bpmc"' '
  $0 == pkg { found = 1 }
  found && /^version = / { print "   " $0; exit }
' Cargo.lock

echo
echo "== the release workflow's tag guard, run the way the workflow runs it"
manifest=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
tag="v${manifest}"
if [ "${tag#v}" != "$manifest" ]; then
  echo "   guard FAILED"
  exit 1
fi
echo "   guard ok: tag $tag matches Cargo.toml $manifest"

echo
echo "== a --locked release build (this is what CI runs)"
cargo build --release --locked 2>&1 | tail -3
echo "release exit: ${PIPESTATUS[0]}"
