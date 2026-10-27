# Apple signing — what the company account's administrator provides (#282)

Releases are ad-hoc signed today, so macOS forgets "Always Allow" (Keychain) and folder access on every
upgrade. Developer ID signing and notarization fix that for every user. This page is for the person who
manages the company Apple Developer account. Everything below works from **Linux** (openssl + a browser);
no Mac is needed.

**Who:** Apple lets only the **Account Holder** of the company account create Developer ID certificates.
If the administrator is not the Account Holder, the Account Holder does step 1.

## 1. The Developer ID Application certificate (signs the app and its three binaries)

1. Make a private key and a certificate request — on the administrator's own machine; the key never leaves
   it except as the password-protected `.p12` in step 4:
   ```sh
   openssl genrsa -out developer-id.key 2048
   openssl req -new -key developer-id.key -out developer-id.csr \
     -subj "/emailAddress=<company email>/CN=<company name>/C=<country code>"
   ```
2. developer.apple.com → **Certificates, Identifiers & Profiles → Certificates → +** → **Developer ID
   Application** (choose the current "G2 Sub-CA" option) → upload `developer-id.csr` → download
   `developerID_application.cer`.
3. Note the **Team ID** (10 characters) from developer.apple.com → Membership details.
4. Bundle the certificate and its key into one password-protected file:
   ```sh
   openssl x509 -inform DER -in developerID_application.cer -out developer-id.pem
   openssl pkcs12 -export -inkey developer-id.key -in developer-id.pem \
     -out developer-id.p12 -name "Developer ID Application"
   ```
   (Choose a strong export password.) Then the signing identity's name is
   `Developer ID Application: <Company Name> (<TEAM ID>)` — shown on the certificate's page.

Not needed: "Developer ID Installer" (only for `.pkg` installers — sensei ships the `.app` and archives).

## 2. The notarization key (lets Apple's notary service accept the builds)

App Store Connect → **Users and Access → Integrations → App Store Connect API → Team Keys → +**, role
**Developer** → download the `.p8` file (**it downloads once**) and note the **Key ID** and the
**Issuer ID** shown above the list.

## 3. Hand-over — straight into GitHub, not by email or chat

github.com/sensei-hq/sensei → **Settings → Secrets and variables → Actions → New repository secret**:

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | `base64 -w0 developer-id.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | the `.p12` export password |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: <Company Name> (<TEAM ID>)` |
| `APPLE_TEAM_ID` | the Team ID |
| `APPLE_API_KEY` | the Key ID |
| `APPLE_API_ISSUER` | the Issuer ID |
| `APPLE_API_KEY_P8` | the `.p8` file's contents |

The person who builds locally also needs the `.p12` (to sign local installs with the same identity, so a
reinstall stops revoking Keychain approval) — share it and its password through the company password
manager, never as a chat or email attachment. Afterwards, delete the loose `.key`, `.csr`, `.pem` copies.

## Then (engineering, #282)

`release.yml` signs `senseid`, `sensei`, `sensei-mcp` and `Sensei.app` with the hardened runtime and fixed
identifiers, notarizes and staples; `scripts/codesign-local.sh` prefers the Developer ID identity when it is
in the login keychain.
