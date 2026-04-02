#[macro_use]
extern crate rocket;

mod actions;
mod auth;
mod config;
mod cors;
mod dto;
mod models;
mod resp;
mod routes;
mod state;

use clap::Parser;
use rocket::fairing::AdHoc;
use rocket::Config;
use std::net::IpAddr;
use std::str::FromStr;

use cors::CorsFairing;
use state::AppState;

#[derive(Parser, Debug)]
#[command(name = "aethercore-agent", about = "AetherCore OS dashboard agent (Rocket)")]
struct Args {
    /// Port to listen on (overrides config)
    #[arg(short, long)]
    port: Option<u16>,

    /// Path to aethercore.defaults.cjson
    #[arg(short, long)]
    config: Option<String>,

    /// Bind address (default: 127.0.0.1)
    #[arg(short, long)]
    address: Option<String>,

    /// Disable auth token checks (development only)
    #[arg(long)]
    no_safe: bool,
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    let mut cfg = config::load_agent_config(args.config.as_deref());

    if args.no_safe {
        cfg.auth_mode = "unsafe".into();
    }
    if let Some(port) = args.port {
        cfg.port = port;
    }

    let address = args.address.as_deref().unwrap_or("127.0.0.1").to_string();
    tracing::info!(port = cfg.port, address = %address, auth_mode = %cfg.auth_mode, "aethercore-agent starting");

    build_rocket(cfg, &address).launch().await?;

    Ok(())
}

pub(crate) fn build_rocket(cfg: config::AgentConfig, address: &str) -> rocket::Rocket<rocket::Build> {
    let port = cfg.port;
    let app_state = AppState::new(&cfg);

    let rocket_cfg = Config {
        port,
        address: IpAddr::from_str(address).unwrap_or(IpAddr::from([127, 0, 0, 1])),
        ..Config::default()
    };

    rocket::custom(rocket_cfg)
        .manage(app_state)
        .attach(CorsFairing)
        .attach(AdHoc::on_liftoff("Agent background workers", |rocket| {
            Box::pin(async move {
                let bg_state = rocket.state::<AppState>().expect("app state on liftoff").clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
                    loop {
                        interval.tick().await;
                        let triggered = state::tick_scheduler(&bg_state);
                        if !triggered.is_empty() {
                            tracing::debug!(?triggered, "scheduler triggered tasks");
                        }
                        let pruned = state::prune_expired_confirmations(&bg_state);
                        if pruned > 0 {
                            tracing::debug!(pruned = pruned, "pruned expired confirmations");
                        }
                        let pruned_idempotency = state::prune_idempotency_registry(&bg_state, 24);
                        if pruned_idempotency > 0 {
                            tracing::debug!(pruned = pruned_idempotency, "pruned idempotency registry");
                        }
                        actions::dispatch_queue(&bg_state);
                    }
                });
            })
        }))
        .mount(
            "/",
            routes![
                cors::options_preflight,
                routes::health::health,
                routes::health::ready,
                routes::health::status,
                routes::health::metrics,
                routes::health::agent_state,
                routes::health::queue,
                routes::auth_routes::auth_status,
                routes::auth_routes::auth_rotate,
                routes::catalog::catalog,
                routes::catalog::catalog_action,
                routes::ops::jobs::list_jobs,
                routes::ops::jobs::jobs_stats,
                routes::ops::events::global_events,
                routes::ops::events::events_stats,
                routes::ops::events::global_events_stream,
                routes::ops::jobs::get_job,
                routes::ops::jobs::cancel_job,
                routes::ops::jobs::retry_existing_job,
                routes::ops::jobs::prune_jobs,
                routes::ops::jobs::job_stream,
                routes::ops::jobs::job_events,
                routes::run_routes::run,
                routes::run_routes::run_async,
                routes::dispatch::dispatch_run_async,
                routes::dispatch::dispatch_fanout,
                routes::dispatch::dispatch_jobs,
                routes::dispatch::dispatch_job,
                routes::dispatch::dispatch_job_cancel,
                routes::dispatch::dispatch_job_retry,
                routes::dispatch::dispatch_job_stream,
                routes::dispatch::dispatch_job_events,
                routes::scheduler::get_scheduler,
                routes::scheduler::scheduler_templates,
                routes::scheduler::scheduler_apply_template,
                routes::scheduler::scheduler_run_now,
                routes::roadmap::roadmap_status,
                routes::roadmap::roadmap_master,
                routes::roadmap::roadmap_master_update,
                routes::roadmap::roadmap_batch_record,
                routes::config_mgmt::get_config,
                routes::config_mgmt::config_compose,
                routes::config_mgmt::config_drift,
                routes::config_mgmt::config_drift_apply,
                routes::config_mgmt::config_export,
                routes::config_mgmt::config_overrides_template,
                routes::config_mgmt::config_update,
                routes::config_mgmt::config_auto,
                routes::config_mgmt::config_import,
                routes::config_mgmt::config_compose_apply,
                routes::hosts::list_hosts,
                routes::hosts::hosts_register,
                routes::hosts::hosts_update,
                routes::hosts::hosts_remove,
                routes::hosts::hosts_heartbeat,
                routes::hosts::status_hosts,
                routes::policy_routes::get_policy,
                routes::policy_routes::policy_template,
                routes::policy_routes::policy_apply,
                routes::policy_routes::policy_validate,
                routes::policy_routes::policy_reset,
                routes::policy_routes::policy_simulate,
                routes::policy_routes::policy_traces,
                routes::confirm::confirm_list,
                routes::confirm::confirm_request,
                routes::confirm::confirm_revoke,
                routes::plugins::crash_summary,
                routes::plugins::plugins_list,
                routes::plugins::plugin_detail,
                routes::plugins::plugins_health,
                routes::launcher::launcher_agent_status,
                routes::launcher::launcher_audit,
                routes::launcher::launcher_start_agent,
                routes::launcher::launcher_stop_agent,
                routes::launcher::launcher_restart_agent,
                routes::compliance::compliance_report,
                routes::compliance::security_regression_template,
            ],
        )
}

#[cfg(test)]
mod tests;

