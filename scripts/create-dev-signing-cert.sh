#!/usr/bin/env bash
# Create the self-signed `sensei-dev` code-signing identity in the login
# keychain, once per machine (see scripts/codesign-local.sh for why). macOS
# asks for your login password to trust it — this is meant to be run by you.
set -euo pipefail

if security find-identity -v -p codesigning 2>/dev/null | grep -q '"sensei-dev"'; then
  echo "sensei-dev already exists:"; security find-identity -v -p codesigning | grep '"sensei-dev"'; exit 0
fi

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
cat > "$tmp/cfg" <<'CFG'
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = sensei-dev
[ext]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
CFG
openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -config "$tmp/cfg" \
  -keyout "$tmp/key.pem" -out "$tmp/cert.pem" >/dev/null 2>&1
openssl pkcs12 -export -inkey "$tmp/key.pem" -in "$tmp/cert.pem" -name sensei-dev \
  -passout pass:sensei-dev -out "$tmp/id.p12" -legacy 2>/dev/null \
  || openssl pkcs12 -export -inkey "$tmp/key.pem" -in "$tmp/cert.pem" -name sensei-dev \
       -passout pass:sensei-dev -out "$tmp/id.p12"
keychain="${KEYCHAIN:-$HOME/Library/Keychains/login.keychain-db}"
security import "$tmp/id.p12" -k "$keychain" -P sensei-dev -T /usr/bin/codesign
security add-trusted-cert -p codeSign -k "$keychain" "$tmp/cert.pem"
security find-identity -v -p codesigning | grep '"sensei-dev"'
echo "sensei-dev created. The next 'make install*' signs with it; allow each keychain prompt once."
