# 泊影 Framemoor

<img src="desktop/src-tauri/icons/app.svg" width="80" alt="泊影图标">

简体中文 · [English](README.md)

从存储卡或文件夹导入照片与视频的桌面工具。选择素材，设置目标目录和文件名，预览后复制，默认使用 SHA-256 校验内容。

**目前为开发预览版（0.2）。** macOS 已使用 Canon EOS R5 Mark II 素材测试。CI 使用合成素材在真实 WebView2 中测试安装后的 Windows x64 应用。Windows 实机、读卡器及其他相机格式仍需验证。

## 下载

[GitHub Releases](https://github.com/wzk112/Framemoor/releases) 提供 macOS Apple Silicon（`.dmg`）与 Windows x64（`-setup.exe`）安装包、对应源码和 SHA-256 校验值。暂不提供 Intel Mac 和 Windows ARM64 安装包。

- **macOS 13+：** 打开 DMG，将应用拖入“应用程序”。应用采用 ad-hoc 签名，没有 Apple Developer ID 签名或公证，首次启动可能被 Gatekeeper 拦截。
- **Windows 10 1903+ / Windows 11：** 运行安装程序。需要 WebView2，缺失时安装程序可联网下载。安装包未签名，可能出现未知发布者或 SmartScreen 提示。

实测环境与限制见[发行打包与验证](docs/release-validation.md)。目前为预览版，请另行保留重要素材的副本。

## 功能

- **来源与筛选：** 选择已挂载存储设备或文件夹，记忆最近路径，按日期、素材类型和视频编码筛选。
- **目录与文件名：** 编辑每个目录层级，分别设置照片和视频命名，点选变量并查看说明。内置模板和自存预设只保存规则，应用预设后清空内容名称。
- **预览：** 显示目标目录树、原路径与目标路径、拍摄时间、可用空间和冲突。修改设置后需重新生成预览。
- **时间处理：** 保留相机时间，或依据记录的 UTC 偏移转换到指定 IANA 时区，支持手动分钟校正。提示未来日期和时区差异，不修改文件内的元数据。
- **导入：** 来源只读，不覆盖已有目标文件，支持暂停、恢复、任务历史和 JSON 报告。已识别的伴随文件与所属素材分组处理。
- **设置：** 中英双语，跟随系统、浅色或深色外观，默认目标目录与预设，完成通知，导入期间防休眠，以及校验并发数。

扫描先列出文件并按扩展名初步分类，再在后台读取元数据。编码筛选与导入预览需等待元数据读取完成。每次扫描重新读取来源，不使用持久化元数据或校验缓存。

## 内容校验

SHA-256 内容校验**默认开启**。任务会读取源文件内容、检查目标内容，并在执行过程中复查源文件与目标文件。可以调整读取并发数；写入和任务日志更新仍按顺序执行。

可在**设置 → 性能**关闭内容校验。关闭后可能提速，实际幅度取决于存储设备，但可靠性会降低：文件大小不变的内容损坏可能无法发现。界面持续显示警告，任务、历史、通知和报告均标注**未校验**。此模式下中断的文件会从头复制。任务始终使用生成计划时选定的校验模式。

目标目录下的 `.card-ingest` 保存任务状态和暂存文件，导入期间请勿移动或修改。文件出现在最终路径，并不代表任务已经完成。macOS 的 ExFAT 目标盘可能需要额外进行一次排他复制来发布文件，增加耗时和暂存空间需求。详情见[实现说明](docs/architecture.md)。

## 从源码运行

需要 Git、Python 3、Node.js 22.12+（22 LTS 或更新的受支持 LTS）、Rust **1.90.0** 以及 [Tauri 平台依赖](https://v2.tauri.app/start/prerequisites/)。macOS 需要 13 或更高版本及 Xcode 命令行工具，Windows 需要 10 1903 或更新版本、包含 Windows SDK 的 MSVC 构建工具与 WebView2。仓库包含工具链配置和依赖锁文件。

```sh
git clone https://github.com/wzk112/Framemoor.git
cd Framemoor/desktop
npm ci
python3 scripts/prepare-resources.py
python3 scripts/prepare-notices.py
npm run tauri dev
```

资源脚本从固定源码版本获取 **ExifTool 13.59**，并保留其许可证，不读取你的素材。也可以通过 `--source` 指定已有的对应版本源码目录。macOS 通过系统 Perl 运行随应用打包的 ExifTool。

在 `desktop` 目录构建 macOS Release 应用与 DMG：

```sh
python3 scripts/prepare-notices.py
npm run tauri build -- --ci --bundles app,dmg
```

产物位于 `src-tauri/target/release/bundle/`，架构与构建机器一致。`prepare-notices.py` 默认使用 Rust 宿主目标；交叉编译时传入 `--target <triple>`。应用包含 ExifTool、依赖声明和源码版本信息。开发构建使用 `--debug --bundles app`。两种构建均采用 ad-hoc 签名，未公证。

Windows x64 在 `desktop` 目录执行：

```powershell
npm ci
python scripts/prepare-resources.py
python scripts/prepare-windows-resources.py
python scripts/prepare-notices.py
npm run tauri dev -- --config src-tauri/tauri.windows.conf.json
# Release 模式 NSIS 安装包：
npm run tauri build -- --bundles nsis --config src-tauri/tauri.windows.conf.json
```

Windows 资源脚本下载完整 ExifTool 13.59 x64 包，并核对固定的 SHA-256，保留配套 Perl 文件和许可证。脚本为 CC0 启动器加入 UTF-8 进程清单，使其能从中文安装目录加载依赖，不修改系统区域设置。`prepare-notices.py` 收集安装包需要的依赖声明。安装包生成在 `desktop/src-tauri/target/release/bundle/nsis/`，按当前用户安装。安装包未签名，Windows 可能显示未知发布者或 SmartScreen 提示。

成功的 [CI 运行](https://github.com/wzk112/Framemoor/actions/workflows/ci.yml)会提供 **Framemoor-macos-ARM64** 和 **Framemoor-windows-x64** 产物，包含安装包、对应源码、声明和 SHA-256。Windows 测试证据保存在 **windows-native-test-results**。CI 产物有保留期限；已发布的下载保存在 GitHub Releases。详情见 [Windows 验证说明](docs/windows-validation.md)。

设置和历史记录保存在系统应用数据目录的 `dev.cardingest.desktop` 下。开发时可用 `FRAMEMOOR_DATA_DIR` 指向独立目录，用 `FRAMEMOOR_EXIFTOOL` 指定元数据工具。旧的 `CARD_INGEST_*` 环境变量继续兼容。素材在本地处理，应用不会上传素材或复制日志。

## 命令行

在仓库根目录执行：

```sh
cargo build --release --locked
# 将 target/release 加入 PATH，或直接调用其中的程序。
framemoor scan --source /path/to/source > selection.json
framemoor plan --source /path/to/source --destination /path/to/destination \
  --manifest selection.json --out import-plan.json
framemoor run --plan import-plan.json --events
framemoor status --plan import-plan.json
framemoor verify --plan import-plan.json
```

生成计划前检查并编辑 `selection.json`，格式见[示例](examples/selection.json)。来源和目标必须是已存在、互不嵌套的目录。`plan` 会对选中源文件计算完整哈希；`run` 也用于恢复同一计划；`status` 读取记录，`verify` 会实际重新读取内容。省略 `--manifest` 时选择全部非隐藏普通文件并保留相对路径。CLI 的扫描用于列出文件，与桌面的元数据流程分开。

## 开发与验证

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cd desktop
npm ci
npm test
npm run build
npx playwright install chromium
npm run test:ui
python3 scripts/prepare-resources.py
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

[自动检查](https://github.com/wzk112/Framemoor/actions)覆盖 macOS/Windows 复制核心、前端与浏览器交互，以及两个平台的桌面后端。Windows 任务还会安装打包后的应用，测试真实界面与 IPC，再卸载应用。浏览器测试使用模拟 IPC 和示例数据，不能代替实际存储设备或原生对话框验证。识别品牌不代表已验证该品牌的所有机型、卡结构或伴随文件关系。硬件拔线、掉电、网络卷、原生文件夹对话框及 Windows 10/11 设备覆盖仍需进一步测试。

所有修改通过 PR 合并。`main` 要求自动检查通过、讨论已解决、提交历史保持线性；禁止直接推送、强推和删除分支，管理员也受限制。贡献方式见 [CONTRIBUTING.md](CONTRIBUTING.md)。

Unicode 路径、文件内容编码和异常测试的边界见[编码与异常覆盖](docs/encoding-validation.md)。

## 许可证

Copyright © 2026 wzk112 与贡献者。泊影原创代码采用 **GPL-3.0-only**，见 [LICENSE](LICENSE)。允许商用和收费分发，但须遵守许可证，包括相应的源码提供义务。第三方依赖沿用各自许可证，见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
