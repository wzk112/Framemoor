//! Paths are Unicode; copied media/sidecar contents remain opaque bytes.
use anyhow::Result;
use framemoor_core::{self as core, Control, CopySpec};
use std::fs;

#[test]
fn multilingual_paths_and_binary_text_encodings_survive_copy_and_json() -> Result<()> {
    let scratch = tempfile::tempdir()?;
    let source = scratch.path().join("来源_日本語_한국어_📷");
    let destination = scratch.path().join("归档_العربية_Русский_𠮷");
    fs::create_dir(&source)?;
    fs::create_dir(&destination)?;
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
    // UTF-8, UTF-8 BOM, UTF-16 LE/BE, GB18030 and arbitrary invalid UTF-8.
    let contents: [&[u8]; 6] = [
        "中文 日本語 📷".as_bytes(),
        b"\xef\xbb\xbfUTF-8",
        b"\xff\xfe\x2d\x4e\x87\x65",
        b"\xfe\xff\x4e\x2d\x65\x87",
        b"\xd6\xd0\xce\xc4",
        b"\xff\x00\x80\xfe",
    ];
    let mut specs = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let relative = format!("{index}_{name}.XMP");
        fs::write(source.join(&relative), contents[index % contents.len()])?;
        specs.push(CopySpec {
            source: relative.clone(),
            destination: format!("照片/{relative}"),
            group: name.to_string(),
        });
    }
    let plan = core::prepare(&source, &destination, specs.clone())?;
    let saved = destination.join("任务_📷.json");
    core::save_plan(&saved, &plan)?;
    let bytes = fs::read(&saved)?;
    assert!(std::str::from_utf8(&bytes)?.contains("العربية"));
    let restored = core::load_plan(&saved)?;
    let report = core::run(&restored, |_| Ok(Control::Continue))?;
    assert!(report.content_verified);
    assert_eq!(report.files, names.len());
    core::verify(&restored)?;
    for (index, spec) in specs.iter().enumerate() {
        let expected = contents[index % contents.len()];
        assert_eq!(fs::read(source.join(&spec.source))?, expected);
        assert_eq!(fs::read(destination.join(&spec.destination))?, expected);
        assert_eq!(restored.entries[index].spec.source, spec.source);
    }
    assert!(core::status(&restored)?.is_some());
    Ok(())
}

#[test]
fn canonically_equivalent_destination_names_are_blocked_before_writing() -> Result<()> {
    let source = tempfile::tempdir()?;
    let destination = tempfile::tempdir()?;
    let mut specs = Vec::new();
    for (index, name) in ["Café.PNG", "Cafe\u{301}.png"].iter().enumerate() {
        let input = format!("{index}.PNG");
        fs::write(source.path().join(&input), b"fixture")?;
        specs.push(CopySpec {
            source: input,
            destination: name.to_string(),
            group: index.to_string(),
        });
    }
    assert!(core::prepare(source.path(), destination.path(), specs).is_err());
    assert_eq!(fs::read_dir(destination.path())?.count(), 0);
    Ok(())
}

#[cfg(unix)]
#[test]
fn non_utf8_filename_is_rejected_without_lossy_substitution() -> Result<()> {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let source = tempfile::tempdir()?;
    let path = source
        .path()
        .join(OsString::from_vec(b"bad_\xff.PNG".to_vec()));
    if let Err(error) = fs::write(&path, b"keep") {
        // APFS itself rejects malformed UTF-8 names with EILSEQ.
        assert!(
            cfg!(target_os = "macos") && error.raw_os_error() == Some(92),
            "{error}"
        );
        assert_eq!(fs::read_dir(source.path())?.count(), 0);
        return Ok(());
    }
    let error = core::discover(source.path()).unwrap_err();
    assert!(error.to_string().contains("Non-Unicode"));
    assert_eq!(fs::read(path)?, b"keep");
    Ok(())
}
