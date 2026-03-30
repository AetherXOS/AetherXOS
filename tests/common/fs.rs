use crate::common::ctx;

pub fn file(rel: &str) {
    let path = ctx::path(rel);
    assert!(path.is_file(), "expected file: {}", path.display());
}

pub fn dir(rel: &str) {
    let path = ctx::path(rel);
    assert!(path.is_dir(), "expected dir: {}", path.display());
}

pub fn text(rel: &str, needle: &str) {
    let body = ctx::read(rel);
    assert!(body.contains(needle), "missing {needle} in {rel}");
}

pub fn any(rel: &str, needles: &[&str]) {
    let body = ctx::read(rel);
    assert!(
        needles.iter().any(|needle| body.contains(needle)),
        "missing any expected marker in {rel}"
    );
}

pub fn ordered(rel: &str, needles: &[&str]) {
    let body = ctx::read(rel);
    let mut offset = 0;

    for needle in needles {
        let remaining = &body[offset..];
        let Some(found) = remaining.find(needle) else {
            panic!("missing ordered marker {needle} in {rel}");
        };
        offset += found + needle.len();
    }
}
