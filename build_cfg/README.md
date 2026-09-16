# build_cfg Directory

`build_cfg` owns typed configuration loading, validation, feature graphing, and config-driven code emission.

## Core responsibilities

- Parse and validate configuration inputs
- Build feature/runtime policy graph
- Emit generated configuration/runtime glue
- Keep config shape changes centralized

## Design boundary

- Keep runtime behavior in kernel modules
- Keep schema/validation/codegen plumbing in `build_cfg`
- Do not duplicate config validation logic across unrelated crates
