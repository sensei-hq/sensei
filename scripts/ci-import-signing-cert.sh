#!/usr/bin/env bash
# Put the Developer ID certificate into a throwaway keychain for this CI job (#282).
#
# Reads $APPLE_CERTIFICATE (the base64 .p12) and $APPLE_CERTIFICATE_PASSWORD from the
# release secrets. The keychain lives under $RUNNER_TEMP with a random password, is
# added to the search list so `codesign` finds the identity, and lets codesign use the
# key without a prompt. The decoded .p12 is removed at once. Fails unless a
# "Developer ID Application" identity is then available.
set -euo pipefail

: "${APPLE_CERTIFICATE:?APPLE_CERTIFICATE (the base64 .p12) is not set}"
: "${APPLE_CERTIFICATE_PASSWORD:?APPLE_CERTIFICATE_PASSWORD is not set}"

work="${RUNNER_TEMP:?RUNNER_TEMP is not set}"
keychain="$work/sensei-signing.keychain-db"
p12="$work/developer-id-$$.p12"
trap 'rm -f "$p12"' EXIT

keychain_password=$(openssl rand -base64 24)
(umask 077 && printf '%s' "$APPLE_CERTIFICATE" | base64 --decode > "$p12")

security create-keychain -p "$keychain_password" "$keychain"
security set-keychain-settings -lut 3600 "$keychain"
security unlock-keychain -p "$keychain_password" "$keychain"
security import "$p12" -k "$keychain" -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple: -s -k "$keychain_password" "$keychain" > /dev/null
# Searched first, the login keychain kept after it.
existing=$(security list-keychains -d user | tr -d '"' | xargs)
# shellcheck disable=SC2086 # the existing list is word-split on purpose
security list-keychains -d user -s "$keychain" $existing

ids=$(security find-identity -v -p codesigning "$keychain")
grep -q '"Developer ID Application: ' <<<"$ids" || {
  echo "ci-import-signing-cert: no Developer ID Application identity in the certificate" >&2
  exit 1
}
echo "ci-import-signing-cert: Developer ID identity ready"
