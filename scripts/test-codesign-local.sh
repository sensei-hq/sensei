#!/usr/bin/env bash
# codesign-local.sh is a no-op off macOS: code signing is a macOS concept, and
# `make install-debug` on Linux must not die on a missing `codesign`.
#
# A fake `uname` on PATH stands in for the OS, so this runs on any host.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

mkdir "$tmp/bin"
printf '#!/bin/sh\necho Linux\n' > "$tmp/bin/uname"
chmod +x "$tmp/bin/uname"
printf 'binary' > "$tmp/sensei-mcp"

out=$(PATH="$tmp/bin:$PATH" "$here/codesign-local.sh" "$tmp/sensei-mcp" 2>&1) || {
  echo "FAIL: codesign-local.sh exited non-zero on Linux:"; echo "$out"; exit 1; }

grep -q "skipped" <<<"$out" || { echo "FAIL: no skip notice on Linux:"; echo "$out"; exit 1; }
[[ "$(cat "$tmp/sensei-mcp")" == binary ]] || { echo "FAIL: binary was modified"; exit 1; }

echo "PASS: codesign-local.sh skips signing off macOS"
