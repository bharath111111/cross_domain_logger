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
- Default CAN/bus backend: Vector XL API (`vxlapi64.dll`) via `--can-backend vxl`
- Optional ControlDesk backend: COM snapshot/probe tools via `--can-backend controldesk`
- Build profile: release, GNU Windows toolchain

## Channel mapping in this build

| User Channel | App Channel | VN        | Network |
|---|---:|---|---|
| 1  | 0  | vn 1670 1 | FD_CANW  |
| 2  | 1  | vn 1670 1 | FD_CAN5  |
| 3  | 2  | vn 1670 2 | FD_CAN13 |
| 4  | 3  | vn 1670 2 | FD_CAN14 |
| 5  | 4  | vn 1670 2 | FD_CAN15 |
| 6  | 5  | vn 1670 1 | FD_CAN17 |
| 7  | 6  | vn 1670 1 | FD_CAN18 |
| 8  | 7  | vn 1670 1 | FD_CAN20 |
| 9  | 8  | vn 1670 1 | FD_CAN21 |
| 10 | 9  | vn 1670 1 | HS_CAN1  |
| 11 | 10 | vn 1670 1 | HS_CAN4  |

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
cargo +stable-x86_64-pc-windows-gnu build --release
```

Build + package + refresh shareable outputs:

```bat
build_and_update_shareable.bat
```

Check CAN channel mapping from VXL/CANoe app mapping:

```bat
cross_domain_logger_windows.exe --test-can --can-backend vxl --can-map --can-app CANoe --can-max-channels 64
```

What this gives you:

- app channel mapping with hardware/channel association
- network name mapping for expected channels
- direct visibility before starting long capture runs

Start continuous VXL CAN capture into `CAN_LOGS/`:

```bat
cross_domain_logger_windows.exe --test-can --can-backend vxl --can-listen-all --can-max-channels 64 --can-app CANoe --can-iface-version 4 --can-output-dir CAN_LOGS --can-log-format asc
```

Capture ControlDesk bus interfaces plus all detected Ethernet traffic in one run (default 60s):

```bat
run_controldesk_with_eth_all.bat
```

Custom duration (milliseconds), example 120000 ms:

```bat
run_controldesk_with_eth_all.bat 120000
```

This generates raw Ethernet packet capture in `CAN_LOGS/ethernet_all.pcapng` and ControlDesk logs under `CAN_LOGS/`.

Important: ControlDesk COM snapshot output is topology/status data, not raw CAN frame recorder data.

Capture only ETH_STLB Ethernet network traffic (auto-detect interface name containing `STLB`, default 60s):

```bat
run_eth_stlb_capture.bat
```

Custom duration (milliseconds), example 180000 ms:

```bat
run_eth_stlb_capture.bat 180000
```

Output file: `CAN_LOGS/ETH_STLB.pcapng`.

Probe available ControlDesk/dSPACE COM APIs on the test PC and write a single report file:

```bat
run_testpc_api_probe.bat
```

Optional custom output path:

```bat
run_testpc_api_probe.bat CAN_LOGS\testpc_api_probe_custom.txt
```

Default output file: `CAN_LOGS/testpc_api_probe.txt`.

Create a developer triage package zip from captured files in `CAN_LOGS` (`.asc/.blf/.mdf/.mf4/.pcapng/.pcap/.log/.txt`):

```bat
run_make_triage_pack.bat
```

Optional note text for `metadata.txt`:

```bat
run_make_triage_pack.bat "STLB issue repro - build X.Y - 2026-03-06"
```

Outputs are created in `dist/` as timestamped folder + zip.

Optional (legacy) VXL flow:

```bat
cross_domain_logger_windows.exe --test-can --can-backend vxl --can-map --can-app CANoe --can-max-channels 64
```

## Final deliverables

- `dist/cross_domain_logger_windows_can_test_bundle.zip`
- `dist/cross_domain_logger_shareable_package.zip`

`dist/cross_domain_logger_shareable_package.zip` is the single file to share. It contains:

- `cross_domain_logger_windows_can_test_bundle.zip`
- `BUILD_CONFIG_SUMMARY.txt`

## Notes

- Keep ControlDesk running with an active experiment before using ControlDesk backend commands.
- Install `pywin32` (`pip install pywin32`) if `win32com.client` is unavailable.
- Keep `vxlapi64.dll` only if you still use the legacy VXL backend.
- Runtime CAN output goes to `CAN_LOGS/` to avoid clutter in the main folder.
