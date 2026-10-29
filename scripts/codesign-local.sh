#!/usr/bin/env bash
# Sign sensei binaries with a STABLE identity — local installs, and the release (CI
# names the Developer ID through $SENSEI_SIGN_IDENTITY, #282).
#
#   scripts/codesign-local.sh <binary>...
#
# macOS ties a keychain item's "Always Allow", and folder-access (TCC) grants,
# to the program's code signature. An ad-hoc signature (`--sign -`) changes
# with every build, so each `make install*` looked like a new program and the
# keychain asked again. With a stable identity and a fixed identifier, the
# grant survives rebuilds.
#
# Identity: $SENSEI_SIGN_IDENTITY if set, else a `Developer ID Application`
# identity in the keychain (the company's, the one users' grants are keyed on),
# else `sensei-dev` (create it once with `make dev-signing-cert`), else ad-hoc —
# said out loud. A Developer ID signature carries a secure timestamp, which
# notarization requires; the self-signed one does not, so a local install needs
# no network. The identifier is always `dev.sensei.<binary name>`, never the
# hash-bearing default.
set -euo pipefail

identity="${SENSEI_SIGN_IDENTITY:-}"
if [[ -z "$identity" ]]; then
  # Read the whole listing, then search it. `security … | grep -q` under
  # pipefail is a race: grep exits at the match, `security` SIGPIPEs on its next
  # line, and the pipeline reads as "no identity".
  ids=$(security find-identity -v -p codesigning 2>/dev/null || true)
  developer_id=$(grep -o '"Developer ID Application: [^"]*"' <<<"$ids" | head -n 1 | tr -d '"' || true)
  if [[ -n "$developer_id" ]]; then
    identity="$developer_id"
  elif grep -q '"sensei-dev"' <<<"$ids"; then
    identity="sensei-dev"
  else
    identity="-"
    echo "codesign-local: no 'sensei-dev' signing identity — signing ad-hoc, so keychain and folder"
    echo "  permissions will be asked for again after each build. Run 'make dev-signing-cert' once."
  fi
fi

timestamp=()
[[ "$identity" == "Developer ID Application:"* ]] && timestamp=(--timestamp)

for f in "$@"; do
  name=$(basename "$f")
  if [[ "$name" == *.app ]]; then
    # A bundle (the e2e Sensei.app, #284): its own namespace, since
    # `dev.sensei.sensei` is the CLI's and grants are keyed on identity.
    name=$(tr '[:upper:]' '[:lower:]' <<<"${name%.app}")
    id="dev.sensei.app.$name"
  else
    id="dev.sensei.$name"
  fi
  codesign --sign "$identity" --identifier "$id" --options runtime ${timestamp[@]+"${timestamp[@]}"} --force "$f"
done
echo "codesign-local: signed $# binary(ies) as '$identity'"
