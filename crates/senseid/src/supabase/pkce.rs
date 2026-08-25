//! PKCE primitives for the daemon's Supabase sign-in (RFC 7636).
//!
//! PKCE and not a client secret, because the daemon is a PUBLIC client: it runs
//! on the user's machine where any embedded secret can be read out of the binary.
//! The verifier is generated per attempt, never stored, and only its SHA-256
//! hash travels in the first leg — so intercepting the redirect is not enough to
//! complete the exchange.
//!
//! Everything here is pure, so the flow's correctness is testable without a
//! browser, a network, or a Supabase instance.

use base64::Engine;
use sha2::{Digest, Sha256};

/// Minimum verifier length RFC 7636 §4.1 permits.
const MIN_VERIFIER_LEN: usize = 43;
/// Maximum the RFC permits.
const MAX_VERIFIER_LEN: usize = 128;

/// A fresh code verifier: URL-safe base64 of 48 random bytes → 64 characters,
/// comfortably inside the RFC's 43–128 window.
///
/// Randomness comes from `Uuid::new_v4`, which draws from the OS CSPRNG. Three
/// uuids give 48 bytes — deliberately reusing a dependency already present
/// rather than adding a crate for 48 bytes, the same reasoning the Keychain
/// helper uses in shelling to `/usr/bin/security`.
pub fn generate_verifier() -> String {
    let mut bytes = Vec::with_capacity(48);
    for _ in 0..3 {
        bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    }
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// The S256 challenge for a verifier: `BASE64URL(SHA256(ASCII(verifier)))`,
/// unpadded.
///
/// Unpadded is not cosmetic — RFC 7636 §4.2 requires base64url WITHOUT padding,
/// and a trailing `=` makes the server's comparison fail with an opaque
/// "invalid grant" that looks like a credential problem rather than an encoding
/// one.
pub fn challenge_for(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

/// Whether a verifier is well-formed per RFC 7636 §4.1.
///
/// Worth checking rather than assuming: an out-of-range verifier is rejected at
/// the TOKEN exchange, which is the second leg — so the user completes a browser
/// sign-in and only then sees it fail, with an error that says nothing about
/// length.
pub fn is_valid_verifier(v: &str) -> bool {
    (MIN_VERIFIER_LEN..=MAX_VERIFIER_LEN).contains(&v.len())
        && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~'))
}

/// The URL to open in the user's browser to begin sign-in.
///
/// `redirect_to` must match Supabase's allow-list EXACTLY — it compares strings,
/// so `127.0.0.1` and `localhost` are different entries even though they resolve
/// to the same host.
pub fn authorize_url(
    supabase_url: &str,
    provider: &str,
    redirect_to: &str,
    challenge: &str,
    extra_scopes: &[&str],
) -> String {
    let mut url = format!(
        "{}/auth/v1/authorize?provider={}&redirect_to={}&code_challenge={}&code_challenge_method=S256",
        supabase_url.trim_end_matches('/'),
        urlencode(provider),
        urlencode(redirect_to),
        urlencode(challenge),
    );
    // `scopes` APPENDS to the provider default, verified against a live
    // instance: no param yields `user:email`, `scopes=read:org` yields
    // `user:email read:org`. So pass only the EXTRA scopes — repeating
    // `user:email` duplicates it in the consent screen the user reads.
    //
    // read:org is what the provisioning pipeline needs to see a user's
    // organisations at all; without it ProvisionTenants would find none and
    // report an empty result that looks like "you belong to no orgs".
    if !extra_scopes.is_empty() {
        url.push_str("&scopes=");
        url.push_str(&urlencode(&extra_scopes.join(" ")));
    }
    url
}

/// Percent-encode a query-parameter value.
///
/// Hand-rolled over the unreserved set rather than pulling in a crate: the
/// inputs here are a provider name, a loopback URL and a base64url challenge, so
/// the alphabet is small and known. Encoding everything outside RFC 3986's
/// unreserved set is the conservative direction — over-encoding is safe, under-
/// encoding silently corrupts the redirect.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_verifier_is_rfc_valid() {
        for _ in 0..20 {
            let v = generate_verifier();
            assert!(is_valid_verifier(&v), "not RFC-valid: {v:?} (len {})", v.len());
        }
    }

    #[test]
    fn verifiers_do_not_repeat() {
        // A reused verifier would let a captured redirect be replayed. 20 draws
        // is not proof of entropy, but a broken generator (constant, or seeded
        // per process) fails here immediately.
        let a: std::collections::HashSet<String> = (0..20).map(|_| generate_verifier()).collect();
        assert_eq!(a.len(), 20, "generator produced a duplicate");
    }

    #[test]
    fn the_challenge_matches_the_rfc_worked_example() {
        // RFC 7636 Appendix B's vector. Getting this wrong yields an opaque
        // "invalid grant" at the second leg that reads like a credential problem,
        // so it is worth pinning against the spec rather than against ourselves.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(challenge_for(verifier), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn the_challenge_is_unpadded() {
        // RFC 7636 §4.2 requires base64url WITHOUT padding. A trailing '=' is
        // rejected server-side with no hint that encoding is the cause.
        assert!(!challenge_for(&generate_verifier()).contains('='));
    }

    #[test]
    fn a_verifier_outside_the_length_window_is_rejected() {
        assert!(!is_valid_verifier(&"a".repeat(MIN_VERIFIER_LEN - 1)));
        assert!(!is_valid_verifier(&"a".repeat(MAX_VERIFIER_LEN + 1)));
        assert!(is_valid_verifier(&"a".repeat(MIN_VERIFIER_LEN)));
    }

    #[test]
    fn a_verifier_with_reserved_characters_is_rejected() {
        // '+' and '/' are standard-base64 alphabet, not base64URL. A generator
        // that used the wrong engine would produce them, and the failure would
        // otherwise surface only at the exchange.
        assert!(!is_valid_verifier(&format!("{}+", "a".repeat(MIN_VERIFIER_LEN))));
        assert!(!is_valid_verifier(&format!("{}/", "a".repeat(MIN_VERIFIER_LEN))));
    }

    #[test]
    fn the_authorize_url_encodes_the_redirect() {
        // The redirect carries `://` and `/`, all of which must survive as
        // percent-escapes — an unencoded one truncates the parameter at the
        // first `&` or confuses the allow-list comparison.
        let url = authorize_url(
            "http://127.0.0.1:54321",
            "github",
            "http://127.0.0.1:7744/api/auth/callback",
            "abc-123",
            &[],
        );
        assert!(url.starts_with("http://127.0.0.1:54321/auth/v1/authorize?"));
        assert!(url.contains("provider=github"));
        assert!(url.contains("redirect_to=http%3A%2F%2F127.0.0.1%3A7744%2Fapi%2Fauth%2Fcallback"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(!url.contains("redirect_to=http://"), "the redirect must not be raw");
    }

    #[test]
    fn extra_scopes_are_appended_not_substituted() {
        // Verified against a live instance: `scopes` ADDS to the provider
        // default rather than replacing it. Passing `user:email` again would
        // duplicate it in the consent screen; pass only what is extra.
        let url = authorize_url("http://x", "github", "http://cb", "c", &["read:org"]);
        assert!(url.contains("scopes=read%3Aorg"), "{url}");
        assert!(!url.contains("user%3Aemail"), "the default must not be repeated: {url}");

        let multi = authorize_url("http://x", "github", "http://cb", "c", &["read:org", "repo"]);
        assert!(multi.contains("scopes=read%3Aorg%20repo"), "space-separated: {multi}");
    }

    #[test]
    fn no_extra_scopes_means_no_parameter() {
        // An empty `scopes=` is not the same as omitting it, and the provider
        // default is what we want when nothing extra is asked for.
        assert!(!authorize_url("http://x", "github", "http://cb", "c", &[]).contains("scopes="));
    }

    #[test]
    fn a_trailing_slash_on_the_base_url_does_not_double_up() {
        // Config values routinely carry one; `//auth/v1/authorize` 404s.
        let url = authorize_url("http://127.0.0.1:54321/", "github", "http://x", "c", &[]);
        assert!(url.contains("54321/auth/v1/authorize"), "{url}");
        assert!(!url.contains("54321//auth"), "{url}");
    }
}
