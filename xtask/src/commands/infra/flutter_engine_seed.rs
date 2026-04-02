/// Flutter engine provisioning and seeding
/// Downloads and provisions Flutter runtime binaries, Dart VM, and graphics libraries
/// into the initramfs for Flutter app execution capability.

use crate::utils::logging;
use anyhow::Result;
use serde_json::json;
use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::process::Command;

const FLUTTER_ENGINE_VERSION: &str = "3.24.0";
const FLUTTER_STABILITY_CHANNEL: &str = "stable";

const FLUTTER_ESSENTIAL_BINARIES: &[&str] = &[
    "flutter",
    "dart",
];

const FLUTTER_ESSENTIAL_LIBS: &[&str] = &[
    "libflutter.so",
    "libflutter_tonic.so",
    "libskia.so",
    "libfontconfig.so",
    "libfreetype.so.6",
    "libhb.so.1",
    "libpng.so.16",
    "libjpeg.so.62",
    "libzstd.so.1",
    "libz.so.1",
];

/// Download and prepare Flutter engine seed
pub fn prepare_flutter_seed(initramfs_root: &Path) -> Result<()> {
    logging::info(
        "flutter-seed",
        "Preparing Flutter engine seed",
        &[("version", FLUTTER_ENGINE_VERSION)],
    );

    let bin_dir = initramfs_root.join("usr/bin");
    let lib_dir = initramfs_root.join("usr/lib");
    let flutter_dir = initramfs_root.join("opt/flutter");
    let sdk_dir = flutter_dir.join("sdk");

    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&sdk_dir)?;

    // Create Flutter configuration directories
    fs::create_dir_all(initramfs_root.join("var/cache/flutter"))?;
    fs::create_dir_all(initramfs_root.join("var/log/flutter"))?;

    // Try to provision Flutter from host system first
    #[cfg(unix)]
    {
        if provision_flutter_from_host(&bin_dir, &lib_dir, &flutter_dir).is_err() {
            logging::warn("flutter-seed", "Flutter not available via host system", &[]);
            // Try to download Flutter binaries
            if download_flutter_binaries(&bin_dir, &lib_dir, &flutter_dir).is_err() {
                logging::warn(
                    "flutter-seed",
                    "Flutter binary download failed, app execution support limited",
                    &[],
                );
            }
        }
    }

    #[cfg(not(unix))]
    {
        logging::info(
            "flutter-seed",
            "Flutter provisioning skipped on non-Unix build host",
            &[],
        );
    }

    // Create Flutter wrapper script for app execution
    create_flutter_wrapper_script(&bin_dir)?;

    write_flutter_closure_audit(initramfs_root)?;

    logging::ready(
        "flutter-seed",
        "Flutter engine seed prepared",
        &initramfs_root.to_string_lossy(),
    );
    Ok(())
}

fn write_flutter_closure_audit(initramfs_root: &Path) -> Result<()> {
    let lib_hypercore = initramfs_root.join("usr/lib/hypercore");
    fs::create_dir_all(&lib_hypercore)?;

    let binaries = FLUTTER_ESSENTIAL_BINARIES
        .iter()
        .map(|bin| {
            let seeded = initramfs_root.join("usr/bin").join(bin).exists()
                || initramfs_root.join("opt/flutter/bin").join(bin).exists();
            json!({ "name": bin, "seeded": seeded })
        })
        .collect::<Vec<_>>();

    let libraries = FLUTTER_ESSENTIAL_LIBS
        .iter()
        .map(|lib| {
            let seeded = initramfs_root.join("usr/lib").join(lib).exists()
                || initramfs_root.join("opt/flutter/lib").join(lib).exists();
            json!({ "name": lib, "seeded": seeded })
        })
        .collect::<Vec<_>>();

    let required_count = FLUTTER_ESSENTIAL_BINARIES.len() + FLUTTER_ESSENTIAL_LIBS.len();
    let seeded_count = binaries
        .iter()
        .filter(|item| item.get("seeded").and_then(|v| v.as_bool()).unwrap_or(false))
        .count()
        + libraries
            .iter()
            .filter(|item| item.get("seeded").and_then(|v| v.as_bool()).unwrap_or(false))
            .count();

    let manifest = json!({
        "schema": "hypercore.flutter.runtime.closure.v1",
        "flutter_engine_version": FLUTTER_ENGINE_VERSION,
        "stability_channel": FLUTTER_STABILITY_CHANNEL,
        "build_host": std::env::consts::OS,
        "required_count": required_count,
        "seeded_count": seeded_count,
        "closure_ratio": if required_count > 0 {
            (seeded_count as f64) / (required_count as f64)
        } else {
            1.0
        },
        "binaries": binaries,
        "libraries": libraries
    });

    fs::write(
        lib_hypercore.join("flutter-runtime-closure-audit.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;

    Ok(())
}

/// Try to provision Flutter from host system (Unix only)
#[cfg(unix)]
fn provision_flutter_from_host(bin_dir: &Path, lib_dir: &Path, flutter_dir: &Path) -> Result<()> {
    // Check if flutter command exists on host
    if let Ok(output) = Command::new("which")
        .arg("flutter")
        .output()
    {
        if output.status.success() {
            if let Ok(flutter_path_str) = String::from_utf8(output.stdout) {
                let flutter_path = flutter_path_str.trim();
                if Path::new(flutter_path).exists() {
                    logging::info("infra::flutter_engine_seed", "Found Flutter on host", &[("path", flutter_path)]);
                    return copy_flutter_with_dependencies(flutter_path, bin_dir, lib_dir, flutter_dir);
                }
            }
        }
    }

    // Check if Dart SDK exists
    if let Ok(output) = Command::new("which")
        .arg("dart")
        .output()
    {
        if output.status.success() {
            if let Ok(dart_path_str) = String::from_utf8(output.stdout) {
                let dart_path = dart_path_str.trim();
                if Path::new(dart_path).exists() {
                    logging::info("infra::flutter_engine_seed", "Found Dart on host", &[("path", dart_path)]);
                    copy_dart_runtime(dart_path, bin_dir, lib_dir)?;
                }
            }
        }
    }

    Err(anyhow::anyhow!("Flutter/Dart not found on host"))
}

/// Copy Flutter framework from host with dependencies (Unix only)
#[cfg(unix)]
fn copy_flutter_with_dependencies(
    flutter_exe: &str,
    bin_dir: &Path,
    lib_dir: &Path,
    flutter_dir: &Path,
) -> Result<()> {
    use std::io::{Read, Write};

    let flutter_path = Path::new(flutter_exe);
    if !flutter_path.exists() {
        return Err(anyhow::anyhow!("flutter executable not found"));
    }

    // Copy flutter executable
    let dest = bin_dir.join("flutter");
    let mut src = fs::File::open(flutter_path)?;
    let mut dst = fs::File::create(&dest)?;
    let mut buf = [0; 8192];
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 { break; }
        dst.write_all(&buf[..n])?;
    }
    dst.sync_all()?;
    fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))?;
    logging::info("infra::flutter_engine_seed", "Copied flutter executable", &[]);

    // Find and copy Flutter SDK directory
    if let Ok(output) = Command::new("flutter")
        .arg("--version")
        .output()
    {
        if output.status.success() {
            logging::info("infra::flutter_engine_seed", "Flutter version verified", &[]);
        }
    }

    // Copy essential Flutter libraries (simplified)
    for lib_name in FLUTTER_ESSENTIAL_LIBS {
        if let Ok(lib_output) = Command::new("find")
            .arg("/usr/lib")
            .arg("-name")
            .arg(lib_name)
            .output()
        {
            if lib_output.status.success() {
                let lib_paths = String::from_utf8_lossy(&lib_output.stdout);
                for lib_path in lib_paths.lines().take(1) {
                    if let Ok(_) = fs::copy(lib_path, lib_dir.join(lib_name)) {
                        logging::info("infra::flutter_engine_seed", "Copied library", &[("library", lib_name)]);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Copy Dart runtime (Unix only)
#[cfg(unix)]
fn copy_dart_runtime(dart_exe: &str, bin_dir: &Path, lib_dir: &Path) -> Result<()> {
    use std::io::{Read, Write};

    let dart_path = Path::new(dart_exe);
    if !dart_path.exists() {
        return Err(anyhow::anyhow!("dart executable not found"));
    }

    let dest = bin_dir.join("dart");
    let mut src = fs::File::open(dart_path)?;
    let mut dst = fs::File::create(&dest)?;
    let mut buf = [0; 8192];
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 { break; }
        dst.write_all(&buf[..n])?;
    }
    dst.sync_all()?;
    fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))?;
    logging::info("infra::flutter_engine_seed", "Copied dart runtime", &[]);

    Ok(())
}

/// Try to download Flutter engine binaries
#[cfg(unix)]
fn download_flutter_binaries(bin_dir: &Path, lib_dir: &Path, _flutter_dir: &Path) -> Result<()> {
    logging::info("infra::flutter_engine_seed", "Attempting to download Flutter binaries", &[("version", FLUTTER_ENGINE_VERSION)]);

    // Flutter release URL pattern
    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        return Err(anyhow::anyhow!("Unsupported architecture"));
    };

    let flutter_url = format!(
        "https://storage.googleapis.com/flutter_infra_release/releases/{}/flutter-linux-{}-{}.tar.xz",
        FLUTTER_STABILITY_CHANNEL, arch, FLUTTER_ENGINE_VERSION
    );

    logging::info("infra::flutter_engine_seed", "Downloading from", &[("url", &flutter_url)]);

    let temp_dir = std::env::temp_dir().join("hypercore-flutter-seed");
    fs::create_dir_all(&temp_dir)?;

    let download_path = temp_dir.join("flutter.tar.xz");

    // Try curl first
    let download_ok = Command::new("curl")
        .arg("-fsSL")
        .arg(&flutter_url)
        .arg("-o")
        .arg(&download_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !download_ok {
        logging::warn("infra::flutter_engine_seed", "Download failed, trying wget", &[]);
        let wget_ok = Command::new("wget")
            .arg("-qO")
            .arg(&download_path)
            .arg(&flutter_url)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !wget_ok {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(anyhow::anyhow!("Flutter download failed"));
        }
    }

    // Extract tar.xz archive
    if !Command::new("tar")
        .arg("-xJf")
        .arg(&download_path)
        .arg("-C")
        .arg(&temp_dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(anyhow::anyhow!("Failed to extract Flutter archive"));
    }

    // Copy extracted binaries
    let flutter_extracted = temp_dir.join("flutter/bin");
    if flutter_extracted.exists() {
        for entry in fs::read_dir(&flutter_extracted)? {
            let entry = entry?;
            if let Ok(ft) = entry.file_type() {
                if ft.is_file() {
                    let name = entry.file_name();
                    let dest = bin_dir.join(&name);
                    if let Ok(_) = fs::copy(entry.path(), &dest) {
                        let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
                        logging::info("infra::flutter_engine_seed", "Installed binary", &[("name", &name.to_string_lossy())]);
                    }
                }
            }
        }
    }

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    logging::info("infra::flutter_engine_seed", "Flutter binaries installed", &[]);
    Ok(())
}

/// Create Flutter wrapper script for app execution
fn create_flutter_wrapper_script(bin_dir: &Path) -> Result<()> {
    let wrapper_script = if cfg!(windows) {
        r#"@echo off
REM Flutter wrapper for HyperCore Linux compat layer
setlocal enabledelayedexpansion
set FLUTTER_ROOT=%~dp0..\..\opt\flutter
set PATH=%FLUTTER_ROOT%\bin;%PATH%
flutter %*
"#
    } else {
        r#"#!/bin/sh
# Flutter wrapper for HyperCore Linux compat layer
FLUTTER_ROOT="$(cd "$(dirname "$0")/../.." && pwd)/opt/flutter"
export FLUTTER_ROOT
export PATH="$FLUTTER_ROOT/bin:$PATH"
export FLUTTER_DISABLE_ANALYTICS=true
export FLUTTER_NO_EMOJI=1

# Ensure single-threaded mode for container compatibility
export FLUTTER_BUILD_JOBS=1

exec flutter "$@"
"#
    };

    let wrapper_path = bin_dir.join(if cfg!(windows) { "flutter.bat" } else { "flutter-wrapper.sh" });
    fs::write(&wrapper_path, wrapper_script)?;

    #[cfg(unix)]
    {
        fs::set_permissions(&wrapper_path, fs::Permissions::from_mode(0o755))?;
    }

    logging::info("infra::flutter_engine_seed", "Created Flutter wrapper script", &[]);
    Ok(())
}

/// Create minimal Flutter application test harness
pub fn create_flutter_test_harness(initramfs_root: &Path) -> Result<()> {
    let test_app_dir = initramfs_root.join("opt/flutter_test_app");
    fs::create_dir_all(&test_app_dir)?;

    // Create minimal pubspec.yaml
    let pubspec = r#"name: hypercore_test_app
description: HyperCore Flutter test application
publish_to: 'none'

environment:
  sdk: '>=3.0.0 <4.0.0'

dependencies:
  flutter:
    sdk: flutter

dev_dependencies:
  flutter_test:
    sdk: flutter

flutter:
  uses-material-design: true
"#;

    fs::write(test_app_dir.join("pubspec.yaml"), pubspec)?;

    // Create minimal main.dart
    let main_dart = r#"import 'package:flutter/material.dart';

void main() {
  runApp(const HyperCoreTestApp());
}

class HyperCoreTestApp extends StatelessWidget {
  const HyperCoreTestApp({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'HyperCore Flutter Test',
      theme: ThemeData(primarySwatch: Colors.blue),
      home: const TestPage(),
    );
  }
}

class TestPage extends StatefulWidget {
  const TestPage({Key? key}) : super(key: key);

  @override
  State<TestPage> createState() => _TestPageState();
}

class _TestPageState extends State<TestPage> {
  int _counter = 0;

  void _incrementCounter() {
    setState(() {
      _counter++;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('HyperCore Flutter Test')),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            const Text('HyperCore Flutter Runtime Test'),
            Text('Button pressed: $_counter times',
              style: Theme.of(context).textTheme.headlineMedium),
          ],
        ),
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: _incrementCounter,
        tooltip: 'Increment',
        child: const Icon(Icons.add),
      ),
    );
  }
}
"#;

    fs::create_dir_all(test_app_dir.join("lib"))?;
    fs::write(test_app_dir.join("lib/main.dart"), main_dart)?;

    logging::info("infra::flutter_engine_seed", "Created Flutter test application", &[]);
    Ok(())
}
