# Windows validation / Windows 验证

## Build and test scope

The `Desktop (Windows x64)` CI job builds an unsigned, release-mode NSIS installer on GitHub's `windows-latest` VM. It installs that exact package into a temporary directory, runs the installed application through `tauri-driver` and Microsoft Edge WebDriver, and uninstalls it. The application uses real WebView2 and real Tauri IPC; these checks do not mock backend commands. Files are generated in disposable directories, and no personal media is used.

The required CI gate includes:

- Rust core tests on Windows and macOS, frontend unit/browser tests, and both native desktop backends.
- Real ExifTool metadata extraction for 70 files under deep Chinese paths, with a batch exceeding Windows' command-line limit. These fixtures deliberately have a CR3 extension but PNG content; the expected classification is PNG. This is not a Canon RAW-format test.
- Installed ExifTool and its runtime resources, Chinese install/source/destination paths, capture-date filtering, H.264 metadata, and a companion XMP file.
- Chinese UI with SHA-256 verification and English/dark UI with verification disabled. An independent test hash comparison checks both copy results; the app must still mark the second task as unverified.
- Existing destination conflict detection, unchanged source bytes, JSON report export, saved settings/task history after restart, and uninstall.
- Windows thread execution-state acquisition/release for idle-sleep prevention. This checks the OS API state, not an actual unattended sleep cycle.

`Framemoor-windows-x64` contains the installer, source ZIP, notices, and SHA-256 values. `windows-native-test-results` contains screenshots, driver diagnostics, and a JSON result with OS/WebView versions. A run with failed native checks does not publish the installer artifact. Build caches contain compilation products only; application scan/metadata/checksum caching remains disabled.

The test tools (`tauri-driver`, Microsoft Edge WebDriver, and FFmpeg) run only in CI and are not installed with the app. The bundled ExifTool Windows package is pinned and checksum-verified. Third-party license fallbacks are tied to specific package versions and upstream sources; dependency upgrades require reviewing them.

## Remaining validation

This does not certify Windows 10/11 hardware, ARM64, every camera model, non-BMP filenames such as some emoji, removable-card disconnect/reconnect, ExFAT/network-volume behavior, power loss, notification delivery, native folder-selection dialogs, or SmartScreen reputation. Windows Server CI coverage must not be described as a physical Windows 10/11 test. The installer is unsigned; there is no signed production release yet.

## 构建与测试范围

`Desktop (Windows x64)` 在 GitHub 的 `windows-latest` 虚拟机上构建未签名的 Release 模式 NSIS 安装包，再将该安装包安装到临时目录，用 `tauri-driver` 和 Microsoft Edge WebDriver 操作真实 WebView2 窗口，最后卸载。测试调用真实 Tauri IPC，不模拟后端命令。素材在临时目录生成，不使用个人照片或视频。

必需的 CI 检查包括：

- Windows/macOS Rust 核心测试、前端单元与浏览器测试，以及两个平台的桌面后端测试。
- 使用真实 ExifTool 读取深层中文路径下的 70 个文件，批量路径总长度超过 Windows 命令行限制。文件故意使用 CR3 扩展名、实际内容为 PNG，必须按内容识别为 PNG；这不代表验证了佳能 RAW 格式。
- 已安装的 ExifTool 及运行依赖、中文安装/来源/目标路径、拍摄日期筛选、H.264 元数据和 XMP 伴随文件。
- 中文界面下启用 SHA-256 导入，英文深色界面下关闭校验导入。测试程序独立比对两次复制结果的哈希，但应用仍须将第二个任务标记为未校验。
- 已有目标文件冲突、来源内容不变、JSON 报告导出、重启后设置和历史保留、卸载。
- Windows 防休眠执行状态的申请与释放。这里验证的是系统 API 状态，没有实际等待一次自动休眠周期。

`Framemoor-windows-x64` 包含安装包、对应源码、声明和 SHA-256；`windows-native-test-results` 包含截图、驱动日志及记录系统/WebView 版本的 JSON 结果。原生测试失败时不提供安装包产物。构建缓存仅保存编译结果，应用仍不使用扫描、元数据或校验缓存。

测试使用的 `tauri-driver`、Microsoft Edge WebDriver 和 FFmpeg 仅在 CI 运行，不随应用安装。ExifTool Windows 包固定版本并核对哈希。补充许可证记录了对应依赖版本和上游来源，更新依赖时需要一并复核。

## 尚待验证

上述测试不代表已验证 Windows 10/11 实机、ARM64、所有相机机型、部分 emoji 等非 BMP 文件名、拔卡重连、ExFAT/网络卷、掉电、系统通知送达、原生文件夹选择对话框或 SmartScreen 信誉。Windows Server CI 测试不能表述为 Windows 10/11 实机测试。当前安装包未签名，尚无签名正式发行版。
