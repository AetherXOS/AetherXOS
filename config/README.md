# config Directory

This directory contains policy/default/task manifests used by the build and automation layers.

## Contents

- Default policy files
- Task/action manifests
- Command profile definitions
- Plugin config fragments

## Usage rules

- Prefer additive changes over rewriting existing policy files.
- Keep naming explicit and environment-safe.
- Validate config changes through `xtask`/CI before merging.
