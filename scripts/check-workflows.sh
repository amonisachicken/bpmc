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
    # `on:` is parsed as the boolean True by YAML 1.1 parsers, so match either.
    if "name" not in doc:
        problems.append(f"{path}: missing top-level 'name'")
    if not any(k in doc for k in ("on", True)):
        problems.append(f"{path}: missing top-level 'on'")
    if "jobs" not in doc:
        problems.append(f"{path}: missing top-level 'jobs'")
    print(f"== {path}")
    print(f"   name: {doc.get('name')}")
    print(f"   on:   {doc.get('on') if 'on' in doc else doc.get(True)}")
    for job_name, job in (doc.get("jobs") or {}).items():
        steps = job.get("steps") or []
        runs_on = str(job.get("runs-on", ""))
        matrix = ((job.get("strategy") or {}).get("matrix") or {})
        # `runs-on` is often an expression, so expand the matrix (and its plain
        # list values) to find out whether any leg runs on Windows.
        candidates = [runs_on] + [str(v) for v in matrix.get("os", []) or []]
        candidates += [str(e.get("os", "")) for e in matrix.get("include", []) or []]
        windows_possible = any("windows" in c.lower() for c in candidates)
        print(f"   job {job_name}: {len(steps)} steps on {runs_on}"
              f"{' (windows possible)' if windows_possible else ''}")
        for step in steps:
            if not isinstance(step, dict):
                problems.append(f"{path}/{job_name}: step is not a mapping")
                continue
            has_uses, has_run = "uses" in step, "run" in step
            if has_uses == has_run:
                problems.append(f"{path}/{job_name}: step needs exactly one of uses/run")
            # GitHub's default shell is bash on Linux/macOS but pwsh on Windows,
            # so a missing shell is only a problem where a Windows leg can run.
            if has_run and "shell" not in step and windows_possible:
                problems.append(
                    f"{path}/{job_name}: run step without an explicit shell, but this "
                    f"job can run on Windows (the default there is pwsh)"
                )
        problems += check_release_matrix(path, job_name, job)
    return problems


def check_release_matrix(path, job_name, job):
    """Apply the matrix checks to a parsed job."""
    problems = []
    entries = (((job.get("strategy") or {}).get("matrix") or {}).get("include")) or []
    for entry in entries:
        problems += check_matrix_entry(path, job_name, entry)
    return problems


def shallow_check(path, text):
    """Indentation sanity check plus a matrix check, used without PyYAML."""
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

    # Minimal matrix reader: enough for the flat `include:` lists used here.
    # Only entries carrying both `target` and `bin` are treated as matrix rows,
    # which keeps step definitions (also `- name: ...` lists) out of the way.
    import re

    candidates, current = [], None
    for raw in text.splitlines():
        stripped = raw.strip()
        if re.match(r"^-?\s*-\s+name:\s*\S+", stripped) or stripped.startswith("- name:"):
            if current:
                candidates.append(current)
            current = {"name": stripped.split("name:", 1)[1].strip()}
            continue
        if current is not None:
            m = re.match(r"(os|target|bin|ext):\s*(\S+)", stripped)
            if m:
                current[m.group(1)] = m.group(2)
            elif stripped.startswith("- "):
                # A sibling list item, not a field of this entry.
                candidates.append(current)
                current = None
    if current:
        candidates.append(current)

    for entry in candidates:
        if "target" in entry and "bin" in entry:
            problems += check_matrix_entry(path, "release matrix", entry)
    return problems


def check_matrix_entry(path, job_name, entry):
    """Shared by the PyYAML and the minimal path."""
    problems = []
    name = entry.get("name", "?")
    target = entry.get("target", "")
    binary = entry.get("bin", "")
    if not target or not binary:
        problems.append(f"{path}/{job_name}/{name}: needs both target and bin")
        return problems
    expected = f"target/{target}/release/"
    if not binary.startswith(expected):
        problems.append(
            f"{path}/{job_name}/{name}: bin is '{binary}' but --target {target} "
            f"writes to '{expected}...'"
        )
    if "windows" in target and not binary.endswith(".exe"):
        problems.append(f"{path}/{job_name}/{name}: windows bin must end in .exe")
    if "windows" not in target and binary.endswith(".exe"):
        problems.append(f"{path}/{job_name}/{name}: non-windows bin must not end in .exe")
    print(f"   matrix {name}: {target} -> {binary} [ok]")
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
