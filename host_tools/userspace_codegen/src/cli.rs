use std::env;
use std::path::PathBuf;

pub fn parse_args() -> Result<(PathBuf, PathBuf, Option<PathBuf>, bool), String> {
    let mut repo_root = None;
    let mut out = None;
    let mut emit_dir = None;
    let mut run_smoke = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repo-root" => repo_root = args.next().map(PathBuf::from),
            "--out" => out = args.next().map(PathBuf::from),
            "--emit-dir" => emit_dir = args.next().map(PathBuf::from),
            "--run-smoke" => run_smoke = true,
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    match (repo_root, out) {
        (Some(root), Some(out_path)) => Ok((root, out_path, emit_dir, run_smoke)),
        _ => Err(
            "usage: userspace_codegen --repo-root <path> --out <path> [--emit-dir <path>] [--run-smoke]"
                .to_string(),
        ),
    }
}
