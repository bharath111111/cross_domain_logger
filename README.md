# Cross Domain Logger

Cross Domain Logger is a Windows desktop capture tool for test runs where data comes from multiple domains (QNX, Android, Ethernet, and CAN).

## What problem this solves

During validation, logs are often collected from different tools and saved in different places. This project keeps that workflow in one place:

- one panel to start/stop selected sources
- consistent folder structure for each run
- CAN capture with network-based file names
- a single shareable package for handoff

## Current setup

- Language/runtime: Rust (`eframe/egui` UI)
- CAN stack: Vector XL API (`vxlapi64.dll`)
- Build profile: release, GNU Windows toolchain, feature `vxl-can`
- CAN app name: `CANoe`
- Interface version: `4`
- User-visible channels: `1..11`
- Internal app channels: `0..10`

## Channel mapping in this build

| User Channel | App Channel | VN        | Network |
|---|---:|---|---|
| 1  | 0  | vn 1670 1 | FD_CANW  |
| 2  | 1  | vn 1670 1 | FD_CAN5  |
| 3  | 2  | vn 1670 2 | FD_CAN9  |
| 4  | 3  | vn 1670 2 | FD_CAN13 |
| 5  | 4  | vn 1670 2 | FD_CAN14 |
| 6  | 5  | vn 1670 1 | FD_CAN15 |
| 7  | 6  | vn 1670 1 | FD_CAN17 |
| 8  | 7  | vn 1670 1 | FD_CAN18 |
| 9  | 8  | vn 1670 1 | FD_CAN20 |
| 10 | 9  | vn 1670 1 | FD_CAN21 |
| 11 | 10 | vn 1670 1 | HS_CAN1  |

CAN logs are written with network names (example: `FD_CAN5.asc`) under `CAN_LOGS/`.

## Folder layout

- `src/` - application code
- `scripts/` - canonical operational scripts
- `reference/` - fixed reference files
- `dist/` - generated release/shareable artifacts
- root `run_*.bat` and `build_and_update_shareable.bat` - compatibility wrappers

## Build and package

Build manually:

```bat
cargo +stable-x86_64-pc-windows-gnu build --release --features vxl-can
```

Build + package + refresh shareable outputs:

```bat
build_and_update_shareable.bat
```

## Final deliverables

- `dist/cross_domain_logger_windows_can_test_bundle.zip`
- `dist/cross_domain_logger_shareable_package.zip`

`dist/cross_domain_logger_shareable_package.zip` is the single file to share. It contains:

- `cross_domain_logger_windows_can_test_bundle.zip`
- `BUILD_CONFIG_SUMMARY.txt`

## Notes

- Keep `vxlapi64.dll` at repository root for local packaging.
- Runtime CAN output goes to `CAN_LOGS/` to avoid clutter in the main folder.
