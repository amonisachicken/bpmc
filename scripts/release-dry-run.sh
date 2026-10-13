#!/usr/bin/env bash
# Dry-run of the release packaging steps so the workflow is not shipped untested.
# Mirrors .github/workflows/release.yml step by step, using the local release
# binary and the real version from Cargo.toml.
set -u
export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH
PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT" || exit 1

VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
NAME="bpmc-${VERSION}-linux-x86_64"
BIN="target/release/bpmc.exe"   # local build output on this machine

echo "== version: $VERSION"
echo "== tag/version guard"
TAG="v${VERSION}"
if [ "${TAG#v}" != "$VERSION" ]; then
  echo "guard FAILED"
  exit 1
fi
echo "guard ok: $TAG"

echo
echo "== unix packaging"
rm -rf dist
mkdir -p "dist/$NAME"
cp "$BIN" "dist/$NAME/"
cp README.md LICENSE "dist/$NAME/"
tar -C dist -czf "dist/$NAME.tar.gz" "$NAME"
rm -rf "dist/$NAME"
(cd dist && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256")
ls -la dist

echo
echo "== checksum content"
cat "dist/$NAME.tar.gz.sha256"

echo
echo "== archive listing"
tar -tzf "dist/$NAME.tar.gz"
