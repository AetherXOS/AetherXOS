use super::telemetry::NotificationTelemetry;

pub const COLOR_SUCCESS: u32 = 0x2ECC71; // Emerald Green
pub const COLOR_FAILURE: u32 = 0xE74C3C; // Alizarin Red

pub const EMOJI_SUCCESS: &str = "🚀";
pub const EMOJI_FAILURE: &str = "❌";

/// Trait defining pluggable notification delivery channels.
/// Providers self-identify if they can handle a specific endpoint URL!
pub trait NotificationProvider: Send + Sync {
    fn provider_name(&self) -> &'static str;
    
    /// Returns true if this provider is responsible for handling the target webhook URL
    fn can_handle(&self, url: &str) -> bool;
    
    /// Formats the payload according to provider-specific specifications
    fn format_payload(&self, title: &str, message: &str, is_success: bool, telemetry: &NotificationTelemetry) -> serde_json::Value;
}

// ==================== Discord Provider ====================
pub struct DiscordProvider;
impl NotificationProvider for DiscordProvider {
    fn provider_name(&self) -> &'static str { "Discord" }

    fn can_handle(&self, url: &str) -> bool {
        url.contains("discord.com") || url.contains("discordapp.com")
    }

    fn format_payload(&self, title: &str, message: &str, is_success: bool, telemetry: &NotificationTelemetry) -> serde_json::Value {
        let color = if is_success { COLOR_SUCCESS } else { COLOR_FAILURE };
        let emoji = if is_success { EMOJI_SUCCESS } else { EMOJI_FAILURE };
        
        let mut fields = vec![
            serde_json::json!({
                "name": "🖥️  Host System Information",
                "value": format!("**OS:** {}\n**CPU:** {} Cores\n**RAM:** {}", telemetry.os_name, telemetry.cpu_cores, telemetry.total_ram),
                "inline": true
            }),
            serde_json::json!({
                "name": "📦 Build Target Metadata",
                "value": format!("**Kernel Size:** {}", telemetry.kernel_size),
                "inline": true
            })
        ];

        if let Some(ref hash) = telemetry.git_hash {
            let author = telemetry.git_author.as_deref().unwrap_or("Unknown");
            let msg = telemetry.git_message.as_deref().unwrap_or("No message");
            fields.push(serde_json::json!({
                "name": "🌿 Git Commit Context",
                "value": format!("**Commit:** `{}`\n**Author:** {}\n**Message:** `{}`", hash, author, msg),
                "inline": false
            }));
        }

        serde_json::json!({
            "embeds": [{
                "title": format!("{} {}", emoji, title),
                "description": message,
                "color": color,
                "fields": fields,
                "footer": {
                    "text": "AetherX OS Sovereign Build System"
                },
                "timestamp": chrono::Utc::now().to_rfc3339()
            }]
        })
    }
}

// ==================== Slack Provider ====================
pub struct SlackProvider;
impl NotificationProvider for SlackProvider {
    fn provider_name(&self) -> &'static str { "Slack" }

    fn can_handle(&self, url: &str) -> bool {
        url.contains("slack.com") || url.contains("hooks.slack.com")
    }

    fn format_payload(&self, title: &str, message: &str, is_success: bool, telemetry: &NotificationTelemetry) -> serde_json::Value {
        let emoji = if is_success { EMOJI_SUCCESS } else { EMOJI_FAILURE };
        
        let mut text = format!("*{} {}*\n_{}_\n\n*🖥️ Host Info:* OS: {}, CPU: {} Cores, RAM: {}\n*📦 Target Size:* {}", 
            emoji, title, message, telemetry.os_name, telemetry.cpu_cores, telemetry.total_ram, telemetry.kernel_size);

        if let Some(ref hash) = telemetry.git_hash {
            let author = telemetry.git_author.as_deref().unwrap_or("Unknown");
            let msg = telemetry.git_message.as_deref().unwrap_or("No message");
            text.push_str(&format!("\n*🌿 Git Info:* `{}` by *{}* - _{}_", hash, author, msg));
        }

        serde_json::json!({
            "text": text
        })
    }
}

// ==================== Generic JSON Provider ====================
pub struct GenericProvider;
impl NotificationProvider for GenericProvider {
    fn provider_name(&self) -> &'static str { "Generic Webhook" }

    fn can_handle(&self, _url: &str) -> bool {
        true // Serves as the ultimate fallback provider
    }

    fn format_payload(&self, title: &str, message: &str, is_success: bool, telemetry: &NotificationTelemetry) -> serde_json::Value {
        serde_json::json!({
            "title": title,
            "message": message,
            "success": is_success,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "telemetry": {
                "os": telemetry.os_name,
                "cores": telemetry.cpu_cores,
                "ram": telemetry.total_ram,
                "kernel_size": telemetry.kernel_size,
                "git": {
                    "hash": telemetry.git_hash,
                    "author": telemetry.git_author,
                    "message": telemetry.git_message
                }
            }
        })
    }
}

/// Helper function to load all registered providers.
/// Enables simple pluggability by just appending to this list!
pub fn get_registered_providers() -> Vec<Box<dyn NotificationProvider>> {
    vec![
        Box::new(SlackProvider),
        Box::new(DiscordProvider),
        Box::new(GenericProvider), // Keep fallback provider at the end
    ]
}
