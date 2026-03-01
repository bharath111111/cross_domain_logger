# Cross Domain Logger

Windows-based Rust logger for QNX/Android/Ethernet/CAN capture workflows, including Vector XL CAN support and deployable test bundles.

## Production Structure

- `src/` – application source code
- `scripts/` – operational and packaging scripts (canonical location)
- `reference/` – reference inputs (`can config.png`, `ca_temp_log.txt`)
- `dist/` – generated distributables (build outputs for sharing)
- `run_*.bat` and `build_and_update_shareable.bat` in repo root – compatibility wrappers that forward to `scripts/`

## Build (release, GNU toolchain)

```bat
cargo +stable-x86_64-pc-windows-gnu build --release --features vxl-can
```

## Package for sharing

```bat
build_and_update_shareable.bat
```

Generated artifacts:

- `dist/cross_domain_logger_windows_can_test_bundle.zip`
- `dist/cross_domain_logger_shareable_package.zip`

Shareable package content:

- `cross_domain_logger_windows_can_test_bundle.zip` (deployable bundle)
- `BUILD_CONFIG_SUMMARY.txt` (channel/network/build configuration summary)

## Notes

- `vxlapi64.dll` must remain at repository root for local packaging.
- If you need to run can test scripts directly from repo root, wrappers are already provided.
