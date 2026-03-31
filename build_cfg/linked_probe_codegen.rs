use std::env;
use std::fs;
use std::path::Path;

pub fn generate() {
    let rel_path = "boot/initramfs/usr/lib/hypercore/probe-linked.elf";
    println!("cargo:rerun-if-changed={rel_path}");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let source_path = Path::new(&manifest_dir).join(rel_path);
    let generated_path = Path::new(&out_dir).join("linked_probe_image.rs");

    let content = if source_path.exists() {
        let canonical = source_path
            .canonicalize()
            .unwrap_or_else(|_| source_path.clone());
        let escaped = canonical.display().to_string().replace('\\', "\\\\");
        format!(
            "pub static LINKED_PROBE_IMAGE: &[u8] = include_bytes!(r\"{}\");\n",
            escaped
        )
    } else {
        "pub static LINKED_PROBE_IMAGE: &[u8] = &[];\n".to_string()
    };

    fs::write(&generated_path, content)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", generated_path.display()));
}
