//! Unit tests for the Supabase-JWT verification path, driven entirely by
//! SYNTHETIC tokens signed with the test secret — no running Supabase needed.

use dojo_mind::auth::{
    verify_supabase_jwt, DEFAULT_SUPABASE_JWT_AUD, DEFAULT_SUPABASE_JWT_SECRET,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

fn now() -> usize {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
}

/// Sign a synthetic HS256 token from arbitrary claims with `secret`.
fn sign(secret: &str, claims: &serde_json::Value) -> String {
    encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

/// A well-formed Supabase-shaped access token (aud + exp in the future).
fn good_claims(sub: &str) -> serde_json::Value {
    serde_json::json!({
        "sub": sub,
        "aud": DEFAULT_SUPABASE_JWT_AUD,
        "exp": now() + 3600,
        "email": "jerry@example.com",
        "role": "authenticated",
    })
}

#[test]
fn good_token_verifies_and_extracts_claims() {
    let token = sign(DEFAULT_SUPABASE_JWT_SECRET, &good_claims("user-123"));
    let claims = verify_supabase_jwt(&token, DEFAULT_SUPABASE_JWT_SECRET, DEFAULT_SUPABASE_JWT_AUD)
        .expect("a well-formed token must verify");
    assert_eq!(claims.sub, "user-123");
    assert_eq!(claims.email.as_deref(), Some("jerry@example.com"));
    assert_eq!(claims.role.as_deref(), Some("authenticated"));
}

#[test]
fn bad_signature_is_rejected() {
    // Signed with a different secret than we verify against.
    let token = sign("some-other-secret-that-is-also-long-enough-xx", &good_claims("u"));
    assert!(
        verify_supabase_jwt(&token, DEFAULT_SUPABASE_JWT_SECRET, DEFAULT_SUPABASE_JWT_AUD).is_err(),
        "a token signed with the wrong secret must be rejected"
    );
}

#[test]
fn expired_token_is_rejected() {
    // Well past the 60s default leeway.
    let claims = serde_json::json!({
        "sub": "u",
        "aud": DEFAULT_SUPABASE_JWT_AUD,
        "exp": now() - 3600,
    });
    let token = sign(DEFAULT_SUPABASE_JWT_SECRET, &claims);
    let err = verify_supabase_jwt(&token, DEFAULT_SUPABASE_JWT_SECRET, DEFAULT_SUPABASE_JWT_AUD)
        .expect_err("an expired token must be rejected");
    assert_eq!(
        *err.kind(),
        jsonwebtoken::errors::ErrorKind::ExpiredSignature
    );
}

#[test]
fn wrong_audience_is_rejected() {
    let claims = serde_json::json!({
        "sub": "u",
        "aud": "some-other-audience",
        "exp": now() + 3600,
    });
    let token = sign(DEFAULT_SUPABASE_JWT_SECRET, &claims);
    let err = verify_supabase_jwt(&token, DEFAULT_SUPABASE_JWT_SECRET, DEFAULT_SUPABASE_JWT_AUD)
        .expect_err("a token with the wrong audience must be rejected");
    assert_eq!(
        *err.kind(),
        jsonwebtoken::errors::ErrorKind::InvalidAudience
    );
}

#[test]
fn missing_required_claims_is_rejected() {
    // No `sub` — a required spec claim.
    let claims = serde_json::json!({
        "aud": DEFAULT_SUPABASE_JWT_AUD,
        "exp": now() + 3600,
    });
    let token = sign(DEFAULT_SUPABASE_JWT_SECRET, &claims);
    assert!(
        verify_supabase_jwt(&token, DEFAULT_SUPABASE_JWT_SECRET, DEFAULT_SUPABASE_JWT_AUD).is_err(),
        "a token missing the required `sub` claim must be rejected"
    );
}

#[test]
fn garbage_token_is_rejected() {
    assert!(
        verify_supabase_jwt("not-a-jwt", DEFAULT_SUPABASE_JWT_SECRET, DEFAULT_SUPABASE_JWT_AUD)
            .is_err(),
        "a malformed token must be rejected"
    );
}
