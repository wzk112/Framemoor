"""Collect locked dependency licenses for the selected application target."""
import argparse
import json
import hashlib
import shutil
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[2]
desktop = root / "desktop"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--target", help="Rust target triple; defaults to rustc's host target")
args = parser.parse_args()
rust_target = args.target
if not rust_target:
    rust_target = next(line.removeprefix("host: ") for line in subprocess.check_output(
        ["rustc", "-vV"], encoding="utf-8"
    ).splitlines() if line.startswith("host: "))
target = desktop / "src-tauri/resources/notices"
if target.exists():
    shutil.rmtree(target)
target.mkdir(parents=True)
for name in ["LICENSE", "THIRD_PARTY_NOTICES.md"]:
    shutil.copy2(root / name, target / name)
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--format-version", "1", "--locked",
    "--manifest-path", str(desktop / "src-tauri/Cargo.toml"),
    "--filter-platform", rust_target,
], encoding="utf-8"))
resolved_packages = {node["id"] for node in metadata["resolve"]["nodes"]}
packages = []
fallback_root = desktop / "licenses"
fallbacks = json.loads((fallback_root / "sources.json").read_text(encoding="utf-8"))
shutil.copy2(fallback_root / "sources.json", target / "license-sources.json")
for package in metadata["packages"]:
    if package.get("source") is None or package["id"] not in resolved_packages:
        continue
    source = Path(package["manifest_path"]).parent
    files = [p for p in source.iterdir() if p.is_file() and p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT"))]
    if package.get("license_file"):
        files.append(source / package["license_file"])
    fallback = fallbacks.get(package["name"])
    if not files and fallback:
        if fallback["version"] != package["version"]:
            raise SystemExit(f"Review license fallback for {package['name']} {package['version']}")
        notice = fallback_root / fallback["file"]
        if hashlib.sha256(notice.read_bytes()).hexdigest() != fallback["sha256"]:
            raise SystemExit(f"License fallback hash mismatch: {notice.name}")
        files.append(notice)
    destination = target / "rust" / (package["name"] + "-" + package["version"])
    destination.mkdir(parents=True)
    for path in set(files):
        shutil.copy2(path, destination / path.name)
    packages.append({"name":package["name"], "version":package["version"], "license":package.get("license"), "repository":package.get("repository"), "notices":[p.name for p in files]})
# Production npm packages only; collect their original notices, not generated text.
lock = json.loads((desktop / "package-lock.json").read_text(encoding="utf-8"))
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
revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, encoding="utf-8").strip()
(target / "SOURCE.txt").write_text(
    "Framemoor / 泊影 — GPL-3.0-only\nSource revision: " + revision +
    "\nBuild target: " + rust_target +
    "\nhttps://github.com/wzk112/Framemoor/tree/" + revision +
    "\nBuild instructions and dependency lockfiles are in the source repository.\n"
    "ExifTool source: https://github.com/exiftool/exiftool/tree/2200871d9cef988051d2a99d67df3bda6cbb30a8\n",
    encoding="utf-8",
)
print(f"Collected notices for {len(packages)} dependency packages ({rust_target})")
missing=[p["name"] for p in packages if not p["notices"]]
if missing:
    raise SystemExit("Missing dependency notices: " + ", ".join(missing))
