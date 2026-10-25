"""Prepare the pinned ExifTool resource without requiring a global installation.

macOS: python3 scripts/prepare-resources.py
Windows: also pass --windows-package <extracted official ExifTool Windows folder>.
"""
import argparse
import shutil
import subprocess
import tempfile
from pathlib import Path

REVISION = "2200871d9cef988051d2a99d67df3bda6cbb30a8"  # ExifTool 13.59
parser = argparse.ArgumentParser()
parser.add_argument("--source", type=Path, help="Existing ExifTool source checkout")
parser.add_argument("--windows-package", type=Path)
args = parser.parse_args()
resources = Path(__file__).resolve().parents[1] / "src-tauri" / "resources"
with tempfile.TemporaryDirectory(prefix="framemoor-exiftool-") as temporary:
    source = args.source
    if source is None:
        source = Path(temporary) / "source"
        subprocess.run(["git", "init", str(source)], check=True)
        subprocess.run(["git", "-C", str(source), "fetch", "--depth=1",
                        "https://github.com/exiftool/exiftool.git", REVISION], check=True)
        subprocess.run(["git", "-C", str(source), "checkout", "--detach", "FETCH_HEAD"], check=True)
    actual = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    if actual != REVISION:
        raise SystemExit(f"Expected ExifTool revision {REVISION}; received {actual}")
    target = resources / "exiftool"
    target.mkdir(parents=True, exist_ok=True)
    for name in ["exiftool", "LICENSE", "README"]:
        shutil.copy2(source / name, target / name)
    shutil.copytree(source / "lib", target / "lib", dirs_exist_ok=True)
if args.windows_package:
    source = args.windows_package.resolve()
    exe = next((source / name for name in ["exiftool.exe", "exiftool(-k).exe"] if (source / name).is_file()), None)
    if exe is None or not (source / "exiftool_files").is_dir():
        raise SystemExit("Supply the complete extracted official Windows package, including exiftool_files.")
    target = resources / "exiftool-win"
    shutil.copytree(source, target, dirs_exist_ok=True)
    shutil.copy2(exe, target / "exiftool.exe")
print("ExifTool resources prepared.")
