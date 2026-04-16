#!/usr/bin/env bash
# Updates Homebrew formula SHA256 values from a sha256sums.txt file.
# Usage: update-homebrew-sha.sh <sha256sums.txt>
set -euo pipefail

SUMS_FILE="${1:?Usage: update-homebrew-sha.sh <sha256sums.txt>}"
FORMULA="homebrew/Formula/sensei.rb"

if [ ! -f "$SUMS_FILE" ]; then
  echo "Error: $SUMS_FILE not found" >&2
  exit 1
fi

while IFS=' ' read -r sha file; do
  # Strip any leading path or * prefix from filename
  file="$(basename "$file")"
  case "$file" in
    *macos-arm64*)  key="ARM64" ;;
    *macos-x86_64*) key="X86_64" ;;
    *linux-arm64*)  key="LINUX_ARM64" ;;
    *linux-x86_64*) key="LINUX_X86_64" ;;
    *) continue ;;
  esac
  placeholder="REPLACE_WITH_${key}_SHA256"
  # Replace either the placeholder or a previous 64-char hex SHA (portable sed -i)
  if [[ "$OSTYPE" == darwin* ]]; then
    sed -i '' "s|${placeholder}|${sha}|" "$FORMULA"
  else
    sed -i "s|${placeholder}|${sha}|" "$FORMULA"
  fi
  echo "Updated $key: $sha"
done < "$SUMS_FILE"
