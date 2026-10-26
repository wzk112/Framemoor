//! Real Windows ExifTool integration with generated pixels and metadata only.
#![cfg(windows)]
use framemoor_core::workflow;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

#[test]
#[ignore = "requires FRAMEMOOR_TEST_EXIFTOOL pointing to the packaged Windows tool"]
fn unicode_long_paths_and_large_argument_batches_read_real_metadata() {
    let tool = PathBuf::from(
        std::env::var_os("FRAMEMOOR_TEST_EXIFTOOL").expect("Set the Windows ExifTool path"),
    );
    let scratch = tempfile::tempdir().unwrap();
    let seed = scratch.path().join("seed.PNG");
    fs::write(&seed, include_bytes!("fixtures/synthetic.png")).unwrap();
    let mut child = Command::new(&tool)
        .args(["-charset", "filename=UTF8", "-@", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(format!("-overwrite_original\n-Model=Synthetic Camera\n-DateTimeOriginal=2026:10:20 20:00:00\n-OffsetTimeOriginal=+11:00\n{}\n", seed.display()).as_bytes()).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let source = scratch.path().join("中文 source");
    let mut deep = source.clone();
    for index in 0..6 {
        deep.push(format!("目录_{index}_{}", "a".repeat(72)));
    }
    fs::create_dir_all(&deep).unwrap();
    let bytes = fs::read(&seed).unwrap();
    for index in 0..70 {
        // Misleading extension deliberately exercises a full 64-file CR3 batch;
        // the actual content must still be identified as PNG, not guessed as RAW.
        fs::write(deep.join(format!("素材_{index:03}.CR3")), &bytes).unwrap();
    }
    let catalog = workflow::scan_with_readers(&source, &tool, 4, |_, _| {}).unwrap();
    assert_eq!(catalog.file_count, 70);
    for file in catalog.groups.iter().flat_map(|group| &group.files) {
        assert_eq!(file.format, "PNG", "{}: {:?}", file.path, file.issues);
        assert_eq!(file.camera, "Synthetic Camera");
        assert_eq!(file.captured.as_deref(), Some("2026:10:20 20:00:00"));
        assert_eq!(file.offset.as_deref(), Some("+11:00"));
        assert!(
            !file
                .issues
                .iter()
                .any(|issue| issue == "metadata_unavailable")
        );
        assert_eq!(fs::read(source.join(&file.path)).unwrap(), bytes);
    }
}
