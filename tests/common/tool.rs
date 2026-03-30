use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::symlink;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    None,
    CargoSubcommand(&'static str),
    Binary(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandPlan {
    pub label: &'static str,
    pub workdir: &'static str,
    pub program: &'static str,
    pub args: &'static [&'static str],
    pub gate: Option<&'static str>,
    pub availability: Availability,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TierPlan {
    pub label: &'static str,
    pub commands: Vec<CommandPlan>,
}

impl TierPlan {
    pub fn named(&self, label: &str) -> &CommandPlan {
        self.commands
            .iter()
            .find(|plan| plan.label == label)
            .unwrap_or_else(|| panic!("missing command plan: {label}"))
    }
}

impl CommandPlan {
    pub fn invocation(&self, root: &Path, host: &str) -> Vec<String> {
        std::iter::once(self.program.to_owned())
            .chain(self.args.iter().map(|arg| resolve_arg(root, host, arg)))
            .collect()
    }

    pub fn probe_invocation(&self) -> Option<Vec<String>> {
        match self.availability {
            Availability::None => None,
            Availability::CargoSubcommand(subcommand) => Some(vec![
                self.program.to_owned(),
                subcommand.to_owned(),
                "--help".to_owned(),
            ]),
            Availability::Binary(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct ScriptRun {
    pub stdout: String,
    pub stderr: String,
    pub invocations: Vec<Vec<String>>,
}

pub const FAKE_HOST_TRIPLE: &str = "x86_64-unknown-linux-gnu";

pub fn expected_invocations(
    tier: &TierPlan,
    root: &Path,
    host: &str,
    enabled_gates: &[&str],
    available_cargo_subcommands: &[&str],
    available_binaries: &[&str],
) -> Vec<Vec<String>> {
    let enabled_gates: HashSet<_> = enabled_gates.iter().copied().collect();
    let available_cargo_subcommands: HashSet<_> =
        available_cargo_subcommands.iter().copied().collect();
    let available_binaries: HashSet<_> = available_binaries.iter().copied().collect();

    tier.commands
        .iter()
        .flat_map(|command| {
            let mut invocations = Vec::new();

            match command.availability {
                Availability::None => invocations.push(command.invocation(root, host)),
                Availability::CargoSubcommand(subcommand) => {
                    if let Some(probe) = command.probe_invocation() {
                        invocations.push(probe);
                    }
                    if available_cargo_subcommands.contains(subcommand)
                        && gate_enabled(command, &enabled_gates)
                    {
                        invocations.push(command.invocation(root, host));
                    }
                }
                Availability::Binary(binary) => {
                    if available_binaries.contains(binary) && gate_enabled(command, &enabled_gates)
                    {
                        invocations.push(command.invocation(root, host));
                    }
                }
            }

            invocations
        })
        .collect()
}

pub fn run_script(
    rel: &str,
    envs: &[(&str, &str)],
    cargo_subcommands: &[&str],
    binaries: &[&str],
) -> ScriptRun {
    let sandbox = TempDir::new().unwrap_or_else(|err| panic!("failed to create temp dir: {err}"));
    let bin_dir = sandbox.path().join("bin");
    let sys_dir = sandbox.path().join("sysbin");
    let log_path = sandbox.path().join("invocations.log");

    fs::create_dir_all(&bin_dir).unwrap_or_else(|err| panic!("failed to create bin dir: {err}"));
    fs::create_dir_all(&sys_dir).unwrap_or_else(|err| panic!("failed to create sys dir: {err}"));
    write_executable(&bin_dir.join("cargo"), cargo_stub());
    write_executable(&bin_dir.join("rustc"), rustc_stub());
    link_system_tool(&sys_dir, "dirname");
    link_system_tool(&sys_dir, "sed");

    for binary in binaries {
        write_executable(&bin_dir.join(binary), passthrough_stub(binary));
    }

    let path = format!("{}:{}", bin_dir.display(), sys_dir.display());
    let output = Command::new("bash")
        .arg(crate::common::ctx::path(rel))
        .current_dir(crate::common::ctx::root())
        .env_clear()
        .env("PATH", path)
        .env("TEST_LOG", &log_path)
        .env("FAKE_HOST_TRIPLE", FAKE_HOST_TRIPLE)
        .env("FAKE_CARGO_SUBCOMMANDS", cargo_subcommands.join(" "))
        .envs(envs.iter().copied())
        .output()
        .unwrap_or_else(|err| panic!("failed to run {rel}: {err}"));

    let stdout = String::from_utf8(output.stdout)
        .unwrap_or_else(|err| panic!("stdout was not valid utf-8 for {rel}: {err}"));
    let stderr = String::from_utf8(output.stderr)
        .unwrap_or_else(|err| panic!("stderr was not valid utf-8 for {rel}: {err}"));

    assert!(
        output.status.success(),
        "script failed: {rel}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    ScriptRun {
        stdout,
        stderr,
        invocations: read_invocations(&log_path),
    }
}

fn gate_enabled(command: &CommandPlan, enabled_gates: &HashSet<&str>) -> bool {
    command
        .gate
        .map(|gate| enabled_gates.contains(gate))
        .unwrap_or(true)
}

fn resolve_arg(root: &Path, host: &str, arg: &str) -> String {
    if arg == "<host>" {
        return host.to_owned();
    }

    if is_repo_path(arg) {
        return root.join(arg).display().to_string();
    }

    arg.to_owned()
}

fn is_repo_path(arg: &str) -> bool {
    arg.contains('/') && !arg.starts_with('<')
}

fn write_executable(path: &Path, body: String) {
    fs::write(path, body).unwrap_or_else(|err| panic!("failed to write {}: {err}", path.display()));
    let mut perms = fs::metadata(path)
        .unwrap_or_else(|err| panic!("failed to stat {}: {err}", path.display()))
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)
        .unwrap_or_else(|err| panic!("failed to chmod {}: {err}", path.display()));
}

fn read_invocations(path: &Path) -> Vec<Vec<String>> {
    let Ok(body) = fs::read_to_string(path) else {
        return Vec::new();
    };

    body.lines()
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect()
}

fn link_system_tool(sys_dir: &Path, name: &str) {
    let source = Path::new("/usr/bin").join(name);
    let target = sys_dir.join(name);
    symlink(&source, &target).unwrap_or_else(|err| {
        panic!(
            "failed to link {} -> {}: {err}",
            target.display(),
            source.display()
        )
    });
}

fn rustc_stub() -> String {
    r#"#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "-vV" ]]; then
    printf 'binary: rustc\n'
    printf 'host: %s\n' "${FAKE_HOST_TRIPLE:?}"
    exit 0
fi

exit 0
"#
    .to_string()
}

fn cargo_stub() -> String {
    r#"#!/usr/bin/env bash
set -euo pipefail

log="${TEST_LOG:?}"
printf 'cargo' >>"$log"
for arg in "$@"; do
    printf '\t%s' "$arg" >>"$log"
done
printf '\n' >>"$log"

case "${1:-}" in
    geiger|rudra|audit|kani|flamegraph|fuzz)
        if [[ "${2:-}" == "--help" ]]; then
            available=" ${FAKE_CARGO_SUBCOMMANDS:-} "
            if [[ "$available" == *" ${1} "* ]]; then
                exit 0
            fi
            exit 1
        fi
        ;;
esac

exit 0
"#
    .to_string()
}

fn passthrough_stub(program: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

log="${{TEST_LOG:?}}"
printf '{program}' >>"$log"
for arg in "$@"; do
    printf '\t%s' "$arg" >>"$log"
done
printf '\n' >>"$log"

exit 0
"#
    )
}
