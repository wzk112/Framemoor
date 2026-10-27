# Encoding and failure coverage / 编码与异常覆盖

## Encoding contract

The interface supports Simplified Chinese and English; user-entered names may use other Unicode languages. React/JavaScript uses UTF-16 strings, IPC and saved JSON use UTF-8, and Rust uses Unicode strings and native filesystem APIs. Externally edited settings/plans must be valid UTF-8 JSON without a BOM.

Media and sidecars are copied as bytes, without transcoding. UTF-16 or GB18030 sidecars retain their original bytes; this does not imply parsing every legacy encoding. Metadata interpretation uses the pinned ExifTool version. Incorrectly labelled or damaged metadata cannot be reliably repaired by guessing its encoding.

Regression fixtures cover Simplified/Traditional Chinese, Japanese, Korean, Arabic, Cyrillic, accented/decomposed Latin, emoji and a supplementary CJK character:

- Real filesystem copy, SHA-256, JSON plan reload and unchanged sources on macOS/Windows.
- UTF-8, UTF-8 BOM, UTF-16 LE/BE, GB18030 and arbitrary binary payloads without conversion.
- Native settings creation/replacement, frontend inputs/presets/preview across language changes, and Unicode Windows IPC/import/report/restart paths.
- Real ExifTool extraction on both platforms: every sample, including emoji and supplementary CJK names, must return the exact camera/date/offset. ExifTool [documents broader filename limitations](https://exiftool.org/exiftool_pod2.html#WINDOWS-UNICODE-FILE-NAMES); these passing samples do not establish universal format/encoding support.
- NFC/NFD-equivalent destinations are blocked as collisions. Invalid byte-sequence filenames are rejected by the filesystem or discovery without lossy substitution.

Portable output paths intentionally reject Windows reserved names, forbidden characters, trailing dots/spaces, traversal and relative paths exceeding 230 UTF-16 units. The Windows metadata test covers source roots beyond 260 characters without relaxing output limits.

## Failure coverage and limits

On 2026-10-27, a separate native debug app on macOS 27.2 (Apple silicon) completed a 3-file PNG/UTF-16 XMP import using multilingual, emoji and supplementary-CJK paths. Independent hashes matched, source bytes stayed unchanged, the native save panel exported a UTF-8 report, and English settings/history survived restart. This is a synthetic-fixture native check, not a camera/card or macOS release-installer certification.

A later DMG-installed Release build completed a 4-file check including H.264 video, both verification modes and restart persistence; see [Release validation](release-validation.md). That adds packaged native coverage without extending the hardware claims.

Existing tests cover changed sources, destination conflicts/publication races, corrupt partial/final data, malformed journals, injected copy failures, pause/resume, repeated execution, task locks, invalid templates/dates/timezones, unavailable metadata, future dates, disabled verification and notification permission failure. Synthetic or injected faults do not establish every real-device failure mode.

Remaining manual/device work includes IME composition/candidate selection, right-to-left visual order, Windows 10/11 standard-user permissions/ACL failures, disk exhaustion, ExFAT/SMB volumes, removal/reconnection, actual power loss, antivirus/file-lock interference and notifications. No test suite guarantees every application state; reports must identify the OS and distinguish backend, browser, native and hardware checks.

## 编码约定

界面支持简体中文和英文；用户输入的名称可以使用其他 Unicode 语言。React/JavaScript 使用 UTF-16 字符串，IPC 与保存的 JSON 使用 UTF-8，Rust 使用 Unicode 字符串和系统文件接口。外部编辑的设置/计划须为无 BOM 的有效 UTF-8 JSON。

素材和伴随文件按字节复制，不转码。UTF-16、GB18030 等编码的伴随文件保留原始内容，但这不代表解析所有旧编码。元数据由固定版本的 ExifTool 解释；编码标记错误或已损坏的内容，不能靠猜测可靠修复。

回归素材覆盖简繁中文、日文、韩文、阿拉伯文、俄文、重音及组合字符、emoji 和扩展区汉字：

- macOS/Windows 真实复制、SHA-256、JSON 计划重载和源文件内容不变。
- UTF-8、带 BOM 的 UTF-8、UTF-16 LE/BE、GB18030 和任意二进制内容保持原样。
- 原生设置文件创建/替换，前端输入/预设/预览切换语言后保留文字，以及 Windows 原生 IPC、导入、报告和重启中的 Unicode 路径。
- 两个平台的真实 ExifTool 读取：包括 emoji、扩展区汉字在内的每个样本，都必须准确返回相机名、日期和时区偏移。ExifTool [另有文件名限制说明](https://exiftool.org/exiftool_pod2.html#WINDOWS-UNICODE-FILE-NAMES)；这些样本通过不代表所有格式和编码均受支持。
- NFC/NFD 两种写法的同名目标会被当作冲突；无效字节文件名由文件系统或扫描拒绝，不做有损替换。

跨平台输出规则会拒绝 Windows 保留名、非法字符、末尾空格或点、路径越界，以及超过 230 个 UTF-16 单元的相对路径。Windows 元数据测试覆盖超过 260 字符的来源根目录，没有放宽输出限制。

## 异常覆盖与限制

2026-10-27，在 macOS 27.2（Apple silicon）上用独立标识的原生 Debug 应用完成了 3 个 PNG/UTF-16 XMP 文件的导入，路径包含多语言、emoji 和扩展区汉字。独立哈希比对一致，来源未变；通过系统保存面板导出了 UTF-8 报告，重启后英文设置与历史仍在。这是合成素材的原生检查，不代表相机/存储卡或 macOS Release 安装包认证。

随后从 DMG 安装的 Release 构建完成了含 H.264 视频的四文件测试、两种校验模式及重启持久化，见[Release 验证](release-validation.md)。这补充了安装包的原生验证，没有扩大硬件支持结论。

已有测试覆盖来源变化、目标冲突/发布竞争、暂存或目标损坏、损坏日志、注入复制失败、暂停恢复、重复执行、任务锁、非法模板/日期/时区、元数据不可用、未来时间、关闭校验及通知权限拒绝。合成素材和注入故障不代表所有硬件异常均已实测。

仍需人工或设备验证：输入法组合输入/候选词、从右向左文字的视觉顺序、Windows 10/11 普通用户权限/ACL 拒绝、磁盘耗尽、ExFAT/SMB、拔卡重连、实际掉电、杀毒软件或文件锁干扰，以及系统通知。无法保证穷举所有应用状态；报告须注明系统，区分后端、浏览器、原生和硬件检查。
