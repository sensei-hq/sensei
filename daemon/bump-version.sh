#!/usr/bin/env bash
set -euo pipefail

VERSION=${1:?usage: ./bump-version.sh X.Y.Z}

# Update all crate versions in the workspace
for toml in crates/*/Cargo.toml; do
  sed -i '' "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" "$toml"
done

cargo check -q 2>&1 | grep -v "^$" || true

git add crates/*/Cargo.toml Cargo.lock
git commit -m "chore: bump version to v$VERSION"
git tag "v$VERSION"

echo "Tagged v$VERSION — push with: git push && git push --tags"
