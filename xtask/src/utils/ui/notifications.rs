use notify_rust::Notification;

pub fn send(title: &str, message: &str) {
    let _ = Notification::new()
        .summary(title)
        .body(message)
        .icon("aetherx-os") // We can add an icon path later
        .appname("AetherX XTask")
        .show();
}

pub fn pipeline_success(name: &str) {
    send("🚀 Pipeline Success", &format!("Workflow '{}' completed successfully.", name));
}

pub fn pipeline_failed(name: &str, error: &str) {
    send("❌ Pipeline Failed", &format!("Workflow '{}' failed: {}", name, error));
}
