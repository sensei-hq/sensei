//! PKCE primitives for the daemon's Supabase sign-in (RFC 7636).
//!
//! PKCE and not a client secret, because the daemon is a PUBLIC client: it runs
//! on the user's machine where any embedded secret can be read out of the binary.
//! The verifier is generated per attempt, never stored, and only its SHA-256
//! hash travels in the first leg — so intercepting the redirect is not enough to
//! complete the exchange.
//!
//! The authorize URL itself is dōjō's to build (`/v1/auth/cli/start`), so this
//! module keeps only the parts the daemon must own: the verifier is a SECRET and
//! generating it anywhere but here would defeat the point of PKCE.
//!
//! Everything here is pure, so the flow's correctness is testable without a
//! browser, a network, or a dōjō.

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
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_verifier_is_rfc_valid() {
        for _ in 0..20 {
            let v = generate_verifier();
            assert!(
                is_valid_verifier(&v),
                "not RFC-valid: {v:?} (len {})",
                v.len()
            );
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
        assert_eq!(
            challenge_for(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
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
        assert!(!is_valid_verifier(&format!(
            "{}+",
            "a".repeat(MIN_VERIFIER_LEN)
        )));
        assert!(!is_valid_verifier(&format!(
            "{}/",
            "a".repeat(MIN_VERIFIER_LEN)
        )));
    }
}
