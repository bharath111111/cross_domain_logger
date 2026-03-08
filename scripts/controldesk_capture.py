import argparse
import datetime
import os
import sys
import time
from typing import Any, Dict, List, Optional, Set


_OUT_OF_RANGE_TEXT = "Value does not fall within the expected range"
_warned_invalid_indices: Set[int] = set()


def _safe_name(value):
    text = str(value)
    cleaned = []
    for char in text:
        if char.isalnum() or char in ("-", "_", "."):
            cleaned.append(char)
        else:
            cleaned.append("_")
    return "".join(cleaned).strip("_") or "unnamed"


def _classify_bus(name: str) -> str:
    upper = name.upper()
    if "CAN" in upper:
        return "CAN"
    if "LIN" in upper:
        return "LIN"
    if "ETH" in upper or "ETHERNET" in upper:
        return "ETH"
    if "FLEXRAY" in upper or "FRAY" in upper:
        return "FLEXRAY"
    return "OTHER"


def _connect_controldesk():
    try:
        import win32com.client  # type: ignore
    except Exception as exc:
        raise RuntimeError(f"win32com.client not available: {exc}") from exc

    app = win32com.client.Dispatch("ControlDeskNG.Application")
    experiment = app.ActiveExperiment
    return app, experiment


def _is_out_of_range_exception(exc: Exception) -> bool:
    return _OUT_OF_RANGE_TEXT.lower() in str(exc).lower()


def _iter_platforms(experiment, invalid_indices: Optional[Set[int]] = None):
    if invalid_indices is None:
        invalid_indices = set()

    platforms = experiment.Platforms
    count = int(getattr(platforms, "Count", 0) or 0)
    result: List[Dict[str, Any]] = []

    for idx in range(1, count + 1):
        if idx in invalid_indices:
            continue

        try:
            item = platforms.Item(idx)
            name = str(getattr(item, "Name", f"Platform_{idx}"))
            bus = _classify_bus(name)
            result.append({"index": idx, "name": name, "bus": bus, "obj": item})
        except Exception as exc:
            if _is_out_of_range_exception(exc):
                invalid_indices.add(idx)
                if idx not in _warned_invalid_indices:
                    print(f"cd-warn skipping invalid platform index={idx} ({_OUT_OF_RANGE_TEXT})", file=sys.stderr, flush=True)
                    _warned_invalid_indices.add(idx)
            else:
                print(f"cd-warn index={idx} error={exc}", file=sys.stderr, flush=True)

    return result


def _extract_item_value(item: Any) -> Optional[str]:
    value_attrs = [
        "PhysicalValue",
        "Value",
        "RawValue",
        "DisplayValue",
        "State",
        "Text",
    ]

    for attr in value_attrs:
        try:
            value = getattr(item, attr)
        except Exception:
            continue

        if callable(value):
            continue

        if value is None:
            continue

        text = str(value).strip()
        if text:
            return text

    return None


def _collect_platform_samples(platform_obj: Any, max_items_per_collection: int = 8) -> List[str]:
    collection_attrs = [
        "Signals",
        "SignalGroups",
        "Messages",
        "Channels",
        "Variables",
        "MeasurementVariables",
        "Parameters",
        "Elements",
        "Items",
    ]
    samples: List[str] = []

    for collection_name in collection_attrs:
        try:
            collection = getattr(platform_obj, collection_name)
        except Exception:
            continue

        try:
            count = int(getattr(collection, "Count", 0) or 0)
        except Exception:
            continue

        if count <= 0:
            continue

        upper_bound = min(count, max_items_per_collection)
        for item_index in range(1, upper_bound + 1):
            try:
                item = collection.Item(item_index)
            except Exception:
                continue

            item_name = str(getattr(item, "Name", f"{collection_name}_{item_index}"))
            item_value = _extract_item_value(item)
            if item_value is None:
                continue

            samples.append(f"{collection_name}.{item_name}={item_value}")

    return samples


def _try_get_member(obj: Any, member_name: str) -> Any:
    try:
        return getattr(obj, member_name)
    except Exception:
        return None


def _describe_object(name: str, obj: Any) -> List[str]:
    lines: List[str] = []
    if obj is None:
        lines.append(f"[{name}] <unavailable>")
        return lines

    lines.append(f"[{name}]")
    try:
        members = sorted(set(dir(obj)))
    except Exception as exc:
        lines.append(f"  <dir failed: {exc}>")
        return lines

    for member in members:
        if member.startswith("_"):
            continue
        try:
            value = getattr(obj, member)
            kind = "method" if callable(value) else "property"
            lines.append(f"  {kind}: {member}")
        except Exception:
            lines.append(f"  unknown: {member}")

    return lines


def _invoke_zero_arg(obj: Any, member_name: str):
    try:
        member = getattr(obj, member_name)
    except Exception as exc:
        return None, f"getattr failed: {exc}"

    if not callable(member):
        return member, None

    try:
        return member(), None
    except Exception as exc:
        return None, str(exc)


def probe_api(output_path: str) -> int:
    try:
        app, experiment = _connect_controldesk()
    except Exception as exc:
        print(f"cd-error connect failed: {exc}", file=sys.stderr)
        return 2

    sections: List[str] = []
    sections.append(f"ControlDesk Version: {getattr(app, 'Version', 'unknown')}")
    sections.append(f"Active Experiment: {getattr(experiment, 'Name', 'unknown')}")
    sections.append("")

    probe_targets: List[tuple[str, Any]] = [
        ("Application", app),
        ("ActiveExperiment", experiment),
        ("Application.Measurement", _try_get_member(app, "Measurement")),
        ("Application.Recorder", _try_get_member(app, "Recorder")),
        ("Application.Recording", _try_get_member(app, "Recording")),
        ("ActiveExperiment.Measurement", _try_get_member(experiment, "Measurement")),
        ("ActiveExperiment.Recorder", _try_get_member(experiment, "Recorder")),
        ("ActiveExperiment.Recording", _try_get_member(experiment, "Recording")),
        ("ActiveExperiment.Platforms", _try_get_member(experiment, "Platforms")),
    ]

    for target_name, target_obj in probe_targets:
        sections.extend(_describe_object(target_name, target_obj))
        sections.append("")

    sections.append("[DeepMethodProbe]")
    sections.append("  Safely invoking likely zero-arg accessors to find recorder/measurement APIs")
    sections.append("")

    app_method_candidates = [
        "MeasurementDataManagement",
        "BusNavigator",
        "PlatformManagement",
        "VariablesManagement",
        "DataSetManagement",
        "CalibrationManagement",
        "DiagnosticsManagement",
        "ProjectManagement",
        "Log",
        "TimeCursorManagement",
    ]

    exp_method_candidates = [
        "Files",
        "Mappings",
        "Platforms",
        "SimulationTimeGroups",
        "CalculatedVariablesConfiguration",
    ]

    for method_name in app_method_candidates:
        value, error = _invoke_zero_arg(app, method_name)
        if error is not None:
            sections.append(f"[Application.{method_name}] <invoke failed: {error}>")
            sections.append("")
            continue

        sections.extend(_describe_object(f"Application.{method_name}()", value))
        sections.append("")

    for method_name in exp_method_candidates:
        value, error = _invoke_zero_arg(experiment, method_name)
        if error is not None:
            sections.append(f"[ActiveExperiment.{method_name}] <invoke failed: {error}>")
            sections.append("")
            continue

        sections.extend(_describe_object(f"ActiveExperiment.{method_name}()", value))
        sections.append("")

    try:
        os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
        with open(output_path, "w", encoding="utf-8") as handle:
            handle.write("\n".join(sections))
    except Exception as exc:
        print(f"cd-error failed to write probe output: {exc}", file=sys.stderr)
        return 3

    print(f"cd-probe wrote: {output_path}")
    return 0


def list_platforms() -> int:
    try:
        app, experiment = _connect_controldesk()
    except Exception as exc:
        print(f"cd-error connect failed: {exc}", file=sys.stderr)
        return 2

    print(f"cd-version={getattr(app, 'Version', 'unknown')}")
    print(f"cd-experiment={getattr(experiment, 'Name', 'unknown')}")

    entries = _iter_platforms(experiment)
    bus_entries = [entry for entry in entries if entry["bus"] != "OTHER"]

    if not bus_entries:
        print("cd-info no bus platforms discovered")
        return 0

    for index, entry in enumerate(bus_entries, start=1):
        name = entry["name"]
        bus = entry["bus"]
        print(f"ch={index} -> {bus}:{name}")
        print(f"cd-platform idx={entry['index']} bus={bus} name={name}")

    return 0


def capture_all(duration_ms: Optional[int], output_dir: str, poll_ms: int, log_format: str) -> int:
    normalized_format = (log_format or "text").strip().lower()
    if normalized_format in {"asc", "blf", "mdf", "mf4"}:
        print(
            "cd-error requested log format requires real bus-frame recorder data; "
            "ControlDesk platform snapshot API cannot produce industry-standard CAN frame logs.",
            file=sys.stderr,
        )
        print(
            "cd-hint use ControlDesk recorder for .asc/.blf/.mdf and use this script only for topology snapshots.",
            file=sys.stderr,
        )
        return 4

    os.makedirs(output_dir, exist_ok=True)

    meta_path = os.path.join(output_dir, "controldesk_bus_capture.log")
    started = time.time()

    try:
        app, experiment = _connect_controldesk()
    except Exception as exc:
        print(f"cd-error connect failed: {exc}", file=sys.stderr)
        return 2

    print(f"cd-capture started version={getattr(app, 'Version', 'unknown')} format={normalized_format}")
    print(f"cd-capture experiment={getattr(experiment, 'Name', 'unknown')}")
    print(f"cd-capture output={meta_path}")
    print("cd-capture mode=platform_snapshot_with_best_effort_values")
    sys.stdout.flush()

    invalid_indices: Set[int] = set()

    with open(meta_path, "a", encoding="utf-8") as handle:
        while True:
            now = datetime.datetime.now().isoformat(timespec="milliseconds")
            entries = _iter_platforms(experiment, invalid_indices)
            bus_entries = [entry for entry in entries if entry["bus"] != "OTHER"]

            if not bus_entries:
                line = f"{now} NO_BUS_INTERFACES"
                handle.write(line + "\n")
            else:
                for index, entry in enumerate(bus_entries, start=1):
                    name = entry["name"]
                    bus = entry["bus"]
                    base = f"{now} ch={index} bus={bus} name={name} platformIndex={entry['index']}"
                    samples = _collect_platform_samples(entry.get("obj"))

                    if samples:
                        line = base + " samples=" + " | ".join(samples)
                    else:
                        line = base

                    handle.write(line + "\n")

                    bus_file = os.path.join(output_dir, f"{bus}_{_safe_name(name)}.log")
                    with open(bus_file, "a", encoding="utf-8") as bus_handle:
                        bus_handle.write(line + "\n")

            handle.flush()
            print(f"cd-capture tick entries={len(bus_entries)}")
            sys.stdout.flush()

            if duration_ms is not None:
                elapsed_ms = int((time.time() - started) * 1000)
                if elapsed_ms >= duration_ms:
                    break

                time.sleep(max(0.1, poll_ms / 1000.0))

    print("cd-capture finished")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="ControlDesk bus interface capture helper")
    parser.add_argument("--list-platforms", action="store_true", help="List bus-like platforms from active experiment")
    parser.add_argument("--capture-all", action="store_true", help="Continuously snapshot all bus-like platforms")
    parser.add_argument("--duration-ms", type=int, default=None, help="Optional capture duration in milliseconds")
    parser.add_argument("--output-dir", default="CAN_LOGS", help="Output directory")
    parser.add_argument("--poll-ms", type=int, default=1000, help="Polling interval in milliseconds")
    parser.add_argument("--format", default="text", help="Requested output format label")
    parser.add_argument("--probe-api", action="store_true", help="Dump COM members to discover recorder/measurement APIs")
    parser.add_argument("--probe-out", default="CAN_LOGS/controldesk_api_probe.txt", help="Path for --probe-api output")
    args = parser.parse_args()

    try:
        if args.probe_api:
            return probe_api(args.probe_out)

        if args.capture_all:
            return capture_all(args.duration_ms, args.output_dir, args.poll_ms, args.format)

        return list_platforms()
    except KeyboardInterrupt:
        print("cd-capture interrupted by user", file=sys.stderr)
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
