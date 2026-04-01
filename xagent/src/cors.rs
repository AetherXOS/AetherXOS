use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{Header, Status};
use rocket::{Request, Response};

/// CORS fairing that mirrors the PowerShell agent's allowed-origins logic.
/// The allowed origins list is read from Rocket managed state at request time.
pub struct CorsFairing;

#[rocket::async_trait]
impl Fairing for CorsFairing {
    fn info(&self) -> Info {
        Info {
            name: "CORS",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, resp: &mut Response<'r>) {
        use crate::state::AppState;

        let origin = req
            .headers()
            .get_one("Origin")
            .unwrap_or("")
            .to_string();

        let allowed = req
            .rocket()
            .state::<AppState>()
            .map(|s| {
                let inner = s.read();
                is_origin_allowed(&origin, &inner.allowed_origins)
            })
            .unwrap_or(true);

        if !allowed {
            return;
        }

        let reflect = if origin.is_empty() {
            "http://127.0.0.1".to_string()
        } else {
            origin.clone()
        };

        resp.set_header(Header::new("Access-Control-Allow-Origin", reflect));
        resp.set_header(Header::new("Vary", "Origin"));
        resp.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "GET,POST,OPTIONS",
        ));
        resp.set_header(Header::new(
            "Access-Control-Allow-Headers",
            "Content-Type,X-HyperCore-Token",
        ));
    }
}

fn is_origin_allowed(origin: &str, allowed: &[String]) -> bool {
    if origin.is_empty() {
        return true;
    }
    for rule in allowed {
        if rule == "*" {
            return true;
        }
        if rule == "null" && origin == "null" {
            return true;
        }
        if origin.to_lowercase().starts_with(&rule.to_lowercase()) {
            return true;
        }
    }
    false
}

/// OPTIONS preflight route (catch-all).
#[rocket::options("/<_path..>")]
pub fn options_preflight(_path: std::path::PathBuf) -> Status {
    Status::NoContent
}
