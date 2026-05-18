use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static LAST_SPEAK: Lazy<Mutex<Instant>> =
    Lazy::new(|| Mutex::new(Instant::now() - Duration::from_secs(60)));

pub fn speak(message: &str) {
    if !crate::utils::core::config::get_settings().voice_enabled {
        return;
    }
    if let Ok(mut last) = LAST_SPEAK.lock() {
        if last.elapsed() < Duration::from_secs(30) {
            return; // Smart Silence: Don't speak too often
        }
        *last = Instant::now();
    }
    if cfg!(windows) {
        // Use PowerShell's SpeechSynthesizer for native Windows TTS
        let script = format!(
            "Add-Type -AssemblyName System.Speech; $synth = New-Object System.Speech.Synthesis.SpeechSynthesizer; $synth.Speak('{}')",
            message.replace('\'', "''")
        );

        let _ = std::process::Command::new("powershell")
            .args(["-Command", &script])
            .spawn()
            .map(|c| crate::utils::sys::process::track_child(&c))
            .map_err(|e| {
                crate::utils::logging::debug(
                    "VOICE",
                    "Failed to speak",
                    &[("error", &e.to_string())],
                )
            })
            .ok();
    } else {
        // Try espeak on Linux
        let _ = std::process::Command::new("espeak")
            .arg(message)
            .spawn()
            .map_err(|e| {
                crate::utils::logging::debug(
                    "VOICE",
                    "Failed to speak",
                    &[("error", &e.to_string())],
                )
            })
            .ok();
    }
}

pub fn pipeline_success_voice(name: &str) {
    speak(&format!("Success. Workflow {} has been completed.", name));
}

pub fn pipeline_failed_voice(name: &str) {
    speak(&format!(
        "Failure. Workflow {} has failed. Please check the logs.",
        name
    ));
}
