//! Bearer-token auth + role-floor enforcement.

use crate::api::AppState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Member = 0,
    Publisher = 1,
    Admin = 2,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "member" => Some(Role::Member),
            "publisher" => Some(Role::Publisher),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }
}

/// True when `have` meets-or-exceeds the required `floor`.
pub fn role_satisfies(have: Role, floor: Role) -> bool {
    have >= floor
}

/// The resolved caller, attached to the request by `require`.
#[derive(Clone)]
pub struct AuthCaller {
    pub member_id: uuid::Uuid,
    pub name: String,
    pub role: Role,
}

fn bearer(req: &Request) -> Option<String> {
    req.headers()
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

/// Middleware: resolve the bearer key → 401 if invalid; attach `AuthCaller`.
pub async fn require(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let key = bearer(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    let caller = state
        .store
        .find_member_by_key(&key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let role = Role::parse(&caller.role).ok_or(StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(AuthCaller {
        member_id: caller.member_id,
        name: caller.name,
        role,
    });
    Ok(next.run(req).await)
}
