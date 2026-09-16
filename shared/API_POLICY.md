# Shared API Policy

This document defines compatibility expectations for `aethercore-common`.

## Scope

- Public modules exported from `shared/src/lib.rs`
- Public macros exported from `shared/src/macros.rs`
- Public structs/enums/functions in exported modules

## Stability Levels

- Stable: Existing public items and signatures in released tags
- Experimental: New items explicitly marked as experimental in docs/comments
- Internal: Non-exported modules and private items; no compatibility guarantee

## Compatibility Rules

- Do not remove stable public items in patch/minor maintenance updates.
- Do not change semantic behavior of stable items without migration notes.
- Additive changes are preferred: new functions/fields/modules over signature breakage.
- Macro behavior changes must preserve existing invocation forms unless versioned.

## Error and Parsing Contracts

- Parsing helpers should return context-rich errors where available.
- Display strings are human-oriented and may evolve; structured fields are the contract.

## Feature Flags

- Feature combinations (`no-default`, `clap`, `serde`, `clap+serde`) must compile.
- New features must be optional and isolated.

## Review Checklist

- Public item changed?
- Backward compatible?
- Migration notes added?
- Feature matrix still green?
- Tests updated?
