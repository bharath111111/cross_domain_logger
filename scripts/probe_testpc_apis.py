import argparse
import datetime
import os
import platform
import sys
import traceback
from typing import Any, Dict, List, Optional, Tuple


def _now() -> str:
    return datetime.datetime.now().isoformat(timespec="seconds")


def _safe_repr(value: Any, max_len: int = 200) -> str:
    try:
        text = repr(value)
    except Exception:
        try:
            text = str(value)
        except Exception:
            text = "<unprintable>"
    if len(text) > max_len:
        return text[: max_len - 3] + "..."
    return text


def _describe_members(name: str, obj: Any, max_items: int = 250) -> List[str]:
    lines: List[str] = [f"[{name}]"]
    if obj is None:
        lines.append("  <unavailable>")
        lines.append("")
        return lines

    try:
        members = sorted(set(dir(obj)))
    except Exception as exc:
        lines.append(f"  <dir failed: {exc}>")
        lines.append("")
        return lines

    shown = 0
    for member in members:
        if member.startswith("_"):
            continue
        if shown >= max_items:
            lines.append("  ... truncated ...")
            break
        try:
            value = getattr(obj, member)
            kind = "method" if callable(value) else "property"
            lines.append(f"  {kind}: {member}")
        except Exception as exc:
            lines.append(f"  unknown: {member} ({exc})")
        shown += 1

    lines.append("")
    return lines


def _invoke_zero_arg(obj: Any, member_name: str) -> Tuple[Optional[Any], Optional[str]]:
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


def _probe_registry(prefixes: List[str]) -> List[str]:
    lines: List[str] = ["[RegistryScan]"]
    try:
        import winreg  # type: ignore
    except Exception as exc:
        lines.append(f"  winreg unavailable: {exc}")
        lines.append("")
        return lines

    checked_roots = [
        (winreg.HKEY_CLASSES_ROOT, "HKCR"),
    ]

    for root, root_name in checked_roots:
        lines.append(f"  Root: {root_name}")
        try:
            key = winreg.OpenKey(root, "")
        except Exception as exc:
            lines.append(f"    open failed: {exc}")
            continue

        idx = 0
        matches: List[str] = []
        try:
            while True:
                subkey = winreg.EnumKey(key, idx)
                idx += 1
                lower = subkey.lower()
                if any(lower.startswith(prefix.lower()) for prefix in prefixes):
                    matches.append(subkey)
        except OSError:
            pass

        if not matches:
            lines.append("    no matches")
        else:
            for item in matches[:200]:
                lines.append(f"    {item}")
            if len(matches) > 200:
                lines.append("    ... truncated ...")

    lines.append("")
    return lines


def run_probe(output_path: str) -> int:
    lines: List[str] = []
    lines.append(f"Generated: {_now()}")
    lines.append(f"Python: {sys.version}")
    lines.append(f"Platform: {platform.platform()}")
    lines.append("")

    try:
        import pythoncom  # type: ignore
        import win32com.client  # type: ignore
    except Exception as exc:
        lines.append(f"ERROR: pywin32 import failed: {exc}")
        lines.append("Install via: py -3 -m pip install pywin32")
        lines.append("")
        os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
        with open(output_path, "w", encoding="utf-8") as handle:
            handle.write("\n".join(lines))
        return 2

    progids = [
        "ControlDeskNG.Application",
        "ControlDesk.Application",
        "dSPACE.ControlDesk.Application",
        "AutomationDesk.Application",
        "dSPACE.AutomationDesk.Application",
        "dSPACE.MeasurementDataAPI.Measurements",
        "dSPACE.MeasurementDataAPI.Measurements.2023-A",
    ]

    lines.append("[ProgIDDispatch]")
    dispatch_results: Dict[str, Any] = {}
    for progid in progids:
        try:
            obj = win32com.client.Dispatch(progid)
            dispatch_results[progid] = obj
            lines.append(f"  OK Dispatch: {progid}")
        except Exception as exc:
            lines.append(f"  FAIL Dispatch: {progid} -> {exc}")
    lines.append("")

    lines.append("[ProgIDActiveObject]")
    for progid in progids:
        try:
            obj = pythoncom.GetActiveObject(progid)
            lines.append(f"  OK ActiveObject: {progid} ({_safe_repr(obj)})")
        except Exception as exc:
            lines.append(f"  FAIL ActiveObject: {progid} -> {exc}")
    lines.append("")

    if "ControlDeskNG.Application" in dispatch_results:
        app = dispatch_results["ControlDeskNG.Application"]
        lines.extend(_describe_members("ControlDeskNG.Application", app))

        for member in [
            "ActiveExperiment",
            "ActiveProject",
            "MeasurementDataManagement",
            "BusNavigator",
            "PlatformManagement",
            "VariablesManagement",
            "ProjectManagement",
            "DataSetManagement",
            "DiagnosticsManagement",
            "CalibrationManagement",
            "TimeCursorManagement",
            "Log",
            "XILAPIEESPort",
        ]:
            value, error = _invoke_zero_arg(app, member)
            if error is not None:
                lines.append(f"[ControlDeskNG.Application.{member}] <invoke failed: {error}>")
                lines.append("")
                continue
            lines.extend(_describe_members(f"ControlDeskNG.Application.{member}", value))

        active_experiment = None
        try:
            active_experiment = app.ActiveExperiment
            lines.append(f"ActiveExperiment.Name: {_safe_repr(getattr(active_experiment, 'Name', None))}")
            lines.append("")
        except Exception as exc:
            lines.append(f"ActiveExperiment read failed: {exc}")
            lines.append("")

        if active_experiment is not None:
            lines.extend(_describe_members("ControlDeskNG.ActiveExperiment", active_experiment))
            for member in ["Platforms", "Files", "Mappings", "SimulationTimeGroups", "CalculatedVariablesConfiguration"]:
                value, error = _invoke_zero_arg(active_experiment, member)
                if error is not None:
                    lines.append(f"[ControlDeskNG.ActiveExperiment.{member}] <invoke failed: {error}>")
                    lines.append("")
                    continue
                lines.extend(_describe_members(f"ControlDeskNG.ActiveExperiment.{member}", value))

    measurement_progids = [
        "dSPACE.MeasurementDataAPI.Measurements",
        "dSPACE.MeasurementDataAPI.Measurements.2023-A",
    ]
    for progid in measurement_progids:
        if progid not in dispatch_results:
            continue

        measurements_obj = dispatch_results[progid]
        lines.extend(_describe_members(progid, measurements_obj))

        common_members = [
            "Count",
            "Item",
            "Open",
            "Create",
            "Load",
            "Add",
            "Start",
            "Stop",
            "Begin",
            "End",
            "Save",
            "SaveAs",
            "Export",
            "Close",
            "FileName",
            "Path",
            "Version",
        ]

        lines.append(f"[{progid}.CommonMemberCheck]")
        for member in common_members:
            try:
                value = getattr(measurements_obj, member)
                kind = "method" if callable(value) else "property"
                lines.append(f"  {kind}: {member}")
            except Exception as exc:
                lines.append(f"  missing: {member} ({exc})")
        lines.append("")

    lines.extend(_probe_registry(["ControlDesk", "dSPACE", "AutomationDesk", "XIL"]))

    lines.append("[Notes]")
    lines.append("  - 'Member not found' during invocation usually means property-only COM member; access style differs.")
    lines.append("  - If recorder APIs are present, they commonly appear under MeasurementDataManagement or project/experiment recorder objects.")
    lines.append("  - Share this file to wire exact start/stop/export automation.")
    lines.append("")

    os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
    try:
        with open(output_path, "w", encoding="utf-8") as handle:
            handle.write("\n".join(lines))
    except Exception:
        traceback.print_exc()
        return 3

    print(f"probe-written: {output_path}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Probe ControlDesk/dSPACE APIs on test PC")
    parser.add_argument("--out", default="CAN_LOGS/testpc_api_probe.txt", help="Output report file path")
    args = parser.parse_args()
    return run_probe(args.out)


if __name__ == "__main__":
    raise SystemExit(main())
