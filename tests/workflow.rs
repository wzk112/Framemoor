use chrono::Utc;
use framemoor_core::{self as core, workflow::*};
use serde_json::json;
use std::{fs, path::Path};
fn options(destination: &Path) -> Options {
    Options {
        destination: destination.to_string_lossy().into(),
        from: String::new(),
        to: String::new(),
        photos: true,
        videos: true,
        others: false,
        unknown_dates: false,
        codec: String::new(),
        time_mode: "camera".into(),
        timezone: "Australia/Melbourne".into(),
        shift_minutes: 0,
        preset: "organized".into(),
        project: "Shoot".into(),
        content: String::new(),
        card: "A001".into(),
        directory_template: "{project}/{date}/{type}/{codec}".into(),
        name_template: "{date_compact}_{time}_{seq}_{original}".into(),
        rules: None,
    }
}
fn media(path: &str, format: &str, date: &str) -> MediaFile {
    describe(
        path.into(),
        4,
        0,
        &json!({"File:FileType":format,"ExifIFD:DateTimeOriginal":date,"ExifIFD:OffsetTimeOriginal":"+10:00","IFD0:Model":"Canon EOS R5m2","Track1:CompressorID":"hvc1","Canon:Quality":"Standard (RAW)"}),
    )
}
fn catalog(root: &Path, files: Vec<MediaFile>) -> Catalog {
    Catalog {
        id: "test".into(),
        root: root.into(),
        identity: core::platform::root_identity(root).unwrap(),
        file_count: files.len(),
        bytes: files.iter().map(|f| f.bytes).sum(),
        groups: group_files(files),
        profile: "canon_r5m2".into(),
        cameras: vec!["Canon EOS R5m2".into()],
        codecs: vec!["HEVC".into()],
    }
}

#[test]
fn canon_container_and_track_are_not_confused() {
    let photo = media("a.CR3", "CR3", "2026:10:06 18:16:37");
    assert_eq!(photo.kind, "photo");
    assert_eq!(photo.codec, "RAW");
    let video = media("a.MP4", "MP4", "2026:10:06 18:16:37");
    assert_eq!(video.codec, "HEVC");
    assert_eq!(media("a.CRM", "CRM", "").codec, "Canon_RAW");
}

#[test]
fn raw_video_formats_keep_concrete_codecs_and_metadata_wins_over_extensions() {
    for (format, codec) in [
        ("CRM", "Canon_RAW"),
        ("BRAW", "Blackmagic_RAW"),
        ("R3D", "RED_RAW"),
        ("NEV", "N_RAW"),
        ("ARI", "ARRIRAW"),
        ("ARX", "ARRIRAW"),
    ] {
        let f = describe("renamed.jpg".into(), 4, 0, &json!({"File:FileType":format}));
        assert_eq!((&*f.kind, &*f.codec), ("video", codec));
        assert!(is_raw_video_codec(&f.codec));
        let hinted = describe(
            format!("clip.{}", format.to_lowercase()),
            4,
            0,
            &serde_json::Value::Null,
        );
        assert_eq!((&*hinted.kind, &*hinted.codec), ("video", codec));
        assert!(hinted.issues.contains(&"metadata_unavailable".into()));
    }
    for (name, metadata, kind, codec) in [
        (
            "fake.braw",
            json!({"File:FileType":"JPEG"}),
            "photo",
            "JPEG",
        ),
        (
            "fake.nev",
            json!({"File:FileType":"MOV","Track1:CompressorID":"hvc1"}),
            "video",
            "HEVC",
        ),
        (
            "fake.crm",
            json!({"File:FileType":"TXT","Track1:CompressorID":"aprn"}),
            "other",
            "Unknown",
        ),
    ] {
        let f = describe(name.into(), 4, 0, &metadata);
        assert_eq!((&*f.kind, &*f.codec), (kind, codec));
    }
}

#[test]
fn raw_track_names_and_mxf_labels_require_explicit_encoding_evidence() {
    for (tag, name, codec) in [
        ("Track1:CompressorName", "Blackmagic RAW", "Blackmagic_RAW"),
        ("Track3:CompressorName", "Nikon N-RAW", "N_RAW"),
        (
            "QuickTime:CompressorName",
            "Apple ProRes RAW HQ",
            "ProRes_RAW",
        ),
        ("MXF:VideoCompressionAlgorithm", "ARRIRAW (HDE)", "ARRIRAW"),
        ("MXF:VideoCompressionAlgorithm", "Sony X-OCN ST", "X_OCN"),
        ("QuickTime:CompressorName", "Phantom Cine RAW", "Cine_RAW"),
    ] {
        let mut metadata = json!({"File:FileType":if tag.starts_with("MXF:") {"MXF"} else {"MOV"}});
        metadata[tag] = json!(name);
        let f = describe("clip.mov".into(), 4, 0, &metadata);
        assert_eq!(f.codec, codec);
        assert!(is_raw_video_codec(&f.codec));
    }
    let raw = describe(
        "external.mov".into(),
        4,
        0,
        &json!({"File:FileType":"MOV","Track1:CompressorID":"mp4a","Track3:CompressorID":"aprh"}),
    );
    assert_eq!(raw.codec, "ProRes_RAW");
    for metadata in [
        json!({"File:FileType":"MXF","IFD0:Model":"Sony VENICE"}),
        json!({"File:FileType":"MXF","IFD0:Model":"ARRI ALEXA"}),
        json!({"File:FileType":"CINE"}),
        json!({"File:FileType":"MOV","Track1:CompressorID":"raw ","Track1:CompressorName":"RAW"}),
        json!({"File:FileType":"MOV","UserData:Title":"Blackmagic RAW"}),
    ] {
        let f = describe("clip.nev".into(), 4, 0, &metadata);
        assert_eq!(f.codec, "Unknown");
        assert!(f.issues.contains(&"unknown_codec".into()));
    }
    let non_raw = describe(
        "clip.braw".into(),
        4,
        0,
        &json!({"File:FileType":"MOV","Track1:CompressorID":"apch","Track1:CompressorName":"ProRes RAW"}),
    );
    assert_eq!(non_raw.codec, "ProRes");
}

#[test]
fn raw_tracks_take_priority_over_preview_tracks_without_trusting_conflicting_names() {
    for (identifier, expected) in [("aprh", "ProRes_RAW"), ("brhq", "Blackmagic_RAW")] {
        let f = describe(
            "clip.mov".into(),
            4,
            0,
            &json!({"File:FileType":"MOV","Track1:CompressorID":"avc1","Track2:CompressorID":identifier}),
        );
        assert_eq!(f.codec, expected);
    }
    let named = describe(
        "clip.mov".into(),
        4,
        0,
        &json!({"File:FileType":"MOV","Track1:CompressorID":"avc1","Track2:CompressorName":"Nikon N-RAW"}),
    );
    assert_eq!(named.codec, "N_RAW");
    for (identifier, expected) in [("avc1", "H264"), ("apch", "ProRes"), ("raw ", "Unknown")] {
        let f = describe(
            "clip.mov".into(),
            4,
            0,
            &json!({"File:FileType":"MOV","Track1:CompressorID":identifier,"Track1:CompressorName":"ProRes RAW"}),
        );
        assert_eq!(f.codec, expected);
    }
}

#[test]
fn still_raw_formats_are_not_raw_video_or_cinemadng_sequences() {
    for format in [
        "CR3", "NEF", "ARW", "DNG", "3FR", "FFF", "IIQ", "RWL", "SRW", "X3F", "GPR", "MRW", "ERF",
        "MEF", "MOS", "DCR", "KDC", "K25", "ORI", "RAW",
    ] {
        let f = describe(
            "frame.mov".into(),
            4,
            0,
            &json!({"File:FileType":format,"Track1:CompressorID":"aprn"}),
        );
        assert_eq!((&*f.kind, &*f.codec), ("photo", "RAW"));
        assert!(!is_raw_video_codec(&f.codec));
    }
    // The generic .raw suffix alone is not enough to distinguish still/video.
    assert_eq!(
        describe("unknown.raw".into(), 4, 0, &serde_json::Value::Null).kind,
        "other"
    );
    let f = describe("frame.dng".into(), 4, 0, &serde_json::Value::Null);
    assert_eq!((&*f.kind, &*f.codec), ("photo", "RAW"));
}

#[test]
fn aggregate_raw_filter_keeps_concrete_folders_and_companions() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let mut files = Vec::new();
    for (name, format) in [
        ("canon.CRM", "CRM"),
        ("blackmagic.BRAW", "BRAW"),
        ("red.R3D", "R3D"),
        ("nikon.NEV", "NEV"),
        ("arri.ARI", "ARI"),
        ("photo.DNG", "DNG"),
        ("regular.MP4", "MP4"),
        ("canon.XML", "XML"),
    ] {
        files.push(media(name, format, "2026:10:06 18:00:00"));
    }
    let mut prores = media("external.MOV", "MOV", "2026:10:06 18:00:00");
    prores.codec = "ProRes_RAW".into();
    files.push(prores);
    let c = catalog(root.path(), files);
    let mut o = options(dest.path());
    o.photos = false;
    o.codec = "RAW".into();
    let p = preview(&c, o.clone(), Utc::now()).unwrap();
    assert_eq!(p.groups, 6);
    assert_eq!(p.rows.len(), 7);
    assert!(
        p.rows
            .iter()
            .all(|r| r.kind == "other" || is_raw_video_codec(&r.codec))
    );
    assert!(
        p.rows
            .iter()
            .any(|r| r.destination.contains("/Video/Blackmagic_RAW/"))
    );
    assert!(
        p.rows
            .iter()
            .any(|r| r.destination.contains("/Video/RED_RAW/"))
    );
    assert!(
        p.rows
            .iter()
            .any(|r| r.source == "canon.XML" && r.destination.contains("/Video/Canon_RAW/"))
    );
    assert_eq!(
        selection_summary(&c, &o, Utc::now()).unwrap().files,
        p.rows.len()
    );
    o.codec = "Blackmagic_RAW".into();
    assert_eq!(
        preview(&c, o.clone(), Utc::now()).unwrap().rows[0].source,
        "blackmagic.BRAW"
    );
    assert_eq!(selection_summary(&c, &o, Utc::now()).unwrap().files, 1);
    o.codec = "RAW".into();
    o.photos = true;
    assert_eq!(selection_summary(&c, &o, Utc::now()).unwrap().files, 8);
    o.videos = false;
    let photos = preview(&c, o, Utc::now()).unwrap();
    assert_eq!(photos.rows.len(), 1);
    assert_eq!(photos.rows[0].source, "photo.DNG");
}
#[test]
fn companions_match_same_folder_and_survive_type_filter() {
    let files = vec![
        media("a/a.CR3", "CR3", ""),
        media("a/a.JPG", "JPEG", ""),
        media("a/a.XMP", "XMP", ""),
        media("b/v.MP4", "MP4", ""),
        media("b/v.THM", "JPEG", ""),
        media("b/v.CPF", "CPF", ""),
        media("c/v.CPF", "CPF", ""),
    ];
    let g = group_files(files);
    assert_eq!(g.len(), 3);
    assert_eq!(g.iter().find(|g| g.kind == "photo").unwrap().files.len(), 3);
    let v = g.iter().find(|g| g.kind == "video").unwrap();
    assert_eq!(v.files[0].format, "MP4");
    assert_eq!(v.files.len(), 3);
}
#[test]
fn dst_is_date_specific_and_changes_only_on_request() {
    let temp = tempfile::tempdir().unwrap();
    let mut o = options(temp.path());
    let now = "2026-10-26T00:00:00Z".parse().unwrap();
    let f = media("a.CR3", "CR3", "2026:10:06 18:16:37");
    let (t, w) = resolve_time(&f, &o, now).unwrap();
    assert_eq!(t.unwrap().to_string(), "2026-10-06 18:16:37");
    assert!(w.contains(&"timezone_mismatch".into()));
    o.time_mode = "location".into();
    assert_eq!(
        resolve_time(&f, &o, now).unwrap().0.unwrap().to_string(),
        "2026-10-06 19:16:37"
    );
    let september = media("b.CR3", "CR3", "2026:09:19 18:16:37");
    assert!(
        !resolve_time(&september, &o, now)
            .unwrap()
            .1
            .contains(&"timezone_mismatch".into())
    );
}
#[test]
fn resolved_midnight_controls_date_selection() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let c = catalog(
        root.path(),
        vec![media("a.CR3", "CR3", "2026:10:06 23:30:00")],
    );
    let mut o = options(dest.path());
    o.from = "2026-10-07".into();
    o.to = o.from.clone();
    assert!(preview(&c, o.clone(), Utc::now()).unwrap().rows.is_empty());
    o.time_mode = "location".into();
    let p = preview(&c, o, Utc::now()).unwrap();
    assert_eq!(p.rows.len(), 1);
    assert!(p.rows[0].destination.contains("2026-10-07"));
    assert_eq!(p.rows[0].original_time, "2026:10:06 23:30:00 +10:00");
}
#[test]
fn unknown_dates_are_explicit_and_original_layout_can_include_everything() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let c = catalog(root.path(), vec![media("DCIM/INDEX.CTG", "CTG", "")]);
    let mut o = options(dest.path());
    o.others = true;
    assert!(preview(&c, o.clone(), Utc::now()).unwrap().rows.is_empty());
    o.unknown_dates = true;
    o.preset = "original".into();
    assert_eq!(
        preview(&c, o, Utc::now()).unwrap().rows[0].destination,
        "DCIM/INDEX.CTG"
    );
}
#[test]
fn collisions_existing_files_and_invalid_tokens_block_import() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let c = catalog(
        root.path(),
        vec![
            media("a.CR3", "CR3", "2026:09:19 18:00:00"),
            media("b.CR3", "CR3", "2026:09:19 18:00:00"),
        ],
    );
    let mut o = options(dest.path());
    o.preset = "custom".into();
    o.name_template = "same".into();
    let p = preview(&c, o.clone(), Utc::now()).unwrap();
    assert!(p.blockers.iter().any(|b| b.starts_with("collision")));
    assert!(validate_snapshot(&p).is_err());
    o.name_template = "{oops}".into();
    assert!(preview(&c, o, Utc::now()).is_err());
    let mut o = options(dest.path());
    o.preset = "original".into();
    fs::write(dest.path().join("a.CR3"), b"exists").unwrap();
    assert!(
        preview(&c, o, Utc::now())
            .unwrap()
            .blockers
            .iter()
            .any(|b| b.starts_with("exists"))
    );
}
#[test]
fn malformed_dates_and_unknown_zones_fail_before_import() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let c = catalog(
        root.path(),
        vec![media("a.CR3", "CR3", "2026:09:19 18:00:00")],
    );
    let mut o = options(dest.path());
    o.timezone = "not/a/zone".into();
    assert!(preview(&c, o.clone(), Utc::now()).is_err());
    o.timezone = "UTC".into();
    o.from = "2026-10-25".into();
    o.to = "2026-09-01".into();
    assert!(preview(&c, o, Utc::now()).is_err());
}
#[test]
fn future_without_offset_requires_review() {
    let dest = tempfile::tempdir().unwrap();
    let o = options(dest.path());
    let mut f = media("a.CR3", "CR3", "2030:10:06 23:30:00");
    f.offset = None;
    assert!(
        resolve_time(&f, &o, "2026-10-26T00:00:00Z".parse().unwrap())
            .unwrap()
            .1
            .contains(&"future_date".into())
    );
}
#[test]
fn changed_source_cannot_be_imported_from_an_old_preview() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let p = root.path().join("a.CR3");
    fs::write(&p, b"test").unwrap();
    let m = fs::metadata(&p).unwrap();
    let mut f = media("a.CR3", "CR3", "2026:09:19 18:00:00");
    f.modified_ns = m
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let c = catalog(root.path(), vec![f]);
    let plan = preview(&c, options(dest.path()), Utc::now()).unwrap();
    assert!(validate_snapshot(&plan).is_ok());
    fs::write(&p, b"changed").unwrap();
    assert!(validate_snapshot(&plan).is_err());
}

#[test]
fn date_and_content_template_keeps_media_categories_and_companions() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let c = catalog(
        root.path(),
        vec![
            media("a.CR3", "CR3", "2026:10:06 18:00:00"),
            media("v.MP4", "MP4", "2026:10:06 18:00:00"),
            media("v.CPF", "CPF", ""),
        ],
    );
    let mut o = options(dest.path());
    o.preset = "date_content".into();
    assert_eq!(
        preview(&c, o.clone(), Utc::now()).unwrap_err().to_string(),
        "content_required"
    );
    o.content = "街拍".into();
    let p = preview(&c, o.clone(), Utc::now()).unwrap();
    assert!(
        p.rows
            .iter()
            .find(|r| r.source == "a.CR3")
            .unwrap()
            .destination
            .starts_with("2026-10-06_街拍/Photo/RAW/")
    );
    assert_eq!(
        p.rows
            .iter()
            .find(|r| r.source == "v.MP4")
            .unwrap()
            .destination,
        "2026-10-06_街拍/Video/HEVC/v.MP4"
    );
    assert_eq!(
        p.rows
            .iter()
            .find(|r| r.source == "v.CPF")
            .unwrap()
            .destination,
        "2026-10-06_街拍/Video/HEVC/v.CPF"
    );
    o.preset = "custom".into();
    o.directory_template = "{date}+{content}/{type}/{codec}".into();
    assert!(
        preview(&c, o, Utc::now()).unwrap().rows[0]
            .destination
            .starts_with("2026-10-06+街拍/")
    );
    let mut old_settings = serde_json::to_value(options(dest.path())).unwrap();
    old_settings.as_object_mut().unwrap().remove("content");
    assert!(
        serde_json::from_value::<Options>(old_settings)
            .unwrap()
            .content
            .is_empty()
    );
}

fn editable_rules() -> LayoutRules {
    LayoutRules {
        directory_levels: vec!["{date} {content}".into(), "{type}".into()],
        date_format: "YYYY.MM.DD".into(),
        photo_folder: "照片".into(),
        video_folder: "视频".into(),
        other_folder: "其他".into(),
        photo_name_template: "{original}".into(),
        video_name_template: "{original}".into(),
        codec_folders: Default::default(),
    }
}
#[test]
fn editable_camera_archive_matches_existing_folders_and_keeps_companions() {
    let src = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let c = catalog(
        src.path(),
        vec![
            media("DCIM/IMG_0001.CR3", "CR3", "2026:10:25 20:08:14"),
            media("DCIM/IMG_0001.JPG", "JPEG", "2026:10:25 20:08:14"),
            media("DCIM/IMG_0001.XMP", "XMP", ""),
            media("DCIM/clip.MP4", "MP4", "2026:10:25 21:00:00"),
            media("DCIM/clip.CPF", "CPF", ""),
        ],
    );
    let mut o = options(dest.path());
    o.preset = "camera_archive".into();
    o.content = "夜景拍摄".into();
    o.rules = Some(editable_rules());
    let p = preview(&c, o.clone(), Utc::now()).unwrap();
    assert!(p.blockers.is_empty());
    let paths: Vec<_> = p.rows.iter().map(|r| r.destination.as_str()).collect();
    for suffix in [
        "照片/IMG_0001.CR3",
        "照片/IMG_0001.JPG",
        "照片/IMG_0001.XMP",
        "视频/clip.MP4",
        "视频/clip.CPF",
    ] {
        assert!(
            paths.contains(&format!("2026.10.25 夜景拍摄/{suffix}").as_str()),
            "{suffix}"
        );
    }
    o.rules.as_mut().unwrap().video_name_template = "{date_compact}_{seq}_{original}".into();
    let p = preview(&c, o, Utc::now()).unwrap();
    let mp4 = p.rows.iter().find(|r| r.source.ends_with("MP4")).unwrap();
    let cpf = p.rows.iter().find(|r| r.source.ends_with("CPF")).unwrap();
    assert_eq!(
        Path::new(&mp4.destination).file_stem(),
        Path::new(&cpf.destination).file_stem()
    );
    assert!(mp4.destination.ends_with("20261025_0002_clip.MP4"));
}
#[test]
fn editable_layers_date_labels_and_names_are_independent_of_preset_id() {
    let src = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let c = catalog(
        src.path(),
        vec![media("DCIM/a.CR3", "CR3", "2026:10:06 18:16:37")],
    );
    let mut o = options(dest.path());
    o.preset = "original".into();
    let mut r = editable_rules();
    r.directory_levels = vec!["{type}".into(), "{date}".into(), "{codec}".into()];
    r.date_format = "DD_MM_YYYY".into();
    r.photo_folder = "原始照片".into();
    r.codec_folders.insert("RAW".into(), "相机原片".into());
    r.photo_name_template = "{original}_{seq}".into();
    o.rules = Some(r);
    let p = preview(&c, o.clone(), Utc::now()).unwrap();
    assert_eq!(
        p.rows[0].destination,
        "原始照片/06_10_2026/相机原片/a_0001.CR3"
    );
    assert_eq!(p.rows[0].codec, "RAW");
    o.rules.as_mut().unwrap().directory_levels = vec![];
    assert_eq!(
        preview(&c, o, Utc::now()).unwrap().rows[0].destination,
        "a_0001.CR3"
    );
}
#[test]
fn editable_original_structure_handles_root_files_and_rejects_unsafe_rules() {
    let src = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let c = catalog(
        src.path(),
        vec![
            media("DCIM/a.CR3", "CR3", "2026:10:06 18:16:37"),
            media("root.CR3", "CR3", "2026:10:06 18:16:37"),
        ],
    );
    let mut o = options(dest.path());
    let mut r = editable_rules();
    r.directory_levels = vec!["{source_dir}".into()];
    o.rules = Some(r);
    let p = preview(&c, o.clone(), Utc::now()).unwrap();
    assert!(p.rows.iter().all(|r| r.source == r.destination));
    for pattern in ["", "YYYY/MM/DD", "%Y-%m-%d", "YYYY-X"] {
        o.rules.as_mut().unwrap().date_format = pattern.into();
        assert!(preview(&c, o.clone(), Utc::now()).is_err());
    }
    o.rules.as_mut().unwrap().date_format = "YYYY.MM.DD".into();
    o.rules.as_mut().unwrap().directory_levels = vec!["../escape".into()];
    assert!(
        !preview(&c, o.clone(), Utc::now())
            .unwrap()
            .blockers
            .is_empty()
    );
    o.rules.as_mut().unwrap().directory_levels = vec![String::new()];
    assert!(preview(&c, o.clone(), Utc::now()).is_err());
    o.rules.as_mut().unwrap().directory_levels = vec!["{date}".into()];
    o.rules.as_mut().unwrap().photo_name_template = "../{original}".into();
    assert!(preview(&c, o, Utc::now()).is_err());
}

#[test]
fn selection_summary_matches_preview_and_resolved_dates_with_companions() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let c = catalog(
        root.path(),
        vec![
            media("a.CR3", "CR3", "2026:10:06 23:30:00"),
            media("a.JPG", "JPEG", "2026:10:06 23:30:00"),
            media("a.XMP", "XMP", ""),
            media("b.MP4", "MP4", "2026:10:07 23:30:00"),
            media("index.CTG", "CTG", ""),
        ],
    );
    let mut o = options(dest.path());
    o.time_mode = "location".into();
    o.from = "2026-10-07".into();
    o.to = o.from.clone();
    let now = Utc::now();
    let summary = selection_summary(&c, &o, now).unwrap();
    assert_eq!(summary.dates, vec!["2026-10-07", "2026-10-08"]);
    assert_eq!(summary.files, 3);
    let p = preview(&c, o.clone(), now).unwrap();
    assert_eq!(
        (summary.files, summary.bytes, summary.groups),
        (p.rows.len(), p.bytes, p.groups)
    );
    o.photos = false;
    assert_eq!(selection_summary(&c, &o, now).unwrap().files, 0);
    o.others = true;
    o.unknown_dates = true;
    assert_eq!(selection_summary(&c, &o, now).unwrap().files, 1);
    // Counts must work before the destination and naming form are complete.
    o.destination.clear();
    o.preset = "date_content".into();
    assert_eq!(selection_summary(&c, &o, now).unwrap().files, 1);
    o.timezone = "Invalid/Zone".into();
    assert!(selection_summary(&c, &o, now).is_err());
}

#[test]
fn insp_images_remain_non_raw_photos() {
    for metadata in [json!({"File:FileType":"INSP"}), serde_json::Value::Null] {
        let f = describe("panorama.insp".into(), 4, 0, &metadata);
        assert_eq!((&*f.kind, &*f.codec), ("photo", "INSP"));
    }
    assert_eq!(media("panorama.insp", "JPEG", "").codec, "JPEG");
}

#[test]
fn raw_selection_finds_media_behind_same_name_video_proxy() {
    let root = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let c = catalog(
        root.path(),
        vec![
            media("take.MP4", "MP4", "2026:10:06 18:00:00"),
            media("take.NEV", "NEV", "2026:10:06 18:00:00"),
            media("take.XML", "XML", ""),
        ],
    );
    assert_eq!(c.groups.len(), 1);
    assert_eq!(c.groups[0].files[0].codec, "HEVC");
    let mut o = options(dest.path());
    o.photos = false;
    for filter in ["RAW", "N_RAW", "HEVC"] {
        o.codec = filter.into();
        let p = preview(&c, o.clone(), Utc::now()).unwrap();
        assert_eq!(p.groups, 1);
        assert_eq!(p.rows.len(), 3);
        assert!(
            p.rows
                .iter()
                .any(|row| row.source == "take.NEV" && row.destination.contains("/N_RAW/"))
        );
        assert!(
            p.rows
                .iter()
                .any(|row| row.source == "take.MP4" && row.destination.contains("/HEVC/"))
        );
    }
}
