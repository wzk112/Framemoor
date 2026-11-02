# Media formats and recognition

[简体中文](media-formats.zh-CN.md) · English

Framemoor classifies media from file content and metadata, then copies files according to your rules. “Recognition” here means scan classification, filtering and file import. It does not include RAW development, video playback, transcoding or panorama stitching.

**Version: 1.0.0.**

## Formats and filters

| Category | Recognition in 1.0.0 |
| --- | --- |
| RAW photos | CR3, CR2, CRW, NEF, NRW, ARW, SR2, SRF, RAF, ORF, RW2, PEF, DNG, 3FR, FFF, IIQ, RWL, SRW, X3F, GPR, MRW, ERF, MEF, MOS, DCR, KDC, K25, ORI; files explicitly identified as RAW by ExifTool |
| Other photos | JPEG/JPG, HEIC/HEIF/HIF, TIFF/TIF, PNG, INSP |
| Video classification | MP4, MOV, CRM, MXF, MTS, M2TS, AVI, BRAW, R3D, NEV, ARI, ARX, CINE, INSV, 360 |
| Video codec classification | H264, HEVC, ProRes, Canon_RAW, Blackmagic_RAW, RED_RAW, N_RAW, ARRIRAW, ProRes_RAW, X_OCN, Cine_RAW; see the conditions below; uncertain codecs remain Unknown |
| RAW video filtering | **RAW (all types)** selects all recognized RAW video codecs; specific codecs are also selectable |

## Common brands and files

These are representative formats for each brand. Available options depend on the model, firmware, recorder or export application. Photo RAW names are based on the bundled [ExifTool 13.59 file-type table](https://github.com/exiftool/exiftool/blob/2200871d9cef988051d2a99d67df3bda6cbb30a8/lib/Image/ExifTool.pm).

| Brand | Common photo formats | Common video files and notes |
| --- | --- | --- |
| Canon | CR3, CR2, CRW; JPEG, HEIF | MP4, MOV, MXF; CRM is Canon RAW. The R5 Mark II uses DCIM, XFVC and CRM directories for its CR3, MP4 and CRM files. [Specifications](https://www.usa.canon.com/support/p/eos-r5-mark-ii) |
| Nikon | NEF, NRW; JPEG, HEIF | MOV, MP4; some models record N-RAW (NEV) or ProRes RAW (MOV). [Z9 file types](https://onlinemanual.nikonimglib.com/z9/en/06_video_recording_01.html) |
| Sony | ARW, SR2, SRF; JPEG, HEIF | Commonly MP4; professional cameras also use MXF. XAVC H.264/H.265 and X-OCN RAW are classified separately. [α7R V specifications](https://helpguide.sony.net/ilc/2230/v1/en/contents/TP1000669431.html), [X-OCN white paper](https://pro.sony/s3/2023/09/07140729/Sony_X-OCN_WhitePaper_v2.0.pdf) |
| Fujifilm | RAF; JPEG, HEIF, TIFF | MOV, MP4; H.264, HEVC, ProRes. External RAW recordings use the recorder's actual file format. [X-H2 specifications](https://fujifilm-dsc.com/en/manual/x-h2/technical_notes/spec/) |
| Panasonic / LUMIX | RW2; JPEG | MP4, MOV; H.264, HEVC, ProRes, depending on the model and recording mode. [S5IIX recording formats](https://eww.pavc.panasonic.co.jp/dscoi/DC-S5M2X/html/DC-S5M2X_DVQP2992_eng/0033.html) |
| OM SYSTEM / Olympus | ORF; JPEG; ORI | MOV; for example, the OM-1 Mark II records H.264 or HEVC. [Specifications](https://explore.omsystem.com/us/en/om-1-mark-ii-body) |
| Pentax / Ricoh | PEF, DNG; JPEG | MOV; some older models or modes use AVI. [K-3 specifications](https://www.ricoh-imaging.co.jp/english/products/k-3/spec/) |
| Leica | DNG; JPEG; RWL | MOV, MP4; some models offer ProRes. [SL3 specifications](https://leica-camera.com/sites/default/files/pm-14627-Leica-SL3_Technical-Data_en.pdf) |
| Hasselblad | 3FR, FFF; JPEG | Models such as the H6D offer H.264 and proprietary 3FV RAW video. **3FV is not currently included in video classification.** [H6D manual](https://cdn.hasselblad.com/manuals/h6d/current/H6D_USER-GUIDE_EN.pdf), [Phocus manual](https://cdn.hasselblad.com/phocus/Phocus_UserManual_EN.pdf) |
| Phase One | IIQ | This entry covers digital-back photo workflows. [IQ4 file formats](https://www.phaseone.com/wp-content/uploads/2022/01/XT_Manual_Book_IQ4_2022_Interactive.pdf) |
| Sigma | DNG; JPEG; X3F | The fp records MOV and CinemaDNG; DNG sequences are currently imported as individual photos. [fp manual](https://www.sigma-global.com/en/support/download/fp_Manual_FW_Ver.5.0_EN.pdf) |
| DJI | DNG, JPEG | MP4, MOV; some professional systems offer ProRes RAW or CinemaDNG. Ordinary ProRes is not RAW. [Inspire 3 specifications](https://www.dji.com/inspire-3/specs), [professional-system formats](https://www.dji.com/products/comparison-professional-drones) |
| GoPro | JPEG; GPR | MP4, 360; GPR is photo RAW, not video RAW. [GPR format](https://gopro.github.io/gpr/), [MAX manual](https://gopro.com/content/dam/help/max/manuals/MAX_UM_GB_REVB.pdf) |
| Insta360 | JPEG, DNG, INSP | INSV, MP4. INSP is treated as a non-RAW photo; if its content is identified as JPEG, JPEG classification is used. [File formats](https://onlinemanual.insta360.com/x4/en-us/faq/feature/shootingmode) |
| Apple / iPhone | HEIC, JPEG, DNG (ProRAW) | MOV; HEVC, H.264, ProRes; supported models and capture apps can also produce ProRes RAW. ProRAW photos and ProRes RAW video are handled separately. [ProRAW](https://support.apple.com/en-ie/119916), [ProRes RAW recording](https://support.apple.com/en-ae/guide/final-cut-camera/dev6b8c9521d/ios) |

Content detection takes priority; a `.raw` extension alone is not classified as a RAW photo.

## RAW video and cinema cameras

**RAW (all types)** filters video groups only. A matching codec on any video in a group retains the entire group, including already-associated proxies with the same name. The Photos checkbox is independent: when enabled, photos matching the date and other criteria remain selected, including RAW photos. The `{codec}` directory variable retains the specific codec name, such as `Canon_RAW` or `Blackmagic_RAW`; it does not merge all of them into a `RAW` folder.

| Source / format | Codec name | Recognition and scope |
| --- | --- | --- |
| Canon RAW / Cinema RAW Light | `Canon_RAW` | Dedicated CRM format or an explicit RAW codec name. |
| Nikon N-RAW | `N_RAW` | Dedicated NEV format or an explicit N-RAW codec name. [Nikon documentation](https://onlinemanual.nikonimglib.com/z9/en/raw_video_36.html) |
| Blackmagic RAW | `Blackmagic_RAW` | Dedicated BRAW format, the `brhq` codec ID, or an explicit Blackmagic RAW codec name. [Format documentation](https://www.blackmagicdesign.com/products/blackmagicraw) |
| RED REDCODE RAW | `RED_RAW` | Dedicated R3D format or an explicit REDCODE RAW codec name. [RED file formats](https://docs.red.com/955-0199/955-0199_V1.2_Rev_A_RED_PS_V-RAPTOR_Operation_Guide/Content/4_Menus/b_ProjSet/FileFormat.htm) |
| ARRI ARRIRAW / HDE | `ARRIRAW` | ARI, ARX, or an explicit ARRIRAW codec marker in MXF. ARI/ARX contain individual frames and are not assembled into clips. [ARRI format documentation](https://www.arri.com/en/learn/camera-systems/pre-postproduction/file-formats-data-handling/arriraw-faq) |
| Apple ProRes RAW (camera or recorder) | `ProRes_RAW` | Explicit `aprn`/`aprh` codec identifiers or ProRes RAW codec names, with multi-track reading. Ordinary ProRes 422/4444 remains `ProRes`. |
| Sony X-OCN | `X_OCN` | Classified only with an explicit X-OCN codec name in metadata. An `.mxf` extension is insufficient. [Sony white paper](https://pro.sony/s3/2023/09/07140729/Sony_X-OCN_WhitePaper_v2.0.pdf) |
| Phantom / Cine RAW | `Cine_RAW` | Matches explicit Cine RAW codec names only. CINE extensions are classified as video, not automatically as RAW. |
| CinemaDNG | Photo `RAW` | Each DNG frame is a photo; excluded from the RAW video filter and not assembled into video. |

## Reading the result

Scanning uses extensions for provisional classification, then reads metadata with bundled **ExifTool 13.59**. ExifTool's `FileType` takes priority; extensions are used only when no type is returned. For example, a `.braw` file identified as MOV without an explicit RAW codec marker remains `Unknown`.

MOV, MP4 and MXF are containers, not evidence of RAW encoding. Framemoor reads QuickTime track codec IDs/names and the MXF codec field; uncertain codecs remain `Unknown`. Unknown video can still be selected with “All codecs.” When capture dates are missing, enable “Include undated files” to import them under `Undated`; modification time is not substituted for capture time. Unclassified files can be included through “Other files.”

## Validation

| Method | Evidence available |
| --- | --- |
| Real media on macOS | Scanning and importing Canon EOS R5 Mark II media. |
| Actual ExifTool parsing | One BRAW profile sample bundled with Blackmagic RAW Speed Test was identified as MOV with video codec ID `brhq`. This was a read-only classification check of that sample; Blackmagic camera import and hardware tests remain to be done. |
| Classification and filter tests | Synthetic metadata and MOV codec-marker fixtures cover photo/RAW video names, container/codec distinctions, independent photo and video selection, combined RAW filtering, same-name proxy retention and directory naming. Reading real NEV files remains to be validated. |
| Installed Windows app in CI | Synthetic media exercises the UI, IPC and import workflow in real WebView2; see [Windows validation](windows-validation.md). |
| Other brands and hardware | Coverage currently relies mainly on synthetic metadata tests. Real camera files, card readers and physical Windows PCs remain to be tested. |

To retain a complete camera package, preserve source directories and original names, then check whether “Other files” and undated files are included in preview. Current grouping uses rules such as matching names within the same directory; complete relationships for spanned clips, dual-lens media, proxies, separate audio and sidecars still need model-specific validation. See [workflow.rs](../src/workflow.rs), [classification tests](../tests/workflow.rs) and [architecture](architecture.md).
