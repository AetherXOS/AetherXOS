use serde::Serialize;

#[derive(Serialize)]
pub struct SignReport {
    pub schema_version: u32,
    pub generated_utc: String,
    pub ok: bool,
    pub tool: String,
    pub targets: usize,
    pub rows: Vec<SignRow>,
    pub failures: Vec<String>,
    pub error_codes: Vec<String>,
}

#[derive(Serialize)]
pub struct SignRow {
    pub file: String,
    pub signed: bool,
    pub verified: bool,
    pub tool: String,
    pub dry_run: bool,
    pub detail: String,
}

#[derive(Serialize)]
pub struct SbatReport {
    pub schema_version: u32,
    pub generated_utc: String,
    pub ok: bool,
    pub status: String,
    pub strict: bool,
    pub rows: Vec<SbatRow>,
    pub failures: Vec<String>,
    pub error_codes: Vec<String>,
}

#[derive(Serialize)]
pub struct SbatRow {
    pub file: String,
    pub exists: bool,
    pub has_sbat: bool,
}

#[derive(Serialize)]
pub struct PcrReport {
    pub generated_utc: String,
    pub ok: bool,
    pub event_log_path: String,
    pub event_log_exists: bool,
    pub event_log_size_bytes: u64,
    pub event_log_sha256: String,
}

#[derive(Serialize)]
pub struct OvmfSummary {
    pub generated_utc: String,
    pub ok: bool,
    pub dry_run: bool,
    pub rows: Vec<OvmfCaseResult>,
    pub failures: Vec<String>,
}

#[derive(Serialize)]
pub struct OvmfCaseResult {
    pub name: String,
    pub secure_boot: bool,
    pub ok: bool,
    pub rc: i32,
    pub timeout: bool,
    pub duration_sec: f64,
    pub log_path: String,
}
