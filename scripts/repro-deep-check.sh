#!/usr/bin/env bash
# Feed check-workflows.sh's real validation code a hand-written document that
# matches exactly what PyYAML produces for these workflow files, so the deep
# branch can be exercised on a machine that has no PyYAML.
#
# Why this exists: on the Ubuntu runner python3 ships PyYAML, so the deep branch
# runs there and the shallow branch runs here; this closes that gap.
set -u
export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH
PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT" || exit 1

STUB_DIR="${TMPDIR:-/tmp}/cw-fixture"
rm -rf "$STUB_DIR"
mkdir -p "$STUB_DIR"

# A JSON file is loaded by the stub module and handed to safe_load, because the
# checker calls yaml.safe_load(text) with the workflow file's text.
cat > "$STUB_DIR/yaml.py" <<'STUB'
import json
import pathlib

FIXTURE = pathlib.Path(__file__).with_name("fixture.json")


def safe_load(_text):
    """Return the pre-parsed document for whichever file was read."""
    return json.loads(FIXTURE.read_text(encoding="utf-8"))
STUB

# Exactly the structure PyYAML yields: note that `on:` parses to the boolean
# True under YAML 1.1, which is why the checker looks for the "on" key only in
# the shallow path.
cat > "$STUB_DIR/fixture.json" <<'JSON'
{
  "name": "CI",
  "on": {"push": {"branches": ["main"]}, "pull_request": null},
  "jobs": {
    "test": {
      "name": "test / ${{ matrix.os }}",
      "runs-on": "${{ matrix.os }}",
      "strategy": {"fail-fast": false, "matrix": {"os": ["ubuntu-latest", "windows-latest", "macos-latest"]}},
      "steps": [
        {"uses": "actions/checkout@v4"},
        {"name": "Install Rust", "uses": "dtolnay/rust-toolchain@stable", "with": {"components": "clippy, rustfmt"}},
        {"name": "Cache cargo", "uses": "Swatinem/rust-cache@v2"},
        {"name": "Check the workflows themselves", "shell": "bash", "run": "bash scripts/check-workflows.sh"},
        {"name": "Format check", "shell": "bash", "run": "cargo fmt --all -- --check"},
        {"name": "Clippy", "shell": "bash", "run": "cargo clippy --all-targets -- -D warnings"},
        {"name": "Test", "shell": "bash", "run": "cargo test --all-targets"},
        {"name": "Headless render smoke test", "shell": "bash", "run": "cargo run --example render_demo -- --check"},
        {"name": "Build release binary", "shell": "bash", "run": "cargo build --release"},
        {"name": "Upload binary", "uses": "actions/upload-artifact@v4",
         "with": {"name": "bpmc-${{ matrix.os }}", "path": "target/release/bpmc\ntarget/release/bpmc.exe\n", "if-no-files-found": "ignore"}}
      ]
    }
  }
}
JSON

echo "== deep branch against the CI fixture"
PYTHONPATH="$STUB_DIR" /clang64/bin/python3 - <<'PY'
import io
import pathlib
import runpy
import sys

# Run the checker's embedded program with the stub yaml module in place, then
# report what it decided about the fixture.
source = pathlib.Path("scripts/check-workflows.sh").read_text(encoding="utf-8")
program = source.split("<<'PY'\n", 1)[1].rsplit("\nPY\n", 1)[0]

try:
    exec(compile(program, "check-workflows-embedded", "exec"), {"__name__": "__main__"})
except SystemExit as exc:
    print(f"embedded program exited with {exc.code}")
PY

echo
echo "== now the release fixture (4-target matrix)"
cat > "$STUB_DIR/fixture.json" <<'JSON'
{
  "name": "Release",
  "on": {"push": {"tags": ["v*"]}, "workflow_dispatch": {"inputs": {"dry_run": {"description": "x", "type": "boolean", "default": true}}}},
  "permissions": {"contents": "write"},
  "jobs": {
    "tag": {
      "name": "check tag",
      "runs-on": "ubuntu-latest",
      "outputs": {"version": "${{ steps.check.outputs.version }}"},
      "steps": [
        {"uses": "actions/checkout@v4"},
        {"name": "Tag must match Cargo.toml version", "id": "check", "shell": "bash", "run": "echo hi"}
      ]
    },
    "build": {
      "name": "${{ matrix.name }}",
      "needs": "tag",
      "runs-on": "${{ matrix.os }}",
      "strategy": {
        "fail-fast": false,
        "matrix": {
          "include": [
            {"name": "linux-x86_64", "os": "ubuntu-latest", "target": "x86_64-unknown-linux-gnu", "bin": "target/x86_64-unknown-linux-gnu/release/bpmc", "ext": "tar.gz"},
            {"name": "macos-x86_64", "os": "macos-latest", "target": "x86_64-apple-darwin", "bin": "target/x86_64-apple-darwin/release/bpmc", "ext": "tar.gz"},
            {"name": "macos-aarch64", "os": "macos-latest", "target": "aarch64-apple-darwin", "bin": "target/aarch64-apple-darwin/release/bpmc", "ext": "tar.gz"},
            {"name": "windows-x86_64", "os": "windows-latest", "target": "x86_64-pc-windows-msvc", "bin": "target/x86_64-pc-windows-msvc/release/bpmc.exe", "ext": "zip"}
          ]
        }
      },
      "steps": [
        {"uses": "actions/checkout@v4"},
        {"name": "Install Rust", "uses": "dtolnay/rust-toolchain@stable", "with": {"targets": "${{ matrix.target }}"}},
        {"name": "Cache cargo", "uses": "Swatinem/rust-cache@v2", "with": {"key": "${{ matrix.target }}"}},
        {"name": "Build", "shell": "bash", "run": "cargo build"},
        {"name": "Check the binary exists", "shell": "bash", "run": "test -f x"},
        {"name": "Package (unix)", "if": "runner.os != 'Windows'", "shell": "bash", "run": "tar x"},
        {"name": "Package (windows)", "if": "runner.os == 'Windows'", "shell": "pwsh", "run": "Compress-Archive"},
        {"name": "Keep the packages as run artifacts", "uses": "actions/upload-artifact@v4", "with": {"name": "${{ matrix.name }}", "path": "dist/*", "if-no-files-found": "error"}},
        {"name": "Upload to the release", "if": "github.ref_type == 'tag'", "shell": "bash", "run": "gh release upload"}
      ]
    }
  }
}
JSON

PYTHONPATH="$STUB_DIR" /clang64/bin/python3 - <<'PY'
import pathlib

source = pathlib.Path("scripts/check-workflows.sh").read_text(encoding="utf-8")
program = source.split("<<'PY'\n", 1)[1].rsplit("\nPY\n", 1)[0]
try:
    exec(compile(program, "check-workflows-embedded", "exec"), {"__name__": "__main__"})
except SystemExit as exc:
    print(f"embedded program exited with {exc.code}")
PY

rm -rf "$STUB_DIR"
