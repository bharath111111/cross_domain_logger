import argparse
import datetime
import os
import shutil
import zipfile
from pathlib import Path
from typing import List


EXTENSIONS = [".asc", ".blf", ".mdf", ".mf4", ".pcapng", ".pcap", ".log", ".txt"]


def _timestamp() -> str:
    return datetime.datetime.now().strftime("%Y%m%d_%H%M%S")


def _find_files(source_dirs: List[Path], recursive: bool) -> List[Path]:
    found: List[Path] = []
    for source_dir in source_dirs:
        if not source_dir.exists() or not source_dir.is_dir():
            continue

        if recursive:
            walker = source_dir.rglob("*")
        else:
            walker = source_dir.glob("*")

        for item in walker:
            if item.is_file() and item.suffix.lower() in EXTENSIONS:
                found.append(item)

    found = sorted(set(found), key=lambda p: (str(p.parent).lower(), p.name.lower()))
    return found


def _copy_files(files: List[Path], target_data_dir: Path) -> List[Path]:
    copied: List[Path] = []
    collision_count = {}

    for source in files:
        name = source.name
        target = target_data_dir / name
        if target.exists():
            base = source.stem
            ext = source.suffix
            key = name.lower()
            collision_count[key] = collision_count.get(key, 0) + 1
            target = target_data_dir / f"{base}_{collision_count[key]}{ext}"

        shutil.copy2(source, target)
        copied.append(target)

    return copied


def _write_manifest(pack_dir: Path, copied_files: List[Path], source_dirs: List[Path], note: str) -> None:
    manifest = pack_dir / "metadata.txt"
    now = datetime.datetime.now().isoformat(timespec="seconds")

    with manifest.open("w", encoding="utf-8") as handle:
        handle.write(f"Generated: {now}\n")
        handle.write("Purpose: Triage package for CAN/LIN/Ethernet analysis\n\n")
        handle.write("Source directories:\n")
        for src in source_dirs:
            handle.write(f"- {src}\n")

        handle.write("\nIncluded files:\n")
        for file_path in copied_files:
            size = file_path.stat().st_size
            handle.write(f"- {file_path.name} ({size} bytes)\n")

        if note.strip():
            handle.write("\nNotes:\n")
            handle.write(note.strip() + "\n")


def _zip_directory(pack_dir: Path, zip_path: Path) -> None:
    with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for item in pack_dir.rglob("*"):
            if item.is_file():
                archive.write(item, arcname=item.relative_to(pack_dir))


def main() -> int:
    parser = argparse.ArgumentParser(description="Create triage package zip from captured log files")
    parser.add_argument("--source-dir", action="append", default=["CAN_LOGS"], help="Source directory to scan (repeatable)")
    parser.add_argument("--output-dir", default="dist", help="Output directory for triage packages")
    parser.add_argument("--name", default=None, help="Optional package base name")
    parser.add_argument("--note", default="", help="Optional note to include in metadata.txt")
    parser.add_argument("--non-recursive", action="store_true", help="Disable recursive scan")
    args = parser.parse_args()

    source_dirs = [Path(path).resolve() for path in args.source_dir]
    output_dir = Path(args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    stamp = _timestamp()
    base_name = args.name if args.name else f"triage_pack_{stamp}"

    pack_dir = output_dir / base_name
    if pack_dir.exists():
        shutil.rmtree(pack_dir)
    data_dir = pack_dir / "data"
    data_dir.mkdir(parents=True, exist_ok=True)

    files = _find_files(source_dirs, recursive=not args.non_recursive)
    if not files:
        print("No matching log files found (.asc/.blf/.mdf/.mf4/.pcapng/.pcap/.log/.txt).")
        return 1

    copied = _copy_files(files, data_dir)
    _write_manifest(pack_dir, copied, source_dirs, args.note)

    zip_path = output_dir / f"{base_name}.zip"
    if zip_path.exists():
        zip_path.unlink()
    _zip_directory(pack_dir, zip_path)

    print(f"triage-pack-folder: {pack_dir}")
    print(f"triage-pack-zip: {zip_path}")
    print(f"files-included: {len(copied)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
