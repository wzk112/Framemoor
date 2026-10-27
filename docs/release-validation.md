# Release packaging and validation / 发行打包与验证

## Packages

Version 0.2.0 is a development preview. The release provides macOS Apple Silicon and Windows x64 installers, corresponding source, dependency notices, and SHA-256 checksums. Do not describe these packages as signed/notarized production releases.

| Platform | Package | Requirements | Signing |
| --- | --- | --- | --- |
| macOS Apple Silicon | DMG containing an app and an Applications link | macOS 13+ | Ad-hoc; no Developer ID or notarization |
| Windows x64 | Per-user NSIS installer | Windows 10 1903+ / Windows 11, WebView2 | Unsigned |

The app bundles ExifTool 13.59 and license notices. `notices/SOURCE.txt` identifies the source revision and Rust target. `prepare-notices.py` selects the host target unless `--target` is supplied; fallbacks are version- and checksum-bound to upstream material. The macOS Release profile retains optimization and thin LTO but disables stripping: Rust 1.90 can produce misaligned Mach-O string pools that macOS 27 rejects ([upstream issue](https://github.com/rust-lang/rust/issues/157750)).

## Native checks, 2026-10-27

On macOS 27.2 / arm64, a Release app was copied from a read-only mounted DMG into a temporary installation directory containing Chinese, Japanese and emoji characters. DMG verification, strict recursive code-signature verification, app version/architecture, and bundled ExifTool startup passed. App data was isolated with `FRAMEMOOR_DATA_DIR`; the normal installation, settings and personal media were not used.

The actual WKWebView interface and Tauri backend completed:

- Scanning two synthetic PNGs, one H.264 MP4, and a UTF-16 XMP sidecar; capture dates and H.264 classification appeared in preview.
- Chinese UI import with SHA-256, and English/dark UI import with verification disabled. Each imported four files. Independent hashes matched the source for both tasks, and source bytes remained unchanged. The second task correctly retained `content_verified=false`.
- Chinese/Japanese/Korean/Arabic/Cyrillic, accented/decomposed Latin, emoji and supplementary-CJK names through source, destination, project and report paths.
- Native Save panel export to a Unicode JSON filename, UTF-8 report decoding, and settings/history persistence after restart.

Windows CI installs and exercises the actual Release installer on a Windows Server VM, then uninstalls it. It covers native IPC, Chinese and English interfaces, verified/unverified imports, conflicts, metadata, Unicode paths, report export and restart. See [Windows validation](windows-validation.md) for the environment and boundaries. These checks do not establish physical Windows 10/11 or removable-card compatibility.

## Publishing

1. Merge the release changes through a PR only after all required checks pass.
2. Build/download both installers from the same reviewed commit. Check their recorded source revisions, packaged resources and architectures. Retest the final macOS app before publication.
3. Publish a version tag, both installers, corresponding source, notices and `SHA256SUMS.txt` together. Keep personal test records and absolute user paths out of published evidence.
4. Check the public release assets, sizes and SHA-256 digests after upload. CI artifact retention alone is not a release archive.

The minimum OS versions are build targets, not a claim that every OS version has been tested. Intel Mac, Windows ARM64, Gatekeeper/SmartScreen reputation, IME composition, notification delivery, card removal, power loss, full disks, and network/ExFAT volumes require additional device testing. macOS local tests do not reproduce quarantine behavior after a browser download. No global security settings should be disabled to install the preview.

## 安装包

0.2.0 是开发预览版，提供 macOS Apple Silicon 与 Windows x64 安装包、对应源码、依赖声明和 SHA-256。macOS 需要 13+，DMG 中包含应用及“应用程序”快捷方式，采用 ad-hoc 签名，未使用 Developer ID 签名或公证。Windows 需要 10 1903+ 或 Windows 11 以及 WebView2，NSIS 按当前用户安装，安装包未签名。

应用包含 ExifTool 13.59 与许可证，`notices/SOURCE.txt` 记录源码版本和 Rust 目标。声明脚本默认选择宿主目标，也可传入 `--target`；补充声明固定上游版本和校验值。macOS Release 保留优化与 thin LTO，仅关闭符号裁剪，避免 Rust 1.90 生成 macOS 27 拒绝加载的 Mach-O 文件，见上方上游问题链接。

## 2026-10-27 原生验证

在 macOS 27.2 / arm64 上，只读挂载 DMG，将 Release 应用复制到含中文、日文和 emoji 的临时安装目录。镜像完整性、递归签名、版本/架构及随包 ExifTool 启动检查通过。使用 `FRAMEMOOR_DATA_DIR` 隔离测试设置，未使用正常安装的应用、设置或个人素材。

实际 WKWebView 界面和 Tauri 后端完成：

- 扫描两个合成 PNG、一个 H.264 MP4 和一个 UTF-16 XMP，预览中正确显示拍摄日期与 H.264 分类。
- 中文开启 SHA-256 导入、英文深色关闭校验导入，两次各四个文件。独立哈希比对均一致，来源内容不变；第二个任务仍正确记录 `content_verified=false`。
- 来源、目标、项目和报告路径包含中文、日文、韩文、阿拉伯文、俄文、重音/组合字符、emoji 和扩展区汉字。
- 系统保存面板导出 Unicode 文件名的 JSON 报告，UTF-8 解码正常，重启后设置与历史保留。

Windows CI 在 Windows Server 虚拟机中安装实际 Release 包，测试原生 IPC、中英文界面、校验开关、冲突、元数据、Unicode 路径、报告和重启，再卸载。环境与限制见 [Windows 验证](windows-validation.md)，不等同于 Windows 10/11 实机或读卡器测试。

发布前必须经 PR 和所有必需检查；两个安装包取自同一审查后的提交，复查源码版本、资源与架构，并复测最终 macOS 应用。Release 同时保存版本标签、安装包、源码、声明及 SHA-256，上传后核对公开资产。不得公开个人测试记录和绝对用户路径，也不把有过期时间的 CI 产物当作发行归档。

最低系统版本是构建目标，不代表逐个版本均已实测。Intel Mac、Windows ARM64、Gatekeeper/SmartScreen 信誉、输入法组合输入、通知送达、拔卡、掉电、磁盘耗尽及网络/ExFAT 卷仍需设备验证。本地 macOS 测试未复现浏览器下载后的隔离属性行为；不应为安装预览版关闭全局系统安全设置。
