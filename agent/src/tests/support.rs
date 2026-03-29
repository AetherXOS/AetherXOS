use rocket::http::Header;
use rocket::local::blocking::Client;

use crate::{build_rocket, config};

pub const ADMIN_TOKEN: &str = "test-admin-token";

pub fn test_client() -> Client {
    let mut cfg = config::AgentConfig::default();
    cfg.auth_token = ADMIN_TOKEN.into();
    cfg.tokens.admin = ADMIN_TOKEN.into();
    Client::tracked(build_rocket(cfg, "127.0.0.1")).expect("client")
}

pub fn admin_header() -> Header<'static> {
    Header::new("X-HyperCore-Token", ADMIN_TOKEN)
}
