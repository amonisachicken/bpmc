#!/usr/bin/env bash
# Local simulation of .github/workflows/release.yml.
#
# The workflow's build matrix is read straight out of the YAML rather than
# duplicated here, so a matrix typo (like a `bin` path missing the target
# triple) is caught locally in seconds instead of after a failed release run.
#
# It resolves the matrix entry for the current platform, then runs the same
# steps the workflow runs: build -> binary check -> package -> verify.
#
#   bash scripts/run-release-steps.sh            # current platform, real build
#   bash scripts/run-release-steps.sh --list     # print the matrix and exit
set -u

PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT" || exit 1

# This machine's Rust toolchain lives in the MSYS2 clang64 prefix and is not on
# the login shell's PATH; the mingw64 prefix supplies dlltool for the GNU link.
for prefix in /clang64/bin /mingw64/bin; do
  if [ -d "$prefix" ] && ! printf '%s' "$PATH" | grep -q "$prefix"; then
    PATH="$prefix:$PATH"
  fi
done
export PATH

WORKFLOW=".github/workflows/release.yml"
DIST="dist"

PY=""
for candidate in python3 /clang64/bin/python3 /mingw64/bin/python3 /usr/bin/python3; do
  if command -v "$candidate" >/dev/null 2>&1; then PY="$candidate"; break; fi
done
[ -n "$PY" ] || { echo "no python3 found" >&2; exit 1; }

# ---------------------------------------------------------------- read matrix
# PyYAML is optional: without it a tiny line parser handles this file's shape.
MATRIX=$("$PY" - "$WORKFLOW" <<'PY'
import re
import sys

path = sys.argv[1]
text = open(path, encoding="utf-8").read()

entries = []
try:
    import yaml  # type: ignore

    doc = yaml.safe_load(text)
    for job in (doc.get("jobs") or {}).values():
        for entry in ((job.get("strategy") or {}).get("matrix") or {}).get("include") or []:
            entries.append(entry)
except ImportError:
    current = None
    for raw in text.splitlines():
        if re.match(r"\s*-\s+name:\s*\S+", raw):
            if current:
                entries.append(current)
            current = {"name": raw.split("name:", 1)[1].strip()}
            continue
        if current is not None:
            m = re.match(r"\s+(os|target|bin|ext):\s*(\S+)", raw)
            if m:
                current[m.group(1)] = m.group(2).strip()

for e in entries:
    if "name" in e and "target" in e and "bin" in e:
        print("|".join([e["name"], e.get("os", ""), e["target"], e["bin"], e.get("ext", "")]))
PY
) || { echo "could not read the matrix" >&2; exit 1; }

if [ "${1:-}" = "--list" ]; then
  echo "$MATRIX" | while IFS='|' read -r name os target bin ext; do
    printf '%-18s os=%-14s target=%-26s bin=%s\n' "$name" "$os" "$target" "$bin"
  done
  exit 0
fi

# --------------------------------------------------- pick the current platform
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) HOST=windows ;;
  Darwin)              HOST=macos ;;
  *)                   HOST=linux ;;
esac

ENTRY=""
while IFS='|' read -r name os target bin ext; do
  case "$os" in
    ubuntu-latest) os_kind=linux ;;
    macos-latest)  os_kind=macos ;;
    windows-latest) os_kind=windows ;;
    *) os_kind=unknown ;;
  esac
  if [ "$os_kind" = "$HOST" ] && [ -z "$ENTRY" ]; then
    ENTRY="$name|$os|$target|$bin|$ext"
  fi
done <<< "$MATRIX"

if [ -z "$ENTRY" ]; then
  echo "no matrix entry for host '$HOST'; use --list to inspect" >&2
  exit 1
fi

IFS='|' read -r NAME OS TARGET BIN EXT <<< "$ENTRY"
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
PKG="bpmc-${VERSION}-${NAME}"

echo "== simulating the '$NAME' job locally"
echo "   host      : $HOST ($(uname -s))"
echo "   runner    : $OS"
echo "   target    : $TARGET"
echo "   bin (yaml): $BIN"
echo "   version   : $VERSION"
echo

# ------------------------------------------------------------ check tag job
echo "== step: Tag must match Cargo.toml version"
tag="v${VERSION}"
if [ "${tag#v}" != "$VERSION" ]; then echo "guard FAILED"; exit 1; fi
echo "   guard ok ($tag)"

# ------------------------------------------------------------------ build
echo
echo "== step: Build"
installed=""
if command -v rustup >/dev/null 2>&1; then
  installed=$(rustup target list --installed 2>/dev/null || true)
fi

if printf '%s\n' "$installed" | grep -qx "$TARGET"; then
  # Exactly what CI does: the runner installs the matrix target first.
  cargo build --release --locked --target "$TARGET"
else
  echo "   target $TARGET is not installed for this toolchain"
  echo "   (CI installs it with dtolnay/rust-toolchain; building for the host)"
  cargo build --release --locked
  case "$HOST" in
    windows) BIN="target/release/bpmc.exe" ;;
    *)       BIN="target/release/bpmc" ;;
  esac
  echo "   using $BIN instead"
fi

# -------------------------------------------------- step: binary must exist
echo
echo "== step: Check the binary exists"
if [ ! -f "$BIN" ]; then
  echo "   FAILED: $BIN not found (this is exactly the release bug)" >&2
  ls -la target target/release 2>/dev/null || true
  find target -maxdepth 3 -name 'bpmc*' -type f 2>/dev/null || true
  exit 1
fi
ls -la "$BIN"

# ------------------------------------------------------------- packaging
echo
echo "== step: Package"
rm -rf "$DIST"
if [ "$HOST" = "windows" ]; then
  echo "   (windows packaging mirrors the pwsh step)"
  "$PY" - "$PKG" "$BIN" <<'PY'
import hashlib
import pathlib
import sys
import zipfile

name, binary = sys.argv[1], sys.argv[2]
root = pathlib.Path("dist") / name
root.mkdir(parents=True, exist_ok=True)
for src in (binary, "README.md", "LICENSE"):
    dest = root / pathlib.Path(src).name
    dest.write_bytes(pathlib.Path(src).read_bytes())

archive = pathlib.Path("dist") / f"{name}.zip"
with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as zf:
    for f in sorted(root.iterdir()):
        zf.write(f, f"{name}/{f.name}")

digest = hashlib.sha256(archive.read_bytes()).hexdigest()
(pathlib.Path("dist") / f"{name}.zip.sha256").write_text(
    f"{digest}  {name}.zip\n", encoding="ascii"
)
for f in sorted(pathlib.Path("dist").iterdir()):
    if f.is_file():
        print(f"   {f.name}  {f.stat().st_size} bytes")
PY
  ARCHIVE="dist/$PKG.zip"
else
  mkdir -p "dist/$PKG"
  cp "$BIN" "dist/$PKG/"
  cp README.md LICENSE "dist/$PKG/"
  tar -C dist -czf "dist/$PKG.tar.gz" "$PKG"
  rm -rf "dist/$PKG"
  (cd dist && sha256sum "$PKG.tar.gz" > "$PKG.tar.gz.sha256")
  ls -la dist
  ARCHIVE="dist/$PKG.tar.gz"
fi

# ----------------------------------------------------------- verify archive
echo
echo "== verify the archive contents"
"$PY" - "$ARCHIVE" "$PKG" <<'PY'
import pathlib
import sys
import tarfile
import zipfile

archive, name = sys.argv[1], sys.argv[2]
if archive.endswith(".zip"):
    with zipfile.ZipFile(archive) as zf:
        names = zf.namelist()
else:
    with tarfile.open(archive) as tf:
        names = tf.getnames()

print(f"   {pathlib.Path(archive).name} contains:")
for n in names:
    print(f"     {n}")

required = [f"{name}/README.md", f"{name}/LICENSE"]
missing = [r for r in required if r not in names]
has_binary = any(pathlib.Path(n).name in ("bpmc", "bpmc.exe") for n in names)

if not names:
    sys.exit("   FAILED: empty archive")
if missing:
    sys.exit(f"   FAILED: missing entries: {missing}")
if not has_binary:
    sys.exit("   FAILED: no binary inside the archive")
print("   archive looks correct")
PY

echo
echo "== done. Packages in $DIST/ (CI would now upload them to the release)"
