"""Collect source-package licenses for the Windows application bundle."""
import json
import shutil
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[2]
desktop = root / "desktop"
target = desktop / "src-tauri/resources/notices"
if target.exists():
    shutil.rmtree(target)
target.mkdir(parents=True)
for name in ["LICENSE", "THIRD_PARTY_NOTICES.md"]:
    shutil.copy2(root / name, target / name)
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--format-version", "1", "--locked",
    "--manifest-path", str(desktop / "src-tauri/Cargo.toml"),
    "--filter-platform", "x86_64-pc-windows-msvc",
], text=True))
packages = []
for package in metadata["packages"]:
    if package.get("source") is None:
        continue
    source = Path(package["manifest_path"]).parent
    files = [p for p in source.iterdir() if p.is_file() and p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT"))]
    if package.get("license_file"):
        files.append(source / package["license_file"])
    destination = target / "rust" / (package["name"] + "-" + package["version"])
    destination.mkdir(parents=True)
    for path in set(files):
        shutil.copy2(path, destination / path.name)
    packages.append({"name":package["name"], "version":package["version"], "license":package.get("license"), "repository":package.get("repository"), "notices":[p.name for p in files]})
# Production npm packages only; collect their original notices, not generated text.
lock = json.loads((desktop / "package-lock.json").read_text())
for relative, package in lock["packages"].items():
    if not relative or package.get("dev") or package.get("optional"):
        continue
    source = desktop / relative
    if not source.is_dir():
        continue
    files = [p for p in source.iterdir() if p.is_file() and p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT"))]
    destination = target / "javascript" / relative.replace("node_modules/", "").replace("/", "_")
    destination.mkdir(parents=True)
    for path in files:
        shutil.copy2(path, destination / path.name)
    packages.append({"name":relative, "version":package["version"], "license":package.get("license"), "notices":[p.name for p in files]})
(target / "dependencies.json").write_text(json.dumps(packages, indent=2), encoding="utf-8")
revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
(target / "SOURCE.txt").write_text(
    "Framemoor / 泊影 — GPL-3.0-only\nSource revision: " + revision +
    "\nhttps://github.com/wzk112/Framemoor/tree/" + revision +
    "\nBuild instructions and dependency lockfiles are in the source repository.\n"
    "ExifTool source: https://github.com/exiftool/exiftool/tree/2200871d9cef988051d2a99d67df3bda6cbb30a8\n",
    encoding="utf-8",
)
print(f"Collected notices for {len(packages)} dependency packages")
missing=[p["name"] for p in packages if not p["notices"]]
if missing:
    print("Packages without a top-level license file (see dependency metadata):", ", ".join(missing))
