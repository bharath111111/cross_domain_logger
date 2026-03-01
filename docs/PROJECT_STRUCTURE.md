# Project Structure Policy

This repository follows a production-oriented layout with clear separation between source, operations scripts, references, and generated artifacts.

## Rules

1. Keep source code under `src/`.
2. Keep operational batch scripts under `scripts/`.
3. Keep immutable reference files under `reference/`.
4. Keep generated zips/bundles only under `dist/`.
5. Do not commit generated runtime logs (`*.asc`, `*_console.log`) or cargo build output (`target/`).

## Script Conventions

- Canonical scripts live in `scripts/`.
- Root-level scripts are thin wrappers for backward compatibility.
- Packaging script must remain idempotent (safe to run repeatedly).

## Release Flow

1. Build release binary.
2. Stage bundle in a temporary folder under `dist/`.
3. Zip deployable bundle to `dist/cross_domain_logger_windows_can_test_bundle.zip`.
4. Stage shareable package content in a temporary folder under `dist/`.
5. Create `dist/cross_domain_logger_shareable_package.zip`.
6. Remove all temporary staging folders.
