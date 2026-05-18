use notify_rust::Notification;

pub fn send(title: &str, message: &str) {
    let _ = Notification::new()
        .summary(title)
        .body(message)
        .icon("aetherx-os") // We can add an icon path later
        .appname("AetherX XTask")
        .show();
}

fn send_webhook(title: &str, message: &str, is_success: bool) {
    if !crate::utils::core::config::get_settings().webhook_enabled {
        return;
    }
    if let Ok(webhook_url) = std::env::var("AETHERX_WEBHOOK_URL") {
        let notifier = super::webhook::WebhookNotifier::new(webhook_url);
        if let Err(e) = notifier.send_notification(title, message, is_success) {
            crate::utils::logging::error(
                "WEBHOOK",
                "Failed to dispatch webhook notification",
                &[("error", &e.to_string())],
            );
        }
    }
}

pub fn pipeline_success(name: &str) {
    let msg = format!("Workflow '{}' completed successfully.", name);
    send("🚀 Pipeline Success", &msg);
    send_webhook("Pipeline Success", &msg, true);
}

pub fn pipeline_failed(name: &str, error: &str) {
    let msg = format!("Workflow '{}' failed: {}", name, error);
    send("❌ Pipeline Failed", &msg);
    send_webhook("Pipeline Failed", &msg, false);
}
