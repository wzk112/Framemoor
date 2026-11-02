# 素材格式与识别范围

简体中文 · [English](media-formats.md)

泊影按文件内容和元数据分类素材，再按规则复制文件。这里的“识别”指扫描分类、筛选和文件导入，不包括 RAW 解码、视频播放、转码或全景拼接。

**版本：1.0.0。**

## 格式与筛选

| 分类 | 1.0.0 识别范围 |
| --- | --- |
| RAW 照片 | CR3、CR2、CRW、NEF、NRW、ARW、SR2、SRF、RAF、ORF、RW2、PEF、DNG、3FR、FFF、IIQ、RWL、SRW、X3F、GPR、MRW、ERF、MEF、MOS、DCR、KDC、K25、ORI；ExifTool 明确识别为 RAW 的文件 |
| 普通照片 | JPEG/JPG、HEIC/HEIF/HIF、TIFF/TIF、PNG、INSP |
| 视频文件分类 | MP4、MOV、CRM、MXF、MTS、M2TS、AVI、BRAW、R3D、NEV、ARI、ARX、CINE、INSV、360 |
| 视频编码分类 | H264、HEVC、ProRes、Canon_RAW、Blackmagic_RAW、RED_RAW、N_RAW、ARRIRAW、ProRes_RAW、X_OCN、Cine_RAW；识别条件见下表，无法确定时保留 Unknown |
| RAW 视频筛选 | **RAW（全部类型）**同时选中已识别的 RAW 视频编码，也可选择具体编码 |

## 常见品牌与文件

以下列出各品牌的代表格式，可用选项取决于机型、固件、录机或导出软件。照片 RAW 的格式名称以捆绑的 [ExifTool 13.59 文件类型表](https://github.com/exiftool/exiftool/blob/2200871d9cef988051d2a99d67df3bda6cbb30a8/lib/Image/ExifTool.pm)为基础。

| 品牌 | 常见照片格式 | 常见视频文件与说明 |
| --- | --- | --- |
| Canon | CR3、CR2、CRW；JPEG、HEIF | MP4、MOV、MXF；CRM 为 Canon RAW。R5 Mark II 的 CR3、MP4、CRM 分别使用 DCIM、XFVC、CRM 目录。[官方规格](https://www.usa.canon.com/support/p/eos-r5-mark-ii) |
| Nikon | NEF、NRW；JPEG、HEIF | MOV、MP4；部分机型录制 N-RAW（NEV）或 ProRes RAW（MOV）。[Z9 文件类型](https://onlinemanual.nikonimglib.com/z9/en/06_video_recording_01.html) |
| Sony | ARW、SR2、SRF；JPEG、HEIF | 常见为 MP4；专业机型也使用 MXF。XAVC 的 H.264/H.265 与 X-OCN RAW 分开识别。[α7R V 规格](https://helpguide.sony.net/ilc/2230/v1/en/contents/TP1000669431.html)、[X-OCN 白皮书](https://pro.sony/s3/2023/09/07140729/Sony_X-OCN_WhitePaper_v2.0.pdf) |
| Fujifilm | RAF；JPEG、HEIF、TIFF | MOV、MP4；H.264、HEVC、ProRes。外接录机的 RAW 文件按录机实际格式处理。[X-H2 规格](https://fujifilm-dsc.com/en/manual/x-h2/technical_notes/spec/) |
| Panasonic / LUMIX | RW2；JPEG | MP4、MOV；H.264、HEVC、ProRes，取决于机型和录制模式。[S5IIX 录制格式](https://eww.pavc.panasonic.co.jp/dscoi/DC-S5M2X/html/DC-S5M2X_DVQP2992_eng/0033.html) |
| OM SYSTEM / Olympus | ORF；JPEG；ORI | MOV；例如 OM-1 Mark II 使用 H.264 或 HEVC。[官方规格](https://explore.omsystem.com/us/en/om-1-mark-ii-body) |
| Pentax / Ricoh | PEF、DNG；JPEG | MOV，部分旧机型或模式使用 AVI。[K-3 规格](https://www.ricoh-imaging.co.jp/english/products/k-3/spec/) |
| Leica | DNG；JPEG；RWL | MOV、MP4；部分机型提供 ProRes。[SL3 规格](https://leica-camera.com/sites/default/files/pm-14627-Leica-SL3_Technical-Data_en.pdf) |
| Hasselblad | 3FR、FFF；JPEG | H6D 等机型有 H.264 与专用 3FV RAW 视频。**3FV 暂未列入视频分类。**[H6D 手册](https://cdn.hasselblad.com/manuals/h6d/current/H6D_USER-GUIDE_EN.pdf)、[Phocus 手册](https://cdn.hasselblad.com/phocus/Phocus_UserManual_EN.pdf) |
| Phase One | IIQ | 此处覆盖数码后背的照片工作流。[IQ4 文件格式](https://www.phaseone.com/wp-content/uploads/2022/01/XT_Manual_Book_IQ4_2022_Interactive.pdf) |
| Sigma | DNG；JPEG；X3F | fp 的 MOV 与 CinemaDNG；DNG 序列目前逐张作为照片导入。[fp 手册](https://www.sigma-global.com/en/support/download/fp_Manual_FW_Ver.5.0_EN.pdf) |
| DJI | DNG、JPEG | MP4、MOV；部分专业设备提供 ProRes RAW 或 CinemaDNG。普通 ProRes 不属于 RAW。[Inspire 3 规格](https://www.dji.com/inspire-3/specs)、[专业设备格式对照](https://www.dji.com/products/comparison-professional-drones) |
| GoPro | JPEG；GPR | MP4、360；GPR 是照片 RAW，不属于视频 RAW。[GPR 格式](https://gopro.github.io/gpr/)、[MAX 手册](https://gopro.com/content/dam/help/max/manuals/MAX_UM_GB_REVB.pdf) |
| Insta360 | JPEG、DNG、INSP | INSV、MP4。INSP 按普通照片处理；若内容被识别为 JPEG，则使用 JPEG 分类。[官方文件格式说明](https://onlinemanual.insta360.com/x4/en-us/faq/feature/shootingmode) |
| Apple / iPhone | HEIC、JPEG、DNG（ProRAW） | MOV；HEVC、H.264、ProRes；支持的机型和拍摄 App 也可生成 ProRes RAW。ProRAW 照片与 ProRes RAW 视频分开处理。[ProRAW](https://support.apple.com/en-ie/119916)、[ProRes RAW 录制](https://support.apple.com/en-ae/guide/final-cut-camera/dev6b8c9521d/ios) |

文件内容识别优先；单凭 `.raw` 后缀不会认定为照片 RAW。

## RAW 视频与电影机

**RAW（全部类型）**只筛选视频组。组内任一视频符合所选编码，就保留整组，包括已经关联的同名代理文件。照片勾选项独立：勾选照片时，符合日期等条件的照片仍会导入，包括 RAW 照片。目录中的 `{codec}` 继续使用具体编码名称，例如 `Canon_RAW` 或 `Blackmagic_RAW`，不会全部改成一个 `RAW` 文件夹。

| 来源／格式 | 编码名称 | 识别条件与范围 |
| --- | --- | --- |
| Canon RAW / Cinema RAW Light | `Canon_RAW` | CRM 专用格式，或明确的 RAW 编码名称。 |
| Nikon N-RAW | `N_RAW` | NEV 专用格式，或明确的 N-RAW 编码名称。[官方说明](https://onlinemanual.nikonimglib.com/z9/en/raw_video_36.html) |
| Blackmagic RAW | `Blackmagic_RAW` | BRAW 专用格式、`brhq` 编码 ID，或明确的 Blackmagic RAW 编码名称。[官方格式说明](https://www.blackmagicdesign.com/products/blackmagicraw) |
| RED REDCODE RAW | `RED_RAW` | R3D 专用格式，或明确的 REDCODE RAW 编码名称。[RED 文件格式](https://docs.red.com/955-0199/955-0199_V1.2_Rev_A_RED_PS_V-RAPTOR_Operation_Guide/Content/4_Menus/b_ProjSet/FileFormat.htm) |
| ARRI ARRIRAW / HDE | `ARRIRAW` | ARI、ARX，或 MXF 中明确的 ARRIRAW 编码标记。ARI/ARX 是逐帧文件，当前不会合并为片段。[ARRI 格式说明](https://www.arri.com/en/learn/camera-systems/pre-postproduction/file-formats-data-handling/arriraw-faq) |
| Apple ProRes RAW（相机或录机） | `ProRes_RAW` | 明确的 `aprn`/`aprh` 编码标记或 ProRes RAW 编码名称，支持多轨读取。普通 ProRes 422/4444 仍为 `ProRes`。 |
| Sony X-OCN | `X_OCN` | 仅在元数据给出明确 X-OCN 编码名称时归类。不能由 `.mxf` 后缀推断。[Sony 白皮书](https://pro.sony/s3/2023/09/07140729/Sony_X-OCN_WhitePaper_v2.0.pdf) |
| Phantom / Cine RAW | `Cine_RAW` | 仅匹配明确的 Cine RAW 编码名称。CINE 后缀只归为视频，不能据此认定为 RAW。 |
| CinemaDNG | 照片 `RAW` | DNG 逐帧归为照片；不进入 RAW 视频筛选，也不自动拼成视频。 |

## 如何判断识别结果

扫描先用扩展名做临时分类，随后读取随应用打包的 **ExifTool 13.59** 元数据。ExifTool 返回的 `FileType` 优先；没有返回类型时才回退到扩展名。比如 `.braw` 文件若被读取为 MOV，又没有明确 RAW 编码标记，编码会保留为 `Unknown`。

MOV、MP4、MXF 是容器，不能只凭后缀判断 RAW。泊影读取 QuickTime 各视频轨的编码 ID／名称及 MXF 编码字段；无法确定时保留 `Unknown`。`Unknown` 视频仍可通过“全部编码”选择。拍摄日期缺失时，可勾选“包含日期未知的文件”，导入到 `Undated`；应用不会用修改时间冒充拍摄时间。未分类文件可通过“其他文件”选入。

## 验证范围

| 验证方式 | 已有证据 |
| --- | --- |
| macOS 真实素材 | Canon EOS R5 Mark II 素材的扫描与导入。 |
| ExifTool 实际解析 | Blackmagic RAW Speed Test 附带的一个 BRAW profile 样例被识别为 MOV，视频编码 ID 为 `brhq`。范围为该样例的只读分类；Blackmagic 相机导入与硬件测试仍待进行。 |
| 分类与筛选测试 | 合成元数据及 MOV 编码标记样例覆盖照片／RAW 视频名称、容器与编码区分、照片和视频筛选独立、RAW 汇总筛选、同名代理保留及目录名称。NEV 的真实文件读取仍待验证。 |
| Windows 安装版 CI | 在真实 WebView2 中使用合成素材验证界面、IPC 和导入流程；见 [Windows 验证](windows-validation.md)。 |
| 其他品牌与硬件 | 目前以合成元数据测试为主，真实相机文件、读卡器和 Windows 实机仍待验证。 |

需要保留完整拍摄包时，选择保留来源目录和原名，并在预览中检查“其他文件”及日期未知文件是否包含。当前分组采用同目录、同名等规则；跨段视频、双镜头素材、代理文件、独立音频和 sidecar 的完整关系仍需逐机型验证。实现与测试见 [workflow.rs](../src/workflow.rs)、[分类测试](../tests/workflow.rs)及[架构说明](architecture.md)。
