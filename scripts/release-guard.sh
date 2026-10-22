#!/usr/bin/env bash
# A release is cut only from `main` (wave 0b, user 2026-10-22): one version bump,
# one tag, one release run — and the tag is what `release.yml` publishes and
# deploys from (crates, packages, apps, the dōjō Worker, database changes).
# develop is for scanning, coverage and tests; it never publishes.
#
# Refuses unless the checkout is on `main`, has no uncommitted changes, and is
# exactly what `origin/main` holds — so a tag never lands on develop or on a
# commit nobody else has. `make bump` runs this first.
set -uo pipefail

branch=$(git rev-parse --abbrev-ref HEAD)
if [ "$branch" != "main" ]; then
  echo "release-guard: on '$branch' — a release is cut from main. Merge develop → main (PR, green), then: git checkout main && git pull && make bump v=…" >&2
  exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
  echo "release-guard: the working tree is not clean — commit or stash first." >&2
  exit 1
fi

if ! git fetch -q origin main; then
  echo "release-guard: could not fetch origin/main." >&2
  exit 1
fi
if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
  echo "release-guard: main is not what origin/main holds — pull (or push) first so the tag marks a commit everyone has." >&2
  exit 1
fi

echo "release-guard: main is clean and matches origin/main."
