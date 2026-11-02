# Changelog / 更新记录

## 1.0.0

First stable release. The application, desktop package and command-line core now share version 1.0.0. The simple and detailed import workflows remain available.

Changes from 0.3.0:

- Added **RAW (all types)** to video codec selection in both views. The filter includes recognized Canon RAW, Blackmagic RAW, REDCODE RAW, Nikon N-RAW, ARRIRAW, ProRes RAW, Sony X-OCN and Cine RAW; directory templates keep their concrete codec names.
- Added dedicated RAW video format mappings and explicit codec-name recognition. RAW tracks take priority over preview tracks; a conflicting readable name does not override a known non-RAW codec ID on the same track.
- Kept a video group when any associated video matches the codec filter, including same-name RAW clips and proxies.
- Expanded photo RAW classification with 3FR, FFF, IIQ, RWL, SRW, X3F, GPR, MRW, ERF, MEF, MOS, DCR, KDC, K25 and ORI. Added INSP as a non-RAW photo format.
- Documented brand formats and validation limits in Chinese and English. Content-based identification remains authoritative; uncertain video codecs remain `Unknown`, and DNG frames remain photos.

See [1.0.0 release notes](docs/releases/v1.0.0.md) for package information and [media formats](docs/media-formats.md) for the recognition and test scope. These changes do not establish hardware validation for every listed brand.

首个正式版。应用、桌面安装包与命令行核心统一为 1.0.0，继续提供简易导入与详细流程。

相较于 0.3.0 的更新：

- 两个视图的视频编码选择均增加 **RAW（全部类型）**，包含已识别的 Canon RAW、Blackmagic RAW、REDCODE RAW、Nikon N-RAW、ARRIRAW、ProRes RAW、Sony X-OCN 与 Cine RAW；目录模板保留具体编码名称。
- 增加 RAW 视频专用格式映射与明确编码名称识别。RAW 轨道优先于预览轨道；同一轨道明确的非 RAW 编码 ID 不会被矛盾的可读名称覆盖。
- 组内任一已关联视频符合编码筛选时保留整组，包括同名 RAW 片段与代理文件。
- 扩展照片 RAW 分类，增加 3FR、FFF、IIQ、RWL、SRW、X3F、GPR、MRW、ERF、MEF、MOS、DCR、KDC、K25、ORI，并将 INSP 归为普通照片。
- 增加中英文品牌格式与验证范围说明。文件内容识别仍优先，无法确定的视频编码保留 `Unknown`，DNG 逐帧文件继续按照片处理。

安装包信息见 [1.0.0 发行说明](docs/releases/v1.0.0.md)，识别条件与测试范围见[素材格式](docs/media-formats.zh-CN.md)。这些改动不代表所有列出品牌均已完成硬件验证。
