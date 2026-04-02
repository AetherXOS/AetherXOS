use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
};
use crate::state::AppState;

/// Rocket request guard: resolves the caller's role from the `X-AetherCore-Token` header.
/// Always succeeds — returns "anonymous" when no valid token is presented.
pub struct ResolvedRole(pub String);
pub struct OptionalIdempotencyKey(pub Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ResolvedRole {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = req
            .headers()
            .get_one("X-AetherCore-Token")
            .or_else(|| req.headers().get_one("X-HyperCore-Token"))
            .unwrap_or("")
            .to_string();

        let state = req.rocket().state::<AppState>().unwrap();
        let inner = state.read();

        if inner.unsafe_no_auth {
            return Outcome::Success(ResolvedRole("admin".into()));
        }

        let role = inner.resolve_role(&token).to_string();
        Outcome::Success(ResolvedRole(role))
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for OptionalIdempotencyKey {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let key = req.headers().get_one("X-Idempotency-Key").map(|value| value.to_string());
        Outcome::Success(OptionalIdempotencyKey(key))
    }
}

/// Request guard that enforces a minimum role requirement.
/// Produces a 401 JSON error if the token is missing/invalid,
/// or if the resolved role is below the minimum.
pub struct RequireViewer;
pub struct RequireOperator(pub String);
pub struct RequireAdmin;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequireViewer {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ()> {
        let token = req
            .headers()
            .get_one("X-AetherCore-Token")
            .or_else(|| req.headers().get_one("X-HyperCore-Token"))
            .unwrap_or("")
            .to_string();

        let state = req.rocket().state::<AppState>().unwrap();
        let inner = state.read();

        if inner.unsafe_no_auth {
            return Outcome::Success(RequireViewer);
        }

        let role = inner.resolve_role(&token).to_string();
        if crate::state::Inner::role_at_least(&role, "viewer") {
            Outcome::Success(RequireViewer)
        } else {
            Outcome::Error((Status::Unauthorized, ()))
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequireOperator {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ()> {
        let token = req
            .headers()
            .get_one("X-AetherCore-Token")
            .or_else(|| req.headers().get_one("X-HyperCore-Token"))
            .unwrap_or("")
            .to_string();

        let state = req.rocket().state::<AppState>().unwrap();
        let inner = state.read();

        if inner.unsafe_no_auth {
            return Outcome::Success(RequireOperator("admin".into()));
        }

        let role = inner.resolve_role(&token).to_string();
        if crate::state::Inner::role_at_least(&role, "operator") {
            Outcome::Success(RequireOperator(role))
        } else {
            Outcome::Error((Status::Unauthorized, ()))
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequireAdmin {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ()> {
        let token = req
            .headers()
            .get_one("X-AetherCore-Token")
            .or_else(|| req.headers().get_one("X-HyperCore-Token"))
            .unwrap_or("")
            .to_string();

        let state = req.rocket().state::<AppState>().unwrap();
        let inner = state.read();

        if inner.unsafe_no_auth {
            return Outcome::Success(RequireAdmin);
        }

        let role = inner.resolve_role(&token).to_string();
        if crate::state::Inner::role_at_least(&role, "admin") {
            Outcome::Success(RequireAdmin)
        } else {
            Outcome::Error((Status::Unauthorized, ()))
        }
    }
}
