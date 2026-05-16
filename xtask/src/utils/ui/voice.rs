

pub fn speak(message: &str) {
    if cfg!(windows) {
        // Use PowerShell's SpeechSynthesizer for native Windows TTS
        let script = format!(
            "Add-Type -AssemblyName System.Speech; $synth = New-Object System.Speech.Synthesis.SpeechSynthesizer; $synth.Speak('{}')",
            message.replace('\'', "''")
        );
        
        let _ = std::process::Command::new("powershell")
            .args(&["-Command", &script])
            .spawn();
    } else {
        // Try espeak on Linux
        let _ = std::process::Command::new("espeak")
            .arg(message)
            .spawn();
    }
}

pub fn pipeline_success_voice(name: &str) {
    speak(&format!("Success. Workflow {} has been completed.", name));
}

pub fn pipeline_failed_voice(name: &str) {
    speak(&format!("Failure. Workflow {} has failed. Please check the logs.", name));
}
