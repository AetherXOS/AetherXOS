# Deprecation and Migration Policy

This document explains how APIs in `aethercore-common` are deprecated and migrated.

## Deprecation Process

1. Introduce replacement API.
2. Mark old API with `#[deprecated(note = "...")]` where applicable.
3. Add migration notes with before/after examples.
4. Keep deprecated API available for at least one full refactor cycle.

## Migration Notes Template

- Why old API is deprecated
- Replacement API name
- Mechanical migration steps
- Behavioral differences (if any)

## Example Migration

Before:

```rust
let parsed = TargetArch::from_str("x86_64");
```

After:

```rust
let parsed = TargetArch::parse("x86_64");
```

(Use the richer parse API when contextual errors are needed.)

## Removal Policy

- Removal only after roadmap milestone sign-off and changelog entry.
- Breaking removals must be grouped and documented in one migration section.

## Non-Breaking Preference

Prefer adapters and compatibility shims over immediate removal.
