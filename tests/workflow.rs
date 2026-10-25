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
