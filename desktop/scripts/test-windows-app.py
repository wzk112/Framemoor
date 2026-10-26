"""Install and exercise the real Windows app through tauri-driver/WebView2.

Uses disposable synthetic media and real IPC. No app commands are mocked.
The standard-library WebDriver client keeps test tooling out of the application.
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import time
import traceback
import urllib.error
import urllib.request
import zlib

ELEMENT = "element-6066-11e4-a52e-4f735466cecf"


def wait(action, timeout=60):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        try:
            result = action()
            if result:
                return result
        except Exception as error:
            last = error
        time.sleep(.2)
    raise AssertionError(f"Timed out waiting for condition: {last}")


class Driver:
    session = None

    def request(self, method, path, data=None):
        body = None if data is None else json.dumps(data).encode()
        request = urllib.request.Request("http://127.0.0.1:4444" + path, data=body, method=method,
                                         headers={"Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(request, timeout=150) as response:
                value = json.load(response).get("value")
        except urllib.error.HTTPError as error:
            raise AssertionError(error.read().decode()) from error
        if isinstance(value, dict) and value.get("error"):
            raise AssertionError(value)
        return value

    def call(self, method, path, data=None):
        return self.request(method, f"/session/{self.session}" + path, data)

    def start(self, binary):
        value = self.request("POST", "/session", {"capabilities": {"alwaysMatch": {
            "tauri:options": {"application": str(binary)},
        }}})
        self.session = value["sessionId"]
        self.call("POST", "/timeouts", {"script":120000, "pageLoad":120000, "implicit":0})

    def js(self, script, *args):
        return self.call("POST", "/execute/sync", {"script":script, "args":list(args)})

    def invoke(self, command, args=None):
        result = self.call("POST", "/execute/async", {
            "script":"const done=arguments[arguments.length-1]; window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(value=>done({ok:true,value})).catch(error=>done({ok:false,error:String(error)}));",
            "args":[command, args or {}],
        })
        assert result["ok"], result
        return result.get("value")

    def element(self, xpath):
        return wait(lambda:self.call("POST", "/element", {"using":"xpath", "value":xpath}))[ELEMENT]

    def click(self, xpath):
        element = self.element(xpath)
        wait(lambda:self.call("GET", f"/element/{element}/enabled"))
        self.call("POST", f"/element/{element}/click", {})

    def button(self, text):
        self.click(f"//button[normalize-space(.)='{text}']")

    def field(self, label, value):
        element = self.element(f"//label[contains(.,'{label}')]//input")
        if self.call("GET", f"/element/{element}/attribute/type") == "date":
            self.js("const e=arguments[0];Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(e,arguments[1]);e.dispatchEvent(new Event('input',{bubbles:true}));e.dispatchEvent(new Event('change',{bubbles:true}));", {ELEMENT:element}, str(value))
            return
        self.call("POST", f"/element/{element}/clear", {})
        self.call("POST", f"/element/{element}/value", {"text":str(value)})

    def select(self, element_id, value):
        self.js("const e=document.getElementById(arguments[0]);e.value=arguments[1];e.dispatchEvent(new Event('change',{bubbles:true}));", element_id, value)

    def body(self):
        return self.js("return document.body.innerText")

    def screenshot(self, path):
        path.write_bytes(base64.b64decode(self.call("GET", "/screenshot")))

    def close(self):
        if self.session:
            self.call("DELETE", "")
            self.session = None


def sha(path):
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def png(path):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 2, 2, 8, 2, 0, 0, 0))
                    + chunk(b"IDAT", zlib.compress(b"\0\x40\x80\xc0\x40\x80\xc0" * 2)) + chunk(b"IEND", b""))


def metadata(exiftool, path, date):
    # UTF-8 stdin arguments also work on Windows with a non-English filename.
    arguments = "\n".join(["-overwrite_original", "-Model=Synthetic Camera", f"-DateTimeOriginal={date}",
                             "-OffsetTimeOriginal=+11:00", str(path)]) + "\n"
    subprocess.run([str(exiftool), "-charset", "filename=UTF8", "-@", "-"], input=arguments.encode(),
                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, timeout=30)


def fixture(root, exiftool):
    source = root / "中文 相机卡"
    folder = source / "DCIM/100TEST"
    first = folder / "照片_0001.PNG"
    older = folder / "照片_0002.PNG"
    for path, date in [(first,"2026:10:20 20:00:00"),(older,"2026:10:19 20:00:00")]:
        png(path)
        metadata(exiftool, path, date)
    sidecar = first.with_suffix(".XMP")
    sidecar.write_text('<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"/></x:xmpmeta>', encoding="utf-8")
    video = folder / "视频_0001.MP4"
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=blue:s=64x64:r=10",
                    "-t", "1", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-metadata", "creation_time=2026-10-20T09:00:00Z", str(video)],
                   check=True, timeout=60)
    return source, {str(p.relative_to(source)).replace("\\","/"):sha(p) for p in source.rglob("*") if p.is_file()}


def latest(driver):
    tasks = driver.invoke("bootstrap")["tasks"]
    return tasks[0] if tasks else None


def completed(driver, task_id):
    task = driver.invoke("get_task", {"taskId":task_id})
    if task["status"] == "failed":
        raise RuntimeError(task["error"])
    return task if task["status"] == "complete" else None


def check_copy(task, expected, source_hashes):
    assert task["status"] == "complete", task
    rows = task["preview"]["rows"]
    assert set(row["source"] for row in rows) == expected, rows
    for row in rows:
        output = Path(task["preview"]["destination"]) / row["destination"]
        assert sha(output) == source_hashes[row["source"]], row
    assert task["report"]["content_verified"] == task["verification"]["enabled"], task
    assert not task.get("system_warning"), task.get("system_warning")


def run_ui(driver, source, destination, verified, language, results):
    labels = ("素材来源", "目标文件夹", "扫描素材", "继续", "生成预览", "开始导入", "已确认以上提示") if language == "zh" else (
        "Media source", "Destination folder", "Scan media", "Continue", "Build preview", "Import without verification", "I have reviewed these warnings")
    destination.mkdir()
    driver.field(labels[0], source)
    driver.field(labels[1], destination)
    driver.button(labels[2])
    # A date filter exercises actual metadata; extension-only scanning cannot pass.
    wait(lambda:"2026-10-20" in driver.body())
    driver.field("开始日期" if language == "zh" else "From", "2026-10-20")
    driver.field("结束日期" if language == "zh" else "To", "2026-10-20")
    driver.button(labels[3])
    driver.element("//select[@id='layout-preset']")
    driver.select("layout-preset", "builtin:original")
    driver.button(labels[4])
    wait(lambda:"视频_0001.MP4" in driver.body())
    assert "2026-10-20" in driver.body()
    # Unknown camera layout / QuickTime UTC interpretation requires acknowledgement.
    if driver.js("return !!document.querySelector('.acknowledgement input')"):
        driver.click("//label[@class='acknowledgement']//input")
    driver.screenshot(results / f"preview-{language}.png")
    if not verified:
        assert driver.js("return document.querySelectorAll('.verification-warning').length") > 0
    previous={t["id"] for t in driver.invoke("bootstrap")["tasks"]}
    driver.button(labels[5])
    task = wait(lambda:next((t for t in driver.invoke("bootstrap")["tasks"] if t["id"] not in previous),None))
    task = wait(lambda:completed(driver, task["id"]), timeout=120)
    wait(lambda:("导入完成" if language == "zh" else "Import complete") in driver.body())
    driver.screenshot(results / f"complete-{language}.png")
    return task


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--installer", type=Path, required=True)
    parser.add_argument("--native-driver", type=Path, required=True)
    parser.add_argument("--results", type=Path, default=Path("native-test-results"))
    args=parser.parse_args()
    results=args.results.resolve();results.mkdir(parents=True,exist_ok=True)
    root=Path(tempfile.mkdtemp(prefix="framemoor-windows-"))
    install=root / "泊影 App"
    subprocess.run([str(args.installer.resolve()), "/S", f"/D={install}"], check=True, timeout=180)
    binary=install / "framemoor-desktop.exe"
    assert binary.is_file(), list(install.rglob("*.exe"))
    exiftool=install / "exiftool-win/exiftool.exe"
    assert exiftool.is_file(), list(install.iterdir())
    assert subprocess.check_output([str(exiftool),"-ver"],text=True).strip()=="13.59"
    assert (install / "notices/LICENSE").is_file()
    source, source_hashes=fixture(root, exiftool)
    expected={name for name in source_hashes if "照片_0002" not in name}
    environment=dict(os.environ, FRAMEMOOR_DATA_DIR=str(root / "state"), WEBVIEW2_USER_DATA_FOLDER=str(root / "webview"))
    environment.pop("FRAMEMOOR_EXIFTOOL",None);environment.pop("CARD_INGEST_EXIFTOOL",None)
    driver=Driver();process=None
    report={"installer":args.installer.name,"installer_sha256":sha(args.installer),"checks":[],"root":str(root)}
    with (results / "driver.log").open("w",encoding="utf-8") as log:
        try:
            process=subprocess.Popen(["tauri-driver","--native-driver",str(args.native_driver.resolve())],env=environment,stdout=log,stderr=log)
            wait(lambda:driver.request("GET","/status"))
            driver.start(binary)
            wait(lambda:"素材来源" in driver.body())
            assert driver.invoke("bootstrap")["metadata_ready"]
            report["checks"].append("NSIS install, packaged ExifTool, real WebView2 startup")
            verified=run_ui(driver,source,root / "中文 已校验",True,"zh",results)
            check_copy(verified,expected,source_hashes)
            assert all("metadata_unavailable" not in row["issues"] for row in verified["preview"]["rows"])
            assert any(row["codec"]=="H264" for row in verified["preview"]["rows"])
            report["checks"].append("Chinese UI, Unicode paths, date filter, H.264 metadata, sidecars, SHA-256 copy")
            driver.button("设置");driver.select("settings-appearance","dark");driver.select("settings-language","en")
            driver.button("Performance")
            driver.click("//label[contains(.,'SHA-256')]//input[@type='checkbox']")
            wait(lambda:driver.invoke("bootstrap")["settings"].get("verification",{}).get("enabled") is False)
            driver.click("//button[@aria-label='Close settings']")
            driver.button("New import")
            unverified=run_ui(driver,source,root / "英文 未校验",False,"en",results)
            check_copy(unverified,expected,source_hashes)
            report["checks"].append("English/dark UI, disabled-verification warning, unverified copy/report")
            exported=results / "import-report.json"
            driver.invoke("export_report",{"taskId":verified["id"],"path":str(exported)})
            assert json.loads(exported.read_text(encoding="utf-8"))["task"]["report"]["content_verified"]
            # Native preview must block existing destinations without replacing bytes.
            catalog=driver.invoke("scan_source",{"source":str(source)})
            options=dict(verified["preview"]["options"])
            collision=driver.invoke("build_preview",{"catalogId":catalog["id"],"options":options})
            assert any(item.startswith("exists|") for item in collision["blockers"]),collision
            report["checks"].append("Existing destination conflicts are blocked")
            assert source_hashes=={str(p.relative_to(source)).replace("\\","/"):sha(p) for p in source.rglob("*") if p.is_file()}
            report["checks"].append("All source bytes unchanged")
            assert driver.js("return document.documentElement.lang")=="en"
            driver.close()
            driver.start(binary)
            wait(lambda:"Media source" in driver.body())
            assert driver.js("return document.documentElement.dataset.theme")=="dark"
            assert len(driver.invoke("bootstrap")["tasks"])==2
            driver.screenshot(results / "restart-english-dark.png")
            report["checks"].append("Settings and completed tasks survive application restart")
            report["native_checks_passed"]=True
        except Exception:
            report["passed"]=False
            report["error"]=traceback.format_exc()
            if driver.session:
                try:
                    (results / "failure-dom.html").write_text(driver.js("return document.documentElement.outerHTML"),encoding="utf-8")
                    driver.screenshot(results / "failure.png")
                except Exception:
                    pass
            raise
        finally:
            (results / "results.json").write_text(json.dumps(report,indent=2,ensure_ascii=False),encoding="utf-8")
            try:
                driver.close()
            except Exception:
                pass
            if process:
                subprocess.run(["taskkill","/PID",str(process.pid),"/T","/F"],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    # Uninstall after closing the real app; the runner itself is disposable.
    uninstaller=install / "uninstall.exe"
    assert uninstaller.is_file()
    subprocess.run([str(uninstaller),"/S"],check=True,timeout=120)
    wait(lambda:not binary.exists())
    report["checks"].append("NSIS uninstall removes installed executable")
    report["passed"]=True
    (results / "results.json").write_text(json.dumps(report,indent=2,ensure_ascii=False),encoding="utf-8")
    print(json.dumps(report,indent=2,ensure_ascii=False))


if __name__=="__main__":
    main()
