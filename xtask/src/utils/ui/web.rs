use tiny_http::{Server, Response};
use crate::utils::logging;

pub fn start_monitor(port: u16) -> anyhow::Result<()> {
    let server = match Server::http(format!("0.0.0.0:{}", port)) {
        Ok(s) => s,
        Err(_e) => {
            logging::warn("WEB", &format!("Port {} in use, attempting fallback to {}...", port, port + 1), &[]);
            Server::http(format!("0.0.0.0:{}", port + 1))
                .map_err(|_| anyhow::anyhow!("Failed to bind to both {} and {}", port, port + 1))?
        }
    };
    
    logging::status("WEB", &format!("Build monitor live at http://localhost:{}", port));
    
    for request in server.incoming_requests() {
        // 1. Basic Security Audit
        let auth_header = request.headers().iter().find(|h| h.field.as_str() == "Authorization");
        if auth_header.is_none() {
            let _ = request.respond(Response::from_string("Unauthorized").with_status_code(401));
            continue;
        }

        // 2. Routing
        let url = request.url().to_string();
        if url == "/logs" {
            let response = Response::from_string("data: Heartbeat - Engine Live\n\n")
                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/event-stream"[..]).unwrap());
            let _ = request.respond(response);
            continue;
        }

        if url.starts_with("/dispatch/") {
            let workflow = url.trim_start_matches("/dispatch/");
            logging::status("WEB", &format!("Remote dispatch requested: {}", workflow));
            let _ = crate::engine::controller::UniversalController::dispatch_workflow(workflow, &crate::engine::ExecutionContext::from_defaults());
            let _ = request.respond(Response::from_string(format!("Dispatched: {}", workflow)));
            continue;
        }

        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let mem_usage = (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0;
        let current_task = crate::utils::ui::logging::CURRENT_TASK.lock().ok()
            .and_then(|t| t.clone()).unwrap_or_else(|| "IDLE".to_string());

        let html = format!(
            "<html><head><title>AetherX Nexus</title><meta http-equiv='refresh' content='2'><style>body {{ font-family: 'Segoe UI', sans-serif; background: #050505; color: #00ffcc; padding: 3em; }} .card {{ background: #111; padding: 20px; border-radius: 10px; border: 1px solid #222; margin-bottom: 1em; }} .btn {{ display: inline-block; padding: 12px 24px; background: #00ffcc; color: black; text-decoration: none; border-radius: 5px; font-weight: bold; }}</style></head><body><h1>AetherX Nexus Live Monitor</h1><div class='card'><h3>System Telemetry</h3><p>CPU Usage: {:.1}%</p><p>Memory: {:.1}%</p></div><div class='card'><h3>Task Engine</h3><p>Active Task: <b style='color: yellow'>{}</b></p></div><hr/><a href='/dispatch/full_iso' class='btn'>⚡ REMOTE DISPATCH: FULL_ISO</a></body></html>",
            cpu_usage, mem_usage, current_task
        );
        let response = Response::from_string(html).with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap());
        let _ = request.respond(response);
    }
    
    Ok(())
}
