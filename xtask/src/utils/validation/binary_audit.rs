use crate::utils::logging;
use anyhow::Result;
use std::process::Command;

pub struct BinaryAudit {
    pub name: &'static str,
    pub min_version: Option<&'static str>,
    pub version_arg: &'static str,
}

impl BinaryAudit {
    pub fn check(&self) -> Result<bool> {
        let output = Command::new(self.name).arg(self.version_arg).output();

        match output {
            Ok(o) if o.status.success() => {
                let version = String::from_utf8_lossy(&o.stdout);
                logging::info(
                    "AUDIT",
                    "Dependency verified",
                    &[
                        ("binary", self.name),
                        (
                            "version",
                            version.lines().next().unwrap_or("unknown").trim(),
                        ),
                    ],
                );
                Ok(true)
            }
            _ => {
                logging::warn(
                    "AUDIT",
                    "Dependency missing or broken",
                    &[("binary", self.name)],
                );
                Ok(false)
            }
        }
    }
}

pub fn run_comprehensive_audit() -> Result<()> {
    let audits = vec![
        BinaryAudit {
            name: "rustc",
            min_version: Some("1.75.0"),
            version_arg: "--version",
        },
        BinaryAudit {
            name: "cargo",
            min_version: Some("1.75.0"),
            version_arg: "--version",
        },
        BinaryAudit {
            name: "qemu-system-x86_64",
            min_version: None,
            version_arg: "--version",
        },
        BinaryAudit {
            name: "xorriso",
            min_version: None,
            version_arg: "--version",
        },
    ];

    for audit in audits {
        audit.check()?;
    }
    Ok(())
}
