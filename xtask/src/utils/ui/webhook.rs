use std::time::Duration;
use anyhow::{Result, Context};
use crate::utils::logging;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookProvider {
    Discord,
    Slack,
    Generic,
}

pub struct WebhookNotifier {
    client: reqwest::blocking::Client,
    url: String,
    provider: WebhookProvider,
}

impl WebhookNotifier {
    pub fn new(url: String) -> Self {
        let provider = if url.contains("slack.com") {
            WebhookProvider::Slack
        } else if url.contains("discord.com") {
            WebhookProvider::Discord
        } else {
            WebhookProvider::Generic
        };

        // Highly robust client builder with custom timeouts
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());

        Self { client, url, provider }
    }

    pub fn send_notification(&self, title: &str, message: &str, is_success: bool) -> Result<()> {
        let payload = match self.provider {
            WebhookProvider::Slack => self.build_slack_payload(title, message, is_success),
            WebhookProvider::Discord => self.build_discord_payload(title, message, is_success),
            WebhookProvider::Generic => self.build_generic_payload(title, message, is_success),
        };

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

        // Silent AOP log for tracing
        logging::log("AOP", "WEBHOOK", &format!("Dispatched notification: {}", title));
        Ok(())
    }

    fn build_slack_payload(&self, title: &str, message: &str, is_success: bool) -> serde_json::Value {
        let emoji = if is_success { "🚀" } else { "❌" };
        serde_json::json!({
            "text": format!("*{} {}*\n{}", emoji, title, message)
        })
    }

    fn build_discord_payload(&self, title: &str, message: &str, is_success: bool) -> serde_json::Value {
        let color = if is_success { 65280 } else { 16711680 };
        serde_json::json!({
            "embeds": [{
                "title": title,
                "description": message,
                "color": color,
                "footer": {
                    "text": "AetherX OS Build Notification Engine"
                },
                "timestamp": chrono::Utc::now().to_rfc3339()
            }]
        })
    }

    fn build_generic_payload(&self, title: &str, message: &str, is_success: bool) -> serde_json::Value {
        serde_json::json!({
            "title": title,
            "message": message,
            "success": is_success,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }
}
