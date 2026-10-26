"""Prepare pinned Windows ExifTool with a UTF-8 process manifest."""
import hashlib
import os
import shutil
import subprocess
import tempfile
import urllib.request
import zipfile
from pathlib import Path

VERSION = "13.59"
SHA256 = "44b512b25af500724ba579d0a53c8fc5851628b692dd5e5d94ae4a15c2cba9ec"
URL = f"https://downloads.sourceforge.net/project/exiftool/exiftool-{VERSION}_64.zip"


def main():
    if os.name != "nt":
        raise SystemExit("Run on Windows with the Windows SDK installed")
    # The upstream CC0 launcher uses ANSI Win32 paths to find Perl. Opt just
    # this process into UTF-8; do not change the user's system locale or registry.
    mt = shutil.which("mt.exe")
    if mt is None:
        sdk = Path(os.environ["ProgramFiles(x86)"]) / "Windows Kits/10/bin"
        versions = sorted((p for p in sdk.iterdir() if p.name.startswith("10.") and (p / "x64/mt.exe").is_file()),
                          key=lambda p:tuple(map(int,p.name.split("."))))
        if not versions:
            raise SystemExit("Windows SDK manifest tool (mt.exe) is required")
        mt = str(versions[-1] / "x64/mt.exe")
    target = Path(__file__).resolve().parents[1] / "src-tauri/resources/exiftool-win"
    with tempfile.TemporaryDirectory(prefix="framemoor-exiftool-win-") as temporary:
        archive = Path(temporary) / "exiftool.zip"
        request = urllib.request.Request(URL, headers={"User-Agent": "Framemoor resource preparation"})
        with urllib.request.urlopen(request, timeout=90) as response:
            archive.write_bytes(response.read())
        if hashlib.sha256(archive.read_bytes()).hexdigest() != SHA256:
            raise SystemExit("ExifTool archive checksum does not match the pinned official release")
        with zipfile.ZipFile(archive) as package:
            package.extractall(temporary)
        source = Path(temporary) / f"exiftool-{VERSION}_64"
        if not (source / "exiftool_files/LICENSE").is_file():
            raise SystemExit("Incomplete ExifTool distribution")
        if target.exists():
            shutil.rmtree(target)
        shutil.copytree(source, target)
        (target / "exiftool(-k).exe").rename(target / "exiftool.exe")
        manifest = Path(__file__).resolve().parents[1] / "windows/exiftool-utf8.manifest"
        subprocess.run([mt, "-nologo", "-manifest", str(manifest), f"-outputresource:{target / 'exiftool.exe'};#1"], check=True)
        shutil.copy2(manifest, target / manifest.name)
        (target / "FRAMEMOOR-PATCH.txt").write_text(
            f"Original ExifTool {VERSION} x64 archive SHA-256: {SHA256}\n"
            "Framemoor modifies only the CC0 launcher's process manifest to enable UTF-8.\n"
            "The original longPathAware flag is retained. No Perl or ExifTool code is modified.\n"
            "Requires Windows 10 version 1903 or newer.\n"
            "Manifest source: desktop/windows/exiftool-utf8.manifest\n", encoding="utf-8")
        # Fail before compiling the app if the runtime cannot start under an
        # installation path outside the runner's legacy ANSI code page.
        unicode_install = Path(temporary) / "中文 安装目录"
        shutil.copytree(target, unicode_install)
        version = subprocess.check_output([str(unicode_install / "exiftool.exe"), "-ver"],
                                          encoding="utf-8", creationflags=subprocess.CREATE_NO_WINDOW).strip()
        if version != VERSION:
            raise SystemExit(f"Unexpected installed ExifTool version: {version}")
    print(f"Prepared ExifTool {VERSION} x64 with UTF-8 manifest; Unicode runtime check passed")


if __name__ == "__main__":
    main()
