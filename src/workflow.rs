//! Media discovery and a frozen, reviewable import recipe. No source writes.
use crate::{CopySpec, paths, platform};
use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, Duration, FixedOffset, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::UNIX_EPOCH,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaFile {
    pub path: String,
    pub bytes: u64,
    pub modified_ns: u64,
    pub kind: String,
    pub format: String,
    pub codec: String,
    pub camera: String,
    pub captured: Option<String>,
    pub offset: Option<String>,
    pub issues: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaGroup {
    pub id: String,
    pub kind: String,
    pub files: Vec<MediaFile>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Catalog {
    pub id: String,
    pub root: PathBuf,
    pub identity: platform::RootIdentity,
    pub groups: Vec<MediaGroup>,
    pub file_count: usize,
    pub bytes: u64,
    pub cameras: Vec<String>,
    pub codecs: Vec<String>,
    pub profile: String,
}
/// Preliminary counts from file names only. This deliberately has no catalog
/// ID or media metadata and cannot be used to build an import preview.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanInventory {
    pub root: PathBuf,
    pub file_count: usize,
    pub bytes: u64,
    pub photo_groups: usize,
    pub video_groups: usize,
    pub other_groups: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayoutRules {
    pub directory_levels: Vec<String>,
    pub date_format: String,
    pub photo_folder: String,
    pub video_folder: String,
    pub other_folder: String,
    pub photo_name_template: String,
    pub video_name_template: String,
    #[serde(default)]
    pub codec_folders: BTreeMap<String, String>,
}
impl LayoutRules {
    fn templates(&self) -> impl Iterator<Item = &str> {
        self.directory_levels.iter().map(String::as_str).chain([
            self.photo_name_template.as_str(),
            self.video_name_template.as_str(),
        ])
    }
}

// Date patterns are folder components, not arbitrary strftime code or paths.
fn filing_date(date: chrono::NaiveDateTime, pattern: &str) -> Result<String> {
    ensure!(!pattern.trim().is_empty(), "invalid_date_pattern");
    let mut rest = pattern;
    let mut output = String::new();
    while !rest.is_empty() {
        let mut matched = false;
        for (token, format) in [("YYYY", "%Y"), ("MM", "%m"), ("DD", "%d")] {
            if let Some(next) = rest.strip_prefix(token) {
                output.push_str(&date.format(format).to_string());
                rest = next;
                matched = true;
                break;
            }
        }
        if !matched {
            let c = rest.chars().next().unwrap();
            ensure!(['-', '.', '_', ' '].contains(&c), "invalid_date_pattern");
            output.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    ensure!(
        output.chars().any(|c| c.is_ascii_digit()),
        "invalid_date_pattern"
    );
    paths::portable_key(&output).context("invalid_date_pattern")?;
    Ok(output)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Options {
    pub destination: String,
    pub from: String,
    pub to: String,
    pub photos: bool,
    pub videos: bool,
    pub others: bool,
    pub unknown_dates: bool,
    pub codec: String,
    pub time_mode: String,
    pub timezone: String,
    pub shift_minutes: i64,
    pub preset: String,
    pub project: String,
    #[serde(default)]
    pub content: String,
    pub card: String,
    pub directory_template: String,
    pub name_template: String,
    #[serde(default)]
    pub rules: Option<LayoutRules>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreviewRow {
    pub group: String,
    pub kind: String,
    pub source: String,
    pub destination: String,
    pub bytes: u64,
    pub modified_ns: u64,
    pub original_time: String,
    pub resolved_time: String,
    pub codec: String,
    pub issues: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preview {
    pub id: String,
    pub source: PathBuf,
    pub source_identity: platform::RootIdentity,
    pub destination: PathBuf,
    pub destination_identity: platform::RootIdentity,
    pub options: Options,
    pub rows: Vec<PreviewRow>,
    pub groups: usize,
    pub bytes: u64,
    pub excluded_files: usize,
    pub warnings: BTreeMap<String, usize>,
    pub blockers: Vec<String>,
    pub available_bytes: u64,
    pub required_bytes: u64,
}

fn text_tag(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .unwrap_or("")
        .to_string()
}
pub fn describe(path: String, bytes: u64, modified_ns: u64, metadata: &Value) -> MediaFile {
    let ext = Path::new(&path)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_uppercase();
    let format = text_tag(metadata, &["File:FileType"]);
    let format = if format.is_empty() {
        ext.clone()
    } else {
        format
    };
    let kind = if [
        "CR3", "CR2", "CRW", "NEF", "NRW", "ARW", "SR2", "SRF", "RAF", "ORF", "RW2", "PEF", "DNG",
        "JPG", "JPEG", "HEIC", "HEIF", "HIF", "TIFF", "TIF", "PNG",
    ]
    .contains(&format.as_str())
    {
        "photo"
    } else if [
        "MP4", "MOV", "CRM", "MXF", "MTS", "M2TS", "AVI", "BRAW", "R3D", "INSV", "360",
    ]
    .contains(&format.as_str())
    {
        "video"
    } else {
        "other"
    };
    let compressor = text_tag(
        metadata,
        &[
            "Track1:CompressorID",
            "Track2:CompressorID",
            "QuickTime:CompressorID",
        ],
    );
    let codec = if kind == "photo" {
        if ["JPEG", "JPG", "HEIC", "HEIF", "HIF", "TIFF", "TIF", "PNG"].contains(&format.as_str()) {
            format.clone()
        } else {
            "RAW".into()
        }
    } else if format == "CRM" {
        "Canon_RAW".into()
    } else {
        match compressor.to_lowercase().as_str() {
            "hvc1" | "hev1" => "HEVC",
            "avc1" | "avc3" => "H264",
            "apch" | "apcn" | "apcs" | "apco" | "ap4h" | "ap4x" => "ProRes",
            "aprn" | "aprh" => "ProRes_RAW",
            _ => "Unknown",
        }
        .into()
    };
    let camera = text_tag(
        metadata,
        &[
            "IFD0:Model",
            "ExifIFD:Model",
            "QuickTime:Model",
            "Canon:Model",
        ],
    );
    let mut captured = text_tag(
        metadata,
        &[
            "ExifIFD:DateTimeOriginal",
            "EXIF:DateTimeOriginal",
            "QuickTime:DateTimeOriginal",
        ],
    );
    let mut offset = text_tag(
        metadata,
        &["ExifIFD:OffsetTimeOriginal", "EXIF:OffsetTimeOriginal"],
    );
    let mut issues = Vec::new();
    if offset.is_empty() {
        let tz = text_tag(metadata, &["Canon:TimeZone"]);
        if tz.starts_with(['+', '-']) {
            offset = tz;
        }
    }
    if captured.is_empty() && kind == "video" {
        captured = text_tag(metadata, &["QuickTime:CreateDate"]);
        if !captured.is_empty() {
            offset = "+00:00".into();
            issues.push("quicktime_assumed_utc".into());
        }
    }
    if offset.is_empty() {
        offset = captured.get(19..25).unwrap_or("").to_string();
    }
    captured = captured.get(..19).unwrap_or("").to_string();
    if NaiveDateTime::parse_from_str(&captured, "%Y:%m:%d %H:%M:%S").is_err() {
        captured.clear();
    }
    if captured.is_empty() {
        issues.push("missing_date".into());
    }
    if offset.is_empty() && !captured.is_empty() {
        issues.push("missing_offset".into());
    }
    if kind == "video" && codec == "Unknown" {
        issues.push("unknown_codec".into());
    }
    if metadata.get("ExifTool:Error").is_some() || metadata.is_null() {
        issues.push("metadata_unavailable".into());
    }
    MediaFile {
        path,
        bytes,
        modified_ns,
        kind: kind.into(),
        format,
        codec,
        camera,
        captured: (!captured.is_empty()).then_some(captured),
        offset: (!offset.is_empty()).then_some(offset),
        issues,
    }
}

pub fn group_files(files: Vec<MediaFile>) -> Vec<MediaGroup> {
    let key = |f: &MediaFile| {
        Path::new(&f.path)
            .with_extension("")
            .to_string_lossy()
            .to_lowercase()
    };
    let mut groups: BTreeMap<String, MediaGroup> = BTreeMap::new();
    let mut sidecars = Vec::new();
    for mut f in files {
        let extension = Path::new(&f.path)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_uppercase();
        if ["CPF", "LRV", "THM", "XMP", "XML"].contains(&extension.as_str()) {
            f.kind = "other".into();
            f.format = extension;
            sidecars.push(f);
            continue;
        }
        if f.kind == "other" {
            sidecars.push(f);
            continue;
        }
        let id = format!("{}:{}", f.kind, key(&f));
        groups
            .entry(id.clone())
            .or_insert_with(|| MediaGroup {
                id,
                kind: f.kind.clone(),
                files: vec![],
            })
            .files
            .push(f);
    }
    for f in sidecars {
        let expected = match f.format.as_str() {
            "CPF" | "LRV" | "THM" => "video",
            "XMP" => "photo",
            "XML" => "video",
            _ => "",
        };
        let id = format!("{}:{}", expected, key(&f));
        if let Some(group) = groups.get_mut(&id) {
            group.files.push(f);
        } else {
            let id = f.path.clone();
            groups.insert(
                id.clone(),
                MediaGroup {
                    id,
                    kind: "other".into(),
                    files: vec![f],
                },
            );
        }
    }
    // Main media first; companion dates never override the primary asset.
    for group in groups.values_mut() {
        group
            .files
            .sort_by_key(|f| (f.kind == "other", f.path.clone()));
    }
    groups.into_values().collect()
}

// No scan cache. Reader count changes scheduling only, never metadata parsing
// or the validation performed before returning a catalog.
pub const DEFAULT_SCAN_READERS: usize = 4;
pub const MAX_SCAN_READERS: usize = 8;
const SCAN_BATCH: usize = 64;
const FULL_READ_BATCH: usize = 8;

pub fn default_scan_readers() -> usize {
    thread::available_parallelism()
        .map(|n| n.get().min(DEFAULT_SCAN_READERS))
        .unwrap_or(2)
}

#[derive(Clone)]
struct ScanFile {
    source: String,
    path: PathBuf,
    bytes: u64,
    modified_ns: u64,
}

fn scan_metadata(exiftool: &Path, batch: &[ScanFile], fast_cr3: bool) -> Result<Vec<MediaFile>> {
    let mut command = if exiftool.extension().is_some_and(|x| x == "exe") {
        Command::new(exiftool)
    } else {
        let mut c = Command::new(if cfg!(target_os = "macos") {
            "/usr/bin/perl"
        } else {
            "perl"
        });
        c.arg(exiftool);
        c
    };
    command.args([
        "-config",
        "",
        "-json",
        "-G1",
        "-a",
        "-s",
        "-FileType",
        "-Model",
        "-DateTimeOriginal",
        "-OffsetTimeOriginal",
        "-TimeZone",
        "-CompressorID",
        "-QuickTime:CreateDate",
    ]);
    // ExifTool 13.59 -fast skips CR3 media-data scanning, but keeps EXIF and
    // MakerNotes. Do NOT use -fast2: it would discard Canon timezone tags.
    // Other formats keep the full reader, including video metadata trailers.
    if fast_cr3 {
        command.arg("-fast");
    }
    #[cfg(not(windows))]
    for file in batch {
        command.arg(&file.path);
    }
    #[cfg(not(windows))]
    let output = command.output().context("metadata_tool_failed")?;
    #[cfg(windows)]
    let output = {
        use std::{io::Write, os::windows::process::CommandExt, process::Stdio};
        // UTF-8 argument files activate ExifTool's wide-character file I/O and
        // avoid the Windows command-line length limit for batches of long paths.
        let mut arguments = Vec::new();
        for file in batch {
            let path = file.path.to_str().context("Non-Unicode source path")?;
            ensure!(!path.contains(['\r', '\n']), "Invalid source path");
            arguments.extend_from_slice(path.as_bytes());
            arguments.push(b'\n');
        }
        command
            .args(["-charset", "filename=UTF8", "-@", "-"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW: scanning is a background task.
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().context("metadata_tool_failed")?;
        let written = child
            .stdin
            .take()
            .context("metadata_tool_stdin_missing")?
            .write_all(&arguments);
        if let Err(error) = written {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error).context("metadata_tool_input_failed");
        }
        child.wait_with_output().context("metadata_tool_failed")?
    };
    let records: Vec<Value> =
        serde_json::from_slice(&output.stdout).context("metadata_tool_invalid_output")?;
    let records: HashMap<String, Value> = records
        .into_iter()
        .filter_map(|v| {
            v.get("SourceFile")
                .and_then(Value::as_str)
                .map(|p| (p.replace('\\', "/"), v.clone()))
        })
        .collect();
    let mut files = Vec::with_capacity(batch.len());
    for file in batch {
        let record = records.get(&file.path.to_string_lossy().replace('\\', "/"));
        files.push(describe(
            file.source.clone(),
            file.bytes,
            file.modified_ns,
            record.unwrap_or(&Value::Null),
        ));
    }
    // An extension is only a routing hint. Unrecognized or incomplete CR3
    // metadata goes through the original full reader before being reported.
    if fast_cr3 {
        let incomplete: Vec<_> = files
            .iter()
            .zip(batch)
            .filter(|(f, _)| {
                f.format != "CR3"
                    || f.captured.is_none()
                    || f.offset.is_none()
                    || f.camera.is_empty()
                    || f.issues.iter().any(|i| i == "metadata_unavailable")
            })
            .map(|(_, input)| input.clone())
            .collect();
        if !incomplete.is_empty() {
            let replacements: HashMap<_, _> = scan_metadata(exiftool, &incomplete, false)?
                .into_iter()
                .map(|f| (f.path.clone(), f))
                .collect();
            for file in &mut files {
                if let Some(replacement) = replacements.get(&file.path) {
                    *file = replacement.clone();
                }
            }
        }
    }
    Ok(files)
}

pub fn scan(root: &Path, exiftool: &Path, progress: impl FnMut(usize, usize)) -> Result<Catalog> {
    scan_with_readers(root, exiftool, default_scan_readers(), progress)
}

/// Bounded concurrency for benchmarking and callers that know their storage.
/// Every invocation still rereads all source metadata.
pub fn scan_with_readers(
    root: &Path,
    exiftool: &Path,
    readers: usize,
    progress: impl FnMut(usize, usize),
) -> Result<Catalog> {
    scan_observed(root, exiftool, readers, |_| {}, progress)
}

pub fn scan_observed(
    root: &Path,
    exiftool: &Path,
    readers: usize,
    inventory: impl FnOnce(&ScanInventory),
    mut progress: impl FnMut(usize, usize),
) -> Result<Catalog> {
    ensure!(
        (1..=MAX_SCAN_READERS).contains(&readers),
        "invalid_scan_readers"
    );
    let root = fs::canonicalize(root)?;
    let identity = platform::root_identity(&root)?;
    let mount = platform::mount_token(&root)?;
    let specs = crate::discover(&root)?;
    ensure!(!specs.is_empty(), "empty_source");
    ensure!(exiftool.is_file(), "metadata_tool_missing");
    progress(0, specs.len());
    let mut cr3 = Vec::new();
    let mut other = Vec::new();
    for spec in &specs {
        let path = paths::join(&root, &spec.source)?;
        let m = fs::metadata(&path)?;
        let file = ScanFile {
            source: spec.source.clone(),
            bytes: m.len(),
            modified_ns: m.modified()?.duration_since(UNIX_EPOCH)?.as_nanos() as u64,
            path,
        };
        if file
            .path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("cr3"))
        {
            cr3.push(file);
        } else {
            other.push(file);
        }
    }
    let provisional = group_files(
        cr3.iter()
            .chain(&other)
            .map(|f| describe(f.source.clone(), f.bytes, f.modified_ns, &Value::Null))
            .collect(),
    );
    inventory(&ScanInventory {
        root: root.clone(),
        file_count: specs.len(),
        bytes: cr3.iter().chain(&other).map(|f| f.bytes).sum(),
        photo_groups: provisional.iter().filter(|g| g.kind == "photo").count(),
        video_groups: provisional.iter().filter(|g| g.kind == "video").count(),
        other_groups: provisional.iter().filter(|g| g.kind == "other").count(),
    });
    drop(provisional);
    // Full video/unknown reads can take longer than CR3 header reads. Smaller
    // batches scheduled first avoid leaving all videos on one final reader.
    let jobs: Vec<_> = other
        .chunks(FULL_READ_BATCH)
        .map(|b| (false, b))
        .chain(cr3.chunks(SCAN_BATCH).map(|b| (true, b)))
        .collect();
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let mut files = thread::scope(|scope| -> Result<Vec<MediaFile>> {
        let (sender, receiver) = mpsc::channel();
        for _ in 0..readers.min(jobs.len()) {
            let sender = sender.clone();
            let (jobs, next, stop) = (&jobs, &next, &stop);
            scope.spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let Some((fast, batch)) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) else {
                        break;
                    };
                    let result = scan_metadata(exiftool, batch, *fast);
                    if result.is_err() {
                        stop.store(true, Ordering::Relaxed);
                    }
                    if sender.send(result).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);
        let mut files = Vec::with_capacity(specs.len());
        for result in receiver {
            files.extend(result?);
            progress(files.len(), specs.len());
        }
        ensure!(files.len() == specs.len(), "metadata_tool_failed");
        Ok(files)
    })?;
    // Reject changes during the scan, including unplug/replug and directory
    // additions/removals. This is independent of the import's SHA-256 checks.
    ensure!(
        platform::mount_token(&root)? == mount && platform::root_identity(&root)? == identity,
        "source_changed"
    );
    let current = crate::discover(&root)?;
    ensure!(
        current.len() == specs.len()
            && current
                .iter()
                .zip(&specs)
                .all(|(a, b)| a.source == b.source),
        "source_changed"
    );
    for file in &files {
        let m = fs::metadata(paths::join(&root, &file.path)?)?;
        ensure!(
            m.len() == file.bytes
                && m.modified()?.duration_since(UNIX_EPOCH)?.as_nanos() as u64 == file.modified_ns,
            "source_changed"
        );
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    progress(files.len(), specs.len());
    let mut cameras: Vec<_> = files
        .iter()
        .map(|f| f.camera.clone())
        .filter(|s| !s.is_empty())
        .collect();
    cameras.sort();
    cameras.dedup();
    let mut codecs: Vec<_> = files
        .iter()
        .filter(|f| f.kind == "video")
        .map(|f| f.codec.clone())
        .collect();
    codecs.sort();
    codecs.dedup();
    let bytes = files.iter().map(|f| f.bytes).sum();
    let profile = if !cameras.is_empty() && cameras.iter().all(|x| x.contains("R5m2")) {
        "canon_r5m2"
    } else {
        "generic"
    };
    Ok(Catalog {
        id: uuid::Uuid::new_v4().to_string(),
        root,
        identity,
        profile: profile.into(),
        file_count: files.len(),
        bytes,
        cameras,
        codecs,
        groups: group_files(files),
    })
}

pub fn resolve_time(
    file: &MediaFile,
    options: &Options,
    now: DateTime<Utc>,
) -> Result<(Option<NaiveDateTime>, Vec<String>)> {
    let mut issues = file.issues.clone();
    let Some(raw) = file.captured.as_ref() else {
        return Ok((None, issues));
    };
    let wall = NaiveDateTime::parse_from_str(raw, "%Y:%m:%d %H:%M:%S")?;
    let zone: Tz = options
        .timezone
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid_timezone"))?;
    let instant = file
        .offset
        .as_ref()
        .and_then(|v| v.parse::<FixedOffset>().ok())
        .and_then(|o| o.from_local_datetime(&wall).single());
    let mut resolved = wall;
    if let Some(instant) = instant {
        let at_zone = instant.with_timezone(&zone);
        if at_zone.offset().fix().local_minus_utc() != instant.offset().local_minus_utc() {
            issues.push("timezone_mismatch".into());
        }
        if instant.with_timezone(&Utc) + Duration::minutes(options.shift_minutes)
            > now + Duration::minutes(5)
        {
            issues.push("future_date".into());
        }
        if options.time_mode == "location" {
            resolved = at_zone.naive_local();
        }
    } else {
        if options.time_mode == "location" {
            issues.push("cannot_convert_timezone".into());
        }
        if wall + Duration::minutes(options.shift_minutes)
            > now.with_timezone(&zone).naive_local() + Duration::minutes(5)
        {
            issues.push("future_date".into());
        }
    }
    resolved = resolved
        .checked_add_signed(Duration::minutes(options.shift_minutes))
        .context("invalid_time_shift")?;
    issues.sort();
    issues.dedup();
    Ok((Some(resolved), issues))
}
fn safe_component(value: &str) -> String {
    let s: String = value
        .chars()
        .map(|c| {
            if c < ' ' || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let s = s.trim().trim_end_matches('.');
    if s.is_empty() {
        "Unknown".into()
    } else {
        s.to_string()
    }
}
fn render(template: &str, tokens: &BTreeMap<&str, String>) -> Result<String> {
    let mut result = template.to_string();
    for (key, value) in tokens {
        result = result.replace(&format!("{{{key}}}"), value);
    }
    ensure!(!result.contains(['{', '}']), "unknown_template_token");
    Ok(result)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelectionSummary {
    pub files: usize,
    pub groups: usize,
    pub bytes: u64,
    pub dates: Vec<String>,
}
type SelectedGroup<'a> = (&'a MediaGroup, Option<NaiveDateTime>, Vec<String>);
// Shared by the UI count and the frozen import preview. No filesystem access.
fn select_groups<'a>(
    catalog: &'a Catalog,
    options: &Options,
    now: DateTime<Utc>,
) -> Result<(Vec<SelectedGroup<'a>>, Vec<String>)> {
    ensure!(
        ["camera", "location"].contains(&options.time_mode.as_str()),
        "invalid_time_mode"
    );
    options
        .timezone
        .parse::<Tz>()
        .map_err(|_| anyhow::anyhow!("invalid_timezone"))?;
    ensure!(
        (-525600..=525600).contains(&options.shift_minutes),
        "invalid_time_shift"
    );
    for date in [&options.from, &options.to] {
        if !date.is_empty() {
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").context("invalid_date")?;
        }
    }
    ensure!(
        options.from.is_empty() || options.to.is_empty() || options.from <= options.to,
        "invalid_date_range"
    );
    let mut selected = Vec::new();
    let mut dates = std::collections::BTreeSet::new();
    for group in &catalog.groups {
        if !(match group.kind.as_str() {
            "photo" => options.photos,
            "video" => options.videos,
            _ => options.others,
        }) {
            continue;
        }
        let primary = &group.files[0];
        if group.kind == "video" && !options.codec.is_empty() && primary.codec != options.codec {
            continue;
        }
        let (resolved, issues) = resolve_time(primary, options, now)?;
        if let Some(date) = resolved {
            let day = date.format("%Y-%m-%d").to_string();
            dates.insert(day.clone());
            if !options.from.is_empty() && day < options.from
                || !options.to.is_empty() && day > options.to
            {
                continue;
            }
        } else if !options.unknown_dates {
            continue;
        }
        selected.push((group, resolved, issues));
    }
    selected.sort_by_key(|(g, date, _)| (*date, g.id.clone()));
    Ok((selected, dates.into_iter().collect()))
}
pub fn selection_summary(
    catalog: &Catalog,
    options: &Options,
    now: DateTime<Utc>,
) -> Result<SelectionSummary> {
    let (selected, dates) = select_groups(catalog, options, now)?;
    Ok(SelectionSummary {
        files: selected.iter().map(|(g, _, _)| g.files.len()).sum(),
        groups: selected.len(),
        bytes: selected
            .iter()
            .flat_map(|(g, _, _)| &g.files)
            .map(|f| f.bytes)
            .sum(),
        dates,
    })
}

pub fn preview(catalog: &Catalog, options: Options, now: DateTime<Utc>) -> Result<Preview> {
    ensure!(
        ["camera", "location"].contains(&options.time_mode.as_str()),
        "invalid_time_mode"
    );
    ensure!(
        [
            "organized",
            "date_content",
            "camera_archive",
            "original",
            "film",
            "custom"
        ]
        .contains(&options.preset.as_str()),
        "invalid_preset"
    );
    if let Some(rules) = &options.rules {
        filing_date(
            chrono::NaiveDate::from_ymd_opt(2026, 10, 25)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            &rules.date_format,
        )?;
        ensure!(
            rules.directory_levels.len() <= 16,
            "too_many_directory_levels"
        );
        ensure!(
            rules.directory_levels.iter().all(|s| !s.trim().is_empty()),
            "empty_directory_level"
        );
        ensure!(
            !rules.photo_name_template.trim().is_empty()
                && !rules.video_name_template.trim().is_empty(),
            "invalid_name_template"
        );
        if rules.templates().any(|s| s.contains("{content}")) {
            ensure!(!options.content.trim().is_empty(), "content_required");
        }
    } else if options.preset == "date_content"
        || (options.preset == "custom"
            && (options.directory_template.contains("{content}")
                || options.name_template.contains("{content}")))
    {
        ensure!(!options.content.trim().is_empty(), "content_required");
    }
    let destination = fs::canonicalize(&options.destination).context("destination_unavailable")?;
    ensure!(destination.is_dir(), "destination_unavailable");
    let destination_identity = platform::root_identity(&destination)?;
    paths::disjoint(&catalog.identity, &destination_identity)?;
    let (selected, _) = select_groups(catalog, &options, now)?;
    let group_count = selected.len();
    let mut rows = Vec::new();
    let mut warnings = BTreeMap::new();
    let mut blockers = Vec::new();
    let mut keys = HashSet::new();
    for (seq, (group, time, issues)) in selected.into_iter().enumerate() {
        let primary = &group.files[0];
        let mut tokens = BTreeMap::new();
        tokens.insert("project", safe_component(&options.project));
        tokens.insert("content", safe_component(&options.content));
        tokens.insert("card", safe_component(&options.card));
        tokens.insert("camera", safe_component(&primary.camera));
        let type_name = match (group.kind.as_str(), &options.rules) {
            ("photo", Some(r)) => r.photo_folder.as_str(),
            ("video", Some(r)) => r.video_folder.as_str(),
            (_, Some(r)) => r.other_folder.as_str(),
            ("photo", None) => "Photo",
            ("video", None) => "Video",
            (_, None) => "Other",
        };
        tokens.insert("type", type_name.into());
        tokens.insert(
            "date",
            match time {
                Some(t) => {
                    if let Some(r) = &options.rules {
                        filing_date(t, &r.date_format)?
                    } else {
                        t.format("%Y-%m-%d").to_string()
                    }
                }
                None => "Undated".into(),
            },
        );
        tokens.insert(
            "date_compact",
            time.map(|t| t.format("%Y%m%d").to_string())
                .unwrap_or("Undated".into()),
        );
        tokens.insert(
            "time",
            time.map(|t| t.format("%H%M%S").to_string())
                .unwrap_or("000000".into()),
        );
        tokens.insert("seq", format!("{:04}", seq + 1));
        for issue in &issues {
            *warnings.entry(issue.clone()).or_insert(0) += 1;
        }
        for f in &group.files {
            let path = Path::new(&f.path);
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let ext = path.extension().unwrap_or_default().to_string_lossy();
            // Companions stay next to the primary; RAW/JPEG share the same output basename.
            tokens.insert(
                "codec",
                if f.kind == "other" {
                    primary.codec.clone()
                } else {
                    f.codec.clone()
                },
            );
            tokens.insert("original", stem.clone());
            let target = if let Some(rules) = &options.rules {
                tokens.insert(
                    "source_dir",
                    path.parent()
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default(),
                );
                tokens.insert(
                    "format",
                    if f.kind == "other" {
                        primary.format.clone()
                    } else {
                        f.format.clone()
                    },
                );
                if let Some(label) = rules.codec_folders.get(&tokens["codec"]) {
                    tokens.insert("codec", label.clone());
                }
                let mut levels = Vec::new();
                for level in &rules.directory_levels {
                    if level == "{source_dir}" && tokens["source_dir"].is_empty() {
                        continue;
                    }
                    levels.push(render(level, &tokens)?);
                }
                let template = if group.kind == "photo" {
                    &rules.photo_name_template
                } else if group.kind == "video" {
                    &rules.video_name_template
                } else {
                    "{original}"
                };
                let name = render(template, &tokens)?;
                ensure!(
                    !name.is_empty() && !name.contains(['/', '\\']),
                    "invalid_name_template"
                );
                let filename = if ext.is_empty() {
                    name
                } else {
                    format!("{name}.{ext}")
                };
                levels.push(filename);
                levels.join("/")
            } else {
                match options.preset.as_str() {
                    "original" => f.path.clone(),
                    "film" => format!(
                        "{}/Footage/{}/{}/{}/{}",
                        tokens["project"], tokens["date"], tokens["camera"], tokens["card"], f.path
                    ),
                    _ => {
                        let directory = if options.preset == "custom" {
                            options.directory_template.as_str()
                        } else if options.preset == "date_content" {
                            "{date}_{content}/{type}/{codec}"
                        } else {
                            "{project}/{date}/{type}/{codec}"
                        };
                        let naming = if options.preset == "custom" {
                            options.name_template.as_str()
                        } else {
                            "{date_compact}_{time}_{seq}_{original}"
                        };
                        let name = if group.kind == "photo" {
                            render(naming, &tokens)?
                        } else {
                            stem
                        };
                        ensure!(!name.contains(['/', '\\']), "invalid_name_template");
                        format!("{}/{}.{}", render(directory, &tokens)?, name, ext)
                    }
                }
            };
            match paths::portable_key(&target) {
                Ok(key) => {
                    if !keys.insert(key) {
                        blockers.push(format!("collision|{target}"));
                    }
                }
                Err(e) => blockers.push(format!("invalid_path|{target}: {e}")),
            }
            match paths::join(&destination, &target) {
                Ok(p) => {
                    if p.try_exists()? {
                        blockers.push(format!("exists|{target}"));
                    }
                }
                Err(e) => blockers.push(format!("unsafe_path|{e}")),
            }
            rows.push(PreviewRow {
                group: group.id.clone(),
                kind: group.kind.clone(),
                source: f.path.clone(),
                destination: target,
                bytes: f.bytes,
                modified_ns: f.modified_ns,
                original_time: format!(
                    "{} {}",
                    primary.captured.as_deref().unwrap_or("—"),
                    primary.offset.as_deref().unwrap_or("")
                ),
                resolved_time: time
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or("—".into()),
                codec: if f.kind == "other" {
                    primary.codec.clone()
                } else {
                    f.codec.clone()
                },
                issues: issues.clone(),
            });
        }
    }
    for key in &keys {
        let parts: Vec<_> = key.split('/').collect();
        for i in 1..parts.len() {
            if keys.contains(&parts[..i].join("/")) {
                blockers.push(format!("collision|{key}"));
            }
        }
    }
    if rows.is_empty() {
        blockers.push("empty_selection".into());
    }
    let bytes = rows.iter().map(|r| r.bytes).sum::<u64>();
    let largest = rows.iter().map(|r| r.bytes).max().unwrap_or(0);
    let available_bytes = platform::available_space(&destination)?;
    // The exclusive-publication fallback can temporarily retain one staged file.
    let required_bytes = bytes
        .saturating_add(largest)
        .saturating_add(16 * 1024 * 1024);
    if available_bytes < required_bytes {
        blockers.push("insufficient_space".into());
    }
    if catalog.profile == "generic" {
        warnings.insert("generic_profile".into(), 1);
    }
    Ok(Preview {
        id: uuid::Uuid::new_v4().to_string(),
        source: catalog.root.clone(),
        source_identity: catalog.identity.clone(),
        destination,
        destination_identity,
        excluded_files: catalog.file_count - rows.len(),
        options,
        rows,
        groups: group_count,
        bytes,
        warnings,
        blockers,
        available_bytes,
        required_bytes,
    })
}

pub fn validate_snapshot(preview: &Preview) -> Result<Vec<CopySpec>> {
    ensure!(preview.blockers.is_empty(), "preview_blocked");
    ensure!(
        platform::root_identity(&preview.source)? == preview.source_identity,
        "source_volume_changed"
    );
    ensure!(
        platform::root_identity(&preview.destination)? == preview.destination_identity,
        "destination_volume_changed"
    );
    let mut specs = Vec::new();
    for row in &preview.rows {
        let m = fs::metadata(paths::join(&preview.source, &row.source)?)?;
        if m.len() != row.bytes
            || m.modified()?.duration_since(UNIX_EPOCH)?.as_nanos() as u64 != row.modified_ns
        {
            bail!("source_changed|{}", row.source);
        }
        specs.push(CopySpec {
            source: row.source.clone(),
            destination: row.destination.clone(),
            group: row.group.clone(),
        });
    }
    Ok(specs)
}
