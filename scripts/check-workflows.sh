#!/usr/bin/env bash
# Structural check of the GitHub Actions workflows.
#
# GitHub validates a workflow when it is pushed, but a broken file means a slow
# round trip; this catches the mistakes that are easy to make by hand. PyYAML is
# used when available, otherwise a small indentation sanity check runs instead.
set -u
PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT" || exit 1

# On this machine python3 lives in the MSYS2 clang64 prefix and is not on the
# login shell's PATH, so look there too.
PY=""
for candidate in python3 /clang64/bin/python3 /mingw64/bin/python3 /usr/bin/python3; do
  if command -v "$candidate" >/dev/null 2>&1; then
    PY="$candidate"
    break
  fi
done
if [ -z "$PY" ]; then
  echo "no python3 found; skipping structural check"
  exit 0
fi

"$PY" - <<'PY'
import pathlib
import sys

try:
    import yaml
except ImportError:
    yaml = None


def deep_check(path, doc):
    """Checks that only make sense once the file parsed."""
    problems = []
    for key in ("name", "on", "jobs"):
        if key not in doc:
            problems.append(f"{path}: missing top-level '{key}'")
    print(f"== {path}")
    print(f"   name: {doc.get('name')}")
    print(f"   on:   {doc.get('on')}")
    for job_name, job in (doc.get("jobs") or {}).items():
        steps = job.get("steps") or []
        print(f"   job {job_name}: {len(steps)} steps on {job.get('runs-on')}")
        for step in steps:
            if not isinstance(step, dict):
                problems.append(f"{path}/{job_name}: step is not a mapping")
                continue
            has_uses, has_run = "uses" in step, "run" in step
            if has_uses == has_run:
                problems.append(f"{path}/{job_name}: step needs exactly one of uses/run")
            if has_run and "shell" not in step:
                problems.append(
                    f"{path}/{job_name}: run step without explicit shell "
                    f"(default differs between Linux and Windows runners)"
                )
    return problems


def shallow_check(path, text):
    """Indentation sanity check used when PyYAML is unavailable."""
    problems = []
    for lineno, raw in enumerate(text.splitlines(), 1):
        if "\t" in raw:
            problems.append(f"{path}:{lineno}: literal tab (YAML only allows spaces)")
        if raw.rstrip() != raw:
            problems.append(f"{path}:{lineno}: trailing whitespace")
        stripped = raw.strip()
        if stripped and not stripped.startswith("#"):
            indent = len(raw) - len(raw.lstrip(" "))
            if indent % 2 != 0:
                problems.append(f"{path}:{lineno}: odd indentation ({indent} spaces)")
    # Top-level keys we rely on must appear at column 0.
    for key in ("name:", "on:", "jobs:"):
        if not any(line.startswith(key) for line in text.splitlines()):
            problems.append(f"{path}: no top-level '{key}'")
    return problems


problems = []
files = sorted(pathlib.Path(".").glob(".github/workflows/*.yml"))
if not files:
    problems.append("no workflow files found")

for path in files:
    text = path.read_text(encoding="utf-8")
    if yaml is None:
        print(f"== {path} (shallow check, no pyyaml)")
        problems += shallow_check(path, text)
    else:
        try:
            doc = yaml.safe_load(text)
        except Exception as exc:
            problems.append(f"{path}: YAML parse error: {exc}")
            continue
        if not isinstance(doc, dict):
            problems.append(f"{path}: top level is not a mapping")
            continue
        problems += deep_check(path, doc)

print()
if problems:
    print(f"PROBLEMS ({len(problems)}):")
    for p in problems:
        print(" -", p)
    sys.exit(1)
print("workflows look structurally fine" + ("" if yaml else " (shallow check only)"))
PY
