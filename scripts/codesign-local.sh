#!/usr/bin/env bash
# Sign locally-built sensei binaries with a STABLE identity.
#
#   scripts/codesign-local.sh <binary>...
#
# macOS ties a keychain item's "Always Allow", and folder-access (TCC) grants,
# to the program's code signature. An ad-hoc signature (`--sign -`) changes
# with every build, so each `make install*` looked like a new program and the
# keychain asked again. With a stable identity and a fixed identifier, the
# grant survives rebuilds.
#
# Identity: $SENSEI_SIGN_IDENTITY if set, else `sensei-dev` when it is in the
# keychain (create it once with `make dev-signing-cert`), else ad-hoc — said
# out loud. The identifier is always `dev.sensei.<binary name>`, never the
# hash-bearing default.
set -euo pipefail

identity="${SENSEI_SIGN_IDENTITY:-}"
if [[ -z "$identity" ]]; then
  # Read the whole listing, then search it. `security … | grep -q` under
  # pipefail is a race: grep exits at the match, `security` SIGPIPEs on its next
  # line, and the pipeline reads as "no identity".
  ids=$(security find-identity -v -p codesigning 2>/dev/null || true)
  if grep -q '"sensei-dev"' <<<"$ids"; then
    identity="sensei-dev"
  else
    identity="-"
    echo "codesign-local: no 'sensei-dev' signing identity — signing ad-hoc, so keychain and folder"
    echo "  permissions will be asked for again after each build. Run 'make dev-signing-cert' once."
  fi
fi

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
  codesign --sign "$identity" --identifier "$id" --options runtime --force "$f"
done
echo "codesign-local: signed $# binary(ies) as '$identity'"
