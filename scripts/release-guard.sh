#!/usr/bin/env bash
# A release is two steps (wave 0b, user 2026-10-22), because `main` is
# protected — a push to it needs its status checks, so a version commit cannot
# be pushed straight onto it:
#
#   bump     on develop: the version change is a commit that reaches main through
#            the develop → main PR, like any other change.
#   release  on main: tag main's head with VERSION and push ONLY the tag, which
#            runs release.yml (crates, packages, apps, the dōjō deploy). One
#            version, one tag, one release run.
#
#   release-guard.sh bump|release
set -uo pipefail

step="${1:-}"
case "$step" in
  bump) branch_needed=develop ;;
  release) branch_needed=main ;;
  *) echo "usage: release-guard.sh bump|release" >&2; exit 2 ;;
esac

branch=$(git rev-parse --abbrev-ref HEAD)
if [ "$branch" != "$branch_needed" ]; then
  if [ "$step" = bump ]; then
    echo "release-guard: on '$branch' — the version is bumped on develop and reaches main through the PR." >&2
  else
    echo "release-guard: on '$branch' — a release is tagged on main. Merge the develop → main PR (green), then: git checkout main && git pull && make release" >&2
  fi
  exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
  echo "release-guard: the working tree is not clean — commit or stash first." >&2
  exit 1
fi

if ! git fetch -q origin "$branch_needed" --tags; then
  echo "release-guard: could not fetch origin/$branch_needed." >&2
  exit 1
fi
if [ "$(git rev-parse HEAD)" != "$(git rev-parse "origin/$branch_needed")" ]; then
  echo "release-guard: $branch_needed is not what origin/$branch_needed holds — pull (or push) first." >&2
  exit 1
fi

if [ "$step" = release ]; then
  tag="v$(tr -d '[:space:]' < VERSION)"
  if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
    echo "release-guard: $tag already exists — bump the version on develop first (make bump v=patch|minor), then merge the PR." >&2
    exit 1
  fi
fi

echo "release-guard: $branch_needed is clean and matches origin/$branch_needed."
