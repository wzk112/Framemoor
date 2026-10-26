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
