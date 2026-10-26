"""Fetch the complete, checksum-pinned official Windows ExifTool distribution."""
import hashlib
import shutil
import tempfile
import urllib.request
import zipfile
from pathlib import Path

VERSION = "13.59"
SHA256 = "44b512b25af500724ba579d0a53c8fc5851628b692dd5e5d94ae4a15c2cba9ec"
URL = f"https://downloads.sourceforge.net/project/exiftool/exiftool-{VERSION}_64.zip"


def main():
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
    print(f"Prepared official ExifTool {VERSION} x64; SHA-256 {SHA256}")


if __name__ == "__main__":
    main()
