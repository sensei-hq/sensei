#!/usr/bin/env bash
# Notarize signed sensei binaries with Apple's notary service (#282).
#
#   scripts/ci-notarize.sh <binary>...
#
# Reads the App Store Connect API key from the release secrets: $APPLE_API_KEY (the
# key id), $APPLE_API_ISSUER and $APPLE_API_KEY_P8 (the .p8 contents). The key is
# written to a private file for the one call and removed after. A bare binary cannot
# be stapled, so Gatekeeper checks its ticket online; the submission still has to be
# Accepted — `notarytool submit --wait` can finish on a rejected one, so the status is
# read and anything else fails the release with Apple's own log.
set -euo pipefail

: "${APPLE_API_KEY:?APPLE_API_KEY (the key id) is not set}"
: "${APPLE_API_ISSUER:?APPLE_API_ISSUER is not set}"
: "${APPLE_API_KEY_P8:?APPLE_API_KEY_P8 (the .p8 contents) is not set}"
[[ $# -gt 0 ]] || { echo "ci-notarize: no binaries given" >&2; exit 2; }

work="${RUNNER_TEMP:-$(mktemp -d)}"
key="$work/notary-$$.p8"
zip="$work/notarize-$$.zip"
trap 'rm -f "$key" "$zip"' EXIT

(umask 077 && printf '%s\n' "$APPLE_API_KEY_P8" > "$key")
ditto -c -k --keepParent "$@" "$zip"

result=$(xcrun notarytool submit "$zip" \
  --key "$key" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER" \
  --wait --output-format json)
id=$(sed -n 's/.*"id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' <<<"$result")
status=$(sed -n 's/.*"status"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' <<<"$result")

if [[ "$status" != "Accepted" ]]; then
  echo "ci-notarize: Apple did not accept submission ${id:-?} (status: ${status:-unknown})" >&2
  [[ -n "$id" ]] && xcrun notarytool log "$id" --key "$key" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER" >&2 || true
  exit 1
fi
echo "ci-notarize: accepted ($id) — $# binary(ies)"
