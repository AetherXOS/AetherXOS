pub mod telemetry;
pub mod providers;

use std::time::Duration;
use anyhow::{Result, Context};
use crate::utils::logging;
use providers::{get_registered_providers, NotificationProvider};
use telemetry::NotificationTelemetry;

pub struct WebhookNotifier {
    client: reqwest::blocking::Client,
    url: String,
    provider: Box<dyn NotificationProvider>,
}

impl WebhookNotifier {
    /// Dynamically resolves the provider by letting the registered strategies self-identify!
    /// Completely decouples URL matching logic from the orchestrator engine.
    pub fn new(url: String) -> Self {
        let mut selected_provider: Option<Box<dyn NotificationProvider>> = None;
        for prov in get_registered_providers() {
            if prov.can_handle(&url) {
                selected_provider = Some(prov);
                break;
            }
        }

        // Guaranteed fallback if no other provider identifies
        let provider = selected_provider.unwrap_or_else(|| Box::new(providers::GenericProvider));

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());

        Self { client, url, provider }
    }

    /// Gathers dynamic host telemetry and dispatches the payload packet to the resolved endpoint provider
    pub fn send_notification(&self, title: &str, message: &str, is_success: bool) -> Result<()> {
        let telemetry = NotificationTelemetry::gather();
        let payload = self.provider.format_payload(title, message, is_success, &telemetry);

        let body = serde_json::to_string(&payload)
            .context("Failed to serialize webhook payload")?;

        let response = self.client.post(&self.url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .context("HTTP request to webhook failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let err_body = response.text().unwrap_or_else(|_| "No body".to_string());
            anyhow::bail!("Webhook server returned error status: {} | Body: {}", status, err_body);
        }

        logging::log("AOP", "WEBHOOK", &format!(
            "Dispatched notification via {} provider: {}", 
            self.provider.provider_name(), title
        ));
        Ok(())
    }
}
