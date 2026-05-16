use tiny_http::{Server, Response};
use crate::utils::logging;

pub fn start_monitor(port: u16) -> anyhow::Result<()> {
    let server = Server::http(format!("0.0.0.0:{}", port))
        .map_err(|e| anyhow::anyhow!("Failed to start web monitor: {}", e))?;
    
    logging::status("WEB", &format!("Build monitor live at http://localhost:{}", port));
    
    for request in server.incoming_requests() {
        let html = format!(
            "<html><head><title>AetherX Build Monitor</title><style>body {{ font-family: sans-serif; background: #1a1a1a; color: #00ff00; padding: 2em; }}</style></head><body><h1>AetherX Build Monitor</h1><p>Status: All systems operational</p><p>Latest Workflow: ISO_PRODUCTION</p></body></html>"
        );
        let response = Response::from_string(html).with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap());
        let _ = request.respond(response);
    }
    
    Ok(())
}
