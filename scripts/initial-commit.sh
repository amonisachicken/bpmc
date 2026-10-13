#!/usr/bin/env bash
# One-shot: set the git identity, make the initial commit, show the result.
#
# Usage:
#   bash scripts/initial-commit.sh <user.name> <user.email> [--global]
#
# Without --global the identity is written to this repository only
# (.git/config), which is the least invasive option.
set -eu

NAME="${1:-}"
EMAIL="${2:-}"
SCOPE="${3:-local}"

if [ -z "$NAME" ] || [ -z "$EMAIL" ]; then
  echo "usage: bash scripts/initial-commit.sh <user.name> <user.email> [--global]" >&2
  exit 2
fi

PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT"

if [ "$SCOPE" = "--global" ]; then
  git config --global user.name "$NAME"
  git config --global user.email "$EMAIL"
  echo "== identity set globally"
else
  git config user.name "$NAME"
  git config user.email "$EMAIL"
  echo "== identity set for this repository only"
fi

echo "   name:  $(git config user.name)"
echo "   email: $(git config user.email)"
echo

echo "== staged files"
git status --short
echo

if git rev-parse --verify HEAD >/dev/null 2>&1; then
  echo "== a commit already exists, not creating another one"
else
  git commit -m "bpmc: cross-platform BPM counter TUI with zh/en UI"
fi
echo

echo "== log"
git log --oneline --decorate
echo
echo "== remote"
git remote -v
echo
echo "next: git push -u origin main"
