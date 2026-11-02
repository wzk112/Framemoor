//! Real packaged ExifTool on each platform, with synthetic pixels only.
use framemoor_core::workflow;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

const CAMERA: &str = "相机 日本語 한국어 Камера";

#[test]
#[ignore = "requires FRAMEMOOR_TEST_EXIFTOOL"]
fn multilingual_metadata_paths_preserve_capture_values() {
    let tool = PathBuf::from(
        std::env::var_os("FRAMEMOOR_TEST_EXIFTOOL").expect("Set packaged ExifTool path"),
    );
    let scratch = tempfile::tempdir().unwrap();
    let seed = scratch.path().join("seed.PNG");
    fs::write(&seed, include_bytes!("fixtures/synthetic.png")).unwrap();
    let mut command = if cfg!(windows) {
        Command::new(&tool)
    } else {
        let mut command = Command::new("/usr/bin/perl");
        command.arg(&tool);
        command
    };
    let mut child = command
        .args(["-charset", "filename=UTF8", "-@", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(format!("-overwrite_original\n-Model={CAMERA}\n-DateTimeOriginal=2026:10:20 20:00:00\n-OffsetTimeOriginal=+11:00\n{}\n", seed.display()).as_bytes()).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let original = fs::read(&seed).unwrap();
    let source = scratch.path().join("中文_日本語_한국어_العربية_📷_𠮷");
    fs::create_dir(&source).unwrap();
    let names = [
        "简体中文",
        "繁體中文",
        "日本語",
        "한국어",
        "العربية",
        "Русский",
        "café",
        "cafe\u{301}",
        "📷",
        "𠮷",
    ];
    for (index, name) in names.iter().enumerate() {
        fs::write(source.join(format!("{index}_{name}.PNG")), &original).unwrap();
    }
    let catalog = workflow::scan_with_readers(&source, &tool, 4, |_, _| {}).unwrap();
    assert_eq!(catalog.file_count, names.len());
    for file in catalog.groups.iter().flat_map(|group| &group.files) {
        assert_eq!(file.camera, CAMERA, "{}: {:?}", file.path, file.issues);
        assert_eq!(file.captured.as_deref(), Some("2026:10:20 20:00:00"));
        assert_eq!(file.offset.as_deref(), Some("+11:00"));
        assert!(
            !file
                .issues
                .iter()
                .any(|issue| issue == "metadata_unavailable")
        );
        println!("Metadata exact: {}", file.path);
        assert_eq!(fs::read(source.join(&file.path)).unwrap(), original);
    }
}

fn qt_atom(tag: &[u8; 4], contents: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&((contents.len() + 8) as u32).to_be_bytes());
    bytes.extend_from_slice(tag);
    bytes.extend_from_slice(contents);
    bytes
}

// Minimal synthetic QuickTime sample descriptions. No manufacturer footage.
fn synthetic_mov(codec: &[u8; 4], compressor_name: &str) -> Vec<u8> {
    let mut sample = vec![0u8; 78];
    sample[6..8].copy_from_slice(&1u16.to_be_bytes());
    sample[24..26].copy_from_slice(&16u16.to_be_bytes());
    sample[26..28].copy_from_slice(&16u16.to_be_bytes());
    sample[40..42].copy_from_slice(&1u16.to_be_bytes());
    let name = compressor_name.as_bytes();
    assert!(name.len() <= 31);
    sample[42] = name.len() as u8;
    sample[43..43 + name.len()].copy_from_slice(name);
    sample[74..76].copy_from_slice(&24u16.to_be_bytes());
    sample[76..78].copy_from_slice(&u16::MAX.to_be_bytes());
    let mut stsd = vec![0u8; 4];
    stsd.extend_from_slice(&1u32.to_be_bytes());
    stsd.extend(qt_atom(codec, &sample));
    let mut handler = vec![0u8; 8];
    handler.extend_from_slice(b"vide");
    handler.extend([0u8; 12]);
    let mut mdia = qt_atom(b"hdlr", &handler);
    mdia.extend(qt_atom(
        b"minf",
        &qt_atom(b"stbl", &qt_atom(b"stsd", &stsd)),
    ));
    let mut movie = qt_atom(b"ftyp", b"qt  \0\0\0\0qt  ");
    movie.extend(qt_atom(
        b"moov",
        &qt_atom(b"trak", &qt_atom(b"mdia", &mdia)),
    ));
    movie
}

#[test]
#[ignore = "requires FRAMEMOOR_TEST_EXIFTOOL"]
fn packaged_reader_preserves_raw_video_track_identifiers_and_content_priority() {
    let tool = PathBuf::from(
        std::env::var_os("FRAMEMOOR_TEST_EXIFTOOL").expect("Set packaged ExifTool path"),
    );
    let source = tempfile::tempdir().unwrap();
    let fixtures = [
        ("blackmagic.braw", *b"brhq", "", "Blackmagic_RAW"),
        ("renamed.jpg", *b"aprh", "", "ProRes_RAW"),
        ("nikon.nev", *b"zzzz", "Nikon N-RAW", "N_RAW"),
        ("ordinary.braw", *b"hvc1", "", "HEVC"),
    ];
    for (name, codec, label, _) in fixtures {
        fs::write(source.path().join(name), synthetic_mov(&codec, label)).unwrap();
    }
    let catalog = workflow::scan(source.path(), &tool, |_, _| {}).unwrap();
    for (name, _, _, expected) in fixtures {
        let file = catalog
            .groups
            .iter()
            .flat_map(|g| &g.files)
            .find(|f| f.path == name)
            .unwrap();
        assert_eq!(file.format, "MOV", "{name}: {file:?}");
        assert_eq!(file.kind, "video", "{name}: {file:?}");
        assert_eq!(file.codec, expected, "{name}: {file:?}");
        assert!(
            !file.issues.contains(&"metadata_unavailable".into()),
            "{name}: {file:?}"
        );
    }
}
