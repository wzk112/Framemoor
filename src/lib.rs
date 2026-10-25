//! A conservative copy engine. Media interpretation is separate from copying.
//! Original files are only opened for reading. SHA-256 verification is enabled
//! by default; explicitly unverified imports retain structural and I/O checks.
pub mod paths;
pub mod platform;
pub mod verification;
pub mod workflow;

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, UNIX_EPOCH},
};
use uuid::Uuid;

const FORMAT_VERSION: u32 = 1;
const BUFFER: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopySpec {
    pub source: String,
    pub destination: String,
    #[serde(default)]
    pub group: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub spec: CopySpec,
    pub bytes: u64,
    pub modified_ns: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    #[serde(
        default = "verification::enabled_by_default",
        skip_serializing_if = "is_true"
    )]
    pub content_verification: bool,
    pub version: u32,
    pub id: UuidString,
    pub source_root: PathBuf,
    pub destination_root: PathBuf,
    pub source_identity: platform::RootIdentity,
    pub destination_identity: platform::RootIdentity,
    pub entries: Vec<Entry>,
}

fn is_true(value: &bool) -> bool {
    *value
}

// Kept as a string in the public JSON; validated before it is ever used as a path.
pub type UuidString = String;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileState {
    Pending,
    Verified,
    ReadyUnverified,
    Publishing,
    Published,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileProgress {
    pub state: FileState,
    pub copied: u64,
    pub publishing_identity: Option<String>,
    pub published_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Journal {
    pub version: u32,
    pub plan_digest: String,
    pub status: String,
    pub files: Vec<FileProgress>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RunReport {
    #[serde(default = "verification::enabled_by_default")]
    pub content_verified: bool,
    #[serde(default)]
    pub skipped_unverified_files: usize,
    pub files: usize,
    pub bytes_written: u64,
    pub resumed_bytes: u64,
    pub skipped_verified_files: usize,
    pub complete_groups: usize,
    pub publication_bytes: u64,
    pub resumed_publication_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Preflight {
        file: usize,
        total_files: usize,
    },
    BeforeWrite {
        file: usize,
        copied: u64,
    },
    Checkpoint {
        file: usize,
        copied: u64,
        total: u64,
    },
    BeforeVerify {
        file: usize,
    },
    BeforePublish {
        file: usize,
    },
    Published {
        file: usize,
    },
    VerificationProgress {
        phase: String,
        file: usize,
        done: usize,
        total: usize,
    },
    PublishingCheckpoint {
        file: usize,
        copied: u64,
        total: u64,
    },
    Complete,
}

#[derive(Clone, Copy, Debug)]
pub enum Control {
    Continue,
    Pause,
}

#[derive(Debug)]
struct Paused;
impl std::fmt::Display for Paused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task paused at a recoverable checkpoint")
    }
}
impl std::error::Error for Paused {}

fn emit(observer: &mut impl FnMut(&Event) -> Result<Control>, event: Event) -> Result<()> {
    match observer(&event)? {
        Control::Continue => Ok(()),
        Control::Pause => Err(Paused.into()),
    }
}

pub fn hash_file(path: &Path) -> Result<String> {
    hash_prefix(path, None)
}

fn hash_prefix(path: &Path, length: Option<u64>) -> Result<String> {
    hash_prefix_cancel(path, length, &AtomicBool::new(false))
}

fn hash_prefix_cancel(path: &Path, length: Option<u64>, stop: &AtomicBool) -> Result<String> {
    platform::reject_link(path)?;
    let mut input = File::open(path).with_context(|| format!("Read failed: {}", path.display()))?;
    ensure!(input.metadata()?.is_file(), "Source must be a regular file");
    let mut hash = Sha256::new();
    let mut buffer = vec![0; BUFFER];
    let mut remaining = length.unwrap_or(u64::MAX);
    while remaining > 0 {
        if stop.load(Ordering::Acquire) {
            return Err(Paused.into());
        }
        let max = buffer.len().min(remaining as usize);
        let read = input.read(&mut buffer[..max])?;
        if read == 0 {
            ensure!(
                length.is_none(),
                "Unexpected end of file during prefix verification"
            );
            break;
        }
        hash.update(&buffer[..read]);
        remaining -= read as u64;
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn snapshot(path: &Path) -> Result<(u64, u64)> {
    platform::reject_link(path)?;
    let m = fs::metadata(path)?;
    ensure!(m.is_file(), "Only regular files are supported");
    Ok((
        m.len(),
        m.modified()?
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .try_into()?,
    ))
}

pub fn discover(root: &Path) -> Result<Vec<CopySpec>> {
    let root = fs::canonicalize(root)?;
    let mut result = Vec::new();
    fn walk(root: &Path, directory: &Path, result: &mut Vec<CopySpec>) -> Result<()> {
        platform::reject_link(directory)?;
        for item in fs::read_dir(directory)? {
            let item = item?;
            let name = item.file_name();
            let name = name
                .to_str()
                .context("Non-Unicode filenames require an explicit mapping")?;
            // System hidden files are excluded from this initial discovery mode.
            if name.starts_with('.') {
                continue;
            }
            platform::reject_link(&item.path())?;
            if item.file_type()?.is_dir() {
                walk(root, &item.path(), result)?;
            } else if item.file_type()?.is_file() {
                let relative = item
                    .path()
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                paths::portable_key(&relative)?;
                result.push(CopySpec {
                    source: relative.clone(),
                    destination: relative.clone(),
                    group: relative,
                });
            } else {
                bail!("Special files are not supported");
            }
        }
        Ok(())
    }
    walk(&root, &root, &mut result)?;
    result.sort_by(|a, b| a.source.cmp(&b.source));
    Ok(result)
}

pub fn prepare(source: &Path, destination: &Path, specs: Vec<CopySpec>) -> Result<Plan> {
    prepare_observed(source, destination, specs, |_, _, _| Ok(Control::Continue))
}

pub fn prepare_observed(
    source: &Path,
    destination: &Path,
    specs: Vec<CopySpec>,
    observer: impl FnMut(usize, usize, &str) -> Result<Control>,
) -> Result<Plan> {
    prepare_with_readers(source, destination, specs, 1, observer)
}

pub fn prepare_with_readers(
    source: &Path,
    destination: &Path,
    specs: Vec<CopySpec>,
    readers: usize,
    observer: impl FnMut(usize, usize, &str) -> Result<Control>,
) -> Result<Plan> {
    prepare_with_settings(
        source,
        destination,
        specs,
        verification::Settings {
            enabled: true,
            readers: Some(readers),
        },
        observer,
    )
}

pub fn prepare_with_settings(
    source: &Path,
    destination: &Path,
    specs: Vec<CopySpec>,
    settings: verification::Settings,
    mut observer: impl FnMut(usize, usize, &str) -> Result<Control>,
) -> Result<Plan> {
    settings.validate()?;
    let readers = settings.resolve(source, destination)?;
    let source = fs::canonicalize(source).context("Source folder is unavailable")?;
    let destination =
        fs::canonicalize(destination).context("Choose an existing destination folder")?;
    ensure!(
        source.is_dir() && destination.is_dir(),
        "Both roots must be directories"
    );
    let source_identity = platform::root_identity(&source)?;
    let destination_identity = platform::root_identity(&destination)?;
    paths::disjoint(&source_identity, &destination_identity)?;
    let mut keys = HashSet::new();
    let mut sources = HashSet::new();
    let mut checked = Vec::new();
    let total = specs.len();
    for mut spec in specs {
        ensure!(
            keys.insert(paths::portable_key(&spec.destination)?),
            "Destination naming collision"
        );
        ensure!(
            sources.insert(paths::portable_key(&spec.source)?),
            "Source selected more than once"
        );
        paths::join(&source, &spec.source)?;
        let dst = paths::join(&destination, &spec.destination)?;
        ensure!(
            !dst.try_exists()?,
            "Destination already exists: {}",
            dst.display()
        );
        if spec.group.is_empty() {
            spec.group = spec.source.clone();
        }
        checked.push(spec);
    }
    let source_mount = platform::mount_token(&source)?;
    let destination_mount = platform::mount_token(&destination)?;
    let entries = verification::map(
        total,
        readers,
        |i, stop| {
            let spec = checked[i].clone();
            let src = paths::join(&source, &spec.source)?;
            let before = snapshot(&src)?;
            let sha256 = if settings.enabled {
                hash_prefix_cancel(&src, None, stop)?
            } else {
                String::new()
            };
            ensure!(snapshot(&src)? == before, "Source changed during planning");
            Ok(Entry {
                spec,
                bytes: before.0,
                modified_ns: before.1,
                sha256,
            })
        },
        |i, _| observer(i, total, &checked[i].source),
    )?;
    ensure!(!entries.is_empty(), "No files selected");
    let plan = Plan {
        content_verification: settings.enabled,
        version: FORMAT_VERSION,
        id: Uuid::new_v4().to_string(),
        source_root: source,
        destination_root: destination,
        source_identity,
        destination_identity,
        entries,
    };
    validate(&plan)?;
    check_mounts(&plan, source_mount, destination_mount)?;
    check_roots(&plan)?;
    Ok(plan)
}

fn validate(plan: &Plan) -> Result<()> {
    ensure!(plan.version == FORMAT_VERSION, "Unsupported plan version");
    Uuid::parse_str(&plan.id).context("Invalid task ID")?;
    ensure!(
        plan.id == Uuid::parse_str(&plan.id)?.to_string(),
        "Non-canonical task ID"
    );
    ensure!(
        plan.source_root.is_absolute() && plan.destination_root.is_absolute(),
        "Absolute roots are required"
    );
    ensure!(!plan.entries.is_empty(), "Empty plan");
    paths::disjoint(&plan.source_identity, &plan.destination_identity)?;
    let mut destinations = HashSet::new();
    let mut sources = HashSet::new();
    let mut total = 0u64;
    for e in &plan.entries {
        ensure!(
            destinations.insert(paths::portable_key(&e.spec.destination)?),
            "Destination naming collision"
        );
        ensure!(
            sources.insert(paths::portable_key(&e.spec.source)?),
            "Duplicate source"
        );
        ensure!(
            if plan.content_verification {
                e.sha256.len() == 64 && e.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            } else {
                e.sha256.is_empty()
            },
            "Invalid SHA-256"
        );
        total = total
            .checked_add(e.bytes)
            .context("Total byte count overflow")?;
    }
    // Reject file/directory conflicts before any copying (e.g. x and x/y).
    for key in &destinations {
        let mut prefix = String::new();
        let parts: Vec<_> = key.split('/').collect();
        for part in &parts[..parts.len() - 1] {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            ensure!(
                !destinations.contains(&prefix),
                "File/directory destination collision"
            );
        }
    }
    Ok(())
}

fn check_roots(plan: &Plan) -> Result<()> {
    ensure!(
        fs::canonicalize(&plan.source_root)? == plan.source_root
            && fs::canonicalize(&plan.destination_root)? == plan.destination_root,
        "A root path now resolves to a different location"
    );
    ensure!(
        platform::root_identity(&plan.source_root)? == plan.source_identity,
        "Source volume or folder identity changed"
    );
    ensure!(
        platform::root_identity(&plan.destination_root)? == plan.destination_identity,
        "Destination volume or folder identity changed"
    );
    Ok(())
}

fn check_mounts(plan: &Plan, source: u64, destination: u64) -> Result<()> {
    ensure!(
        fs::canonicalize(&plan.source_root)? == plan.source_root
            && fs::canonicalize(&plan.destination_root)? == plan.destination_root,
        "A root path was redirected during the task"
    );
    ensure!(
        platform::mount_token(&plan.source_root)? == source
            && platform::mount_token(&plan.destination_root)? == destination,
        "A mounted volume changed during this run; reconnect and resume"
    );
    Ok(())
}

fn digest<T: Serialize>(value: &T) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

fn save_json<T: Serialize>(path: &Path, value: &T, replace: bool) -> Result<()> {
    platform::reject_link(path)?;
    let parent = path.parent().context("Missing parent folder")?;
    let tmp = parent.join(format!("journal-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut f = OpenOptions::new().create_new(true).write(true).open(&tmp)?;
        serde_json::to_writer_pretty(&mut f, value)?;
        f.write_all(b"\n")?;
        platform::sync_file(&f)?;
        drop(f);
        if replace {
            platform::replace_journal(&tmp, path)?;
        } else if let Err(error) = platform::publish(&tmp, path) {
            if !platform::unsupported(&error) {
                return Err(error);
            }
            // Immutable JSON on a filesystem without exclusive rename. A crash
            // can leave invalid JSON, which will fail closed when loaded.
            let mut target = OpenOptions::new().create_new(true).write(true).open(path)?;
            std::io::copy(&mut File::open(&tmp)?, &mut target)?;
            platform::sync_file(&target)?;
        }
        platform::sync_directory(parent)
    })();
    if tmp.exists() {
        let _ = fs::remove_file(tmp);
    }
    result
}

pub fn save_plan(path: &Path, plan: &Plan) -> Result<()> {
    validate(plan)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let path = fs::canonicalize(parent)?.join(path.file_name().context("Missing plan filename")?);
    ensure!(
        !path.starts_with(&plan.source_root),
        "Do not store a plan on the selected source"
    );
    save_json(&path, plan, false)
}
pub fn load_plan(path: &Path) -> Result<Plan> {
    let plan = serde_json::from_reader(File::open(path)?)?;
    validate(&plan)?;
    Ok(plan)
}

fn task_directory(plan: &Plan) -> Result<PathBuf> {
    validate(plan)?;
    let parent = plan.destination_root.join(".card-ingest");
    platform::reject_link(&parent)?;
    let job = parent.join(&plan.id);
    platform::reject_link(&job)?;
    Ok(job)
}

pub fn status(plan: &Plan) -> Result<Option<Journal>> {
    let path = task_directory(plan)?.join("journal.json");
    platform::reject_link(&path)?;
    if !path.try_exists()? {
        return Ok(None);
    }
    let state: Journal = serde_json::from_reader(File::open(path)?)?;
    ensure!(
        state.version == FORMAT_VERSION
            && state.plan_digest == digest(plan)?
            && state.files.len() == plan.entries.len(),
        "Task journal does not match the approved plan"
    );
    Ok(Some(state))
}

fn validate_source(plan: &Plan, entry: &Entry) -> Result<()> {
    validate_source_cancel(plan, entry, &AtomicBool::new(false))
}

fn validate_source_cancel(plan: &Plan, entry: &Entry, stop: &AtomicBool) -> Result<()> {
    let src = paths::join(&plan.source_root, &entry.spec.source)?;
    ensure!(
        if plan.content_verification {
            snapshot(&src)?.0 == entry.bytes
                && hash_prefix_cancel(&src, None, stop)? == entry.sha256
        } else {
            snapshot(&src)? == (entry.bytes, entry.modified_ns)
        },
        "Source changed: {}",
        entry.spec.source
    );
    Ok(())
}

/// Read-only byte verification of all source/destination pairs, independent of journal state.
pub fn verify(plan: &Plan) -> Result<RunReport> {
    verify_with_readers(plan, 1)
}

pub fn verify_with_readers(plan: &Plan, readers: usize) -> Result<RunReport> {
    verification::Settings {
        readers: Some(readers),
        enabled: true,
    }
    .validate()?;
    validate(plan)?;
    ensure!(
        plan.content_verification,
        "This plan has no content checksums; it cannot produce a verified report"
    );
    check_roots(plan)?;
    let source_mount = platform::mount_token(&plan.source_root)?;
    let destination_mount = platform::mount_token(&plan.destination_root)?;
    verification::map(
        plan.entries.len(),
        readers,
        |i, stop| {
            let entry = &plan.entries[i];
            check_mounts(plan, source_mount, destination_mount)?;
            validate_source_cancel(plan, entry, stop)?;
            let dst = paths::join(&plan.destination_root, &entry.spec.destination)?;
            ensure!(
                snapshot(&dst)?.0 == entry.bytes
                    && (!plan.content_verification
                        || hash_prefix_cancel(&dst, None, stop)? == entry.sha256),
                "Destination verification failed: {}",
                entry.spec.destination
            );
            Ok(())
        },
        |_, _| Ok(Control::Continue),
    )?;
    check_mounts(plan, source_mount, destination_mount)?;
    Ok(RunReport {
        content_verified: true,
        files: plan.entries.len(),
        skipped_verified_files: plan.entries.len(),
        complete_groups: plan
            .entries
            .iter()
            .map(|e| &e.spec.group)
            .collect::<HashSet<_>>()
            .len(),
        ..Default::default()
    })
}

pub fn run(plan: &Plan, observer: impl FnMut(&Event) -> Result<Control>) -> Result<RunReport> {
    run_with_readers(plan, 1, observer)
}

pub fn run_with_readers(
    plan: &Plan,
    readers: usize,
    mut observer: impl FnMut(&Event) -> Result<Control>,
) -> Result<RunReport> {
    verification::Settings {
        readers: Some(readers),
        enabled: true,
    }
    .validate()?;
    validate(plan)?;
    check_roots(plan)?;
    let source_mount = platform::mount_token(&plan.source_root)?;
    let destination_mount = platform::mount_token(&plan.destination_root)?;
    let job = task_directory(plan)?;
    let parent = job.parent().unwrap();
    fs::create_dir_all(parent)?;
    let lock_path = parent.join("destination.lock");
    platform::reject_link(&lock_path)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.try_lock()
        .context("Another import is using this destination")?;
    fs::create_dir_all(&job)?;
    platform::sync_directory(parent)?;
    let journal_path = job.join("journal.json");
    let approved_path = job.join("approved-plan.json");
    platform::reject_link(&approved_path)?;
    if approved_path.try_exists()? {
        ensure!(
            digest(&load_plan(&approved_path)?)? == digest(plan)?,
            "An existing task has a different plan"
        );
    } else {
        save_json(&approved_path, plan, false)?;
    }
    let mut state = status(plan)?.unwrap_or(Journal {
        version: FORMAT_VERSION,
        plan_digest: digest(plan)?,
        status: "planned".into(),
        files: vec![
            FileProgress {
                state: FileState::Pending,
                copied: 0,
                publishing_identity: None,
                published_bytes: 0
            };
            plan.entries.len()
        ],
        last_error: None,
    });
    save_json(&journal_path, &state, true)?;
    let result = (|| {
        // Read-only preflight runs in parallel; journal and publication remain serial.
        verification::map(
            plan.entries.len(),
            readers,
            |i, stop| {
                check_mounts(plan, source_mount, destination_mount)?;
                validate_source_cancel(plan, &plan.entries[i], stop)
            },
            |i, done| {
                observer(&Event::VerificationProgress {
                    phase: "checking".into(),
                    file: i,
                    done,
                    total: plan.entries.len(),
                })
            },
        )?;
        let mut needed = 0u64;
        let mut publication_reserve = 0u64;
        for (i, entry) in plan.entries.iter().enumerate() {
            check_mounts(plan, source_mount, destination_mount)?;
            emit(
                &mut observer,
                Event::Preflight {
                    file: i,
                    total_files: plan.entries.len(),
                },
            )?;
            let dst = paths::join(&plan.destination_root, &entry.spec.destination)?;
            if dst.try_exists()? {
                ensure!(
                    state.files[i].state != FileState::Pending,
                    "Destination conflict: {}",
                    entry.spec.destination
                );
                if state.files[i].state == FileState::Publishing {
                    validate_publication(
                        &dst,
                        entry,
                        &state.files[i],
                        &job.join(format!("{i:08}.part")),
                        plan.content_verification,
                    )?;
                    publication_reserve = publication_reserve.max(if plan.content_verification {
                        entry.bytes - snapshot(&dst)?.0
                    } else {
                        entry.bytes
                    });
                } else {
                    ensure!(
                        snapshot(&dst)?.0 == entry.bytes
                            && (!plan.content_verification || hash_file(&dst)? == entry.sha256),
                        "Previously published destination has changed"
                    );
                }
            } else {
                let part = job.join(format!("{i:08}.part"));
                platform::reject_link(&part)?;
                let length = if part.try_exists()? && plan.content_verification {
                    snapshot(&part)?.0
                } else {
                    0
                };
                ensure!(
                    length <= entry.bytes,
                    "Partial file is larger than its source"
                );
                needed = needed
                    .checked_add(entry.bytes - length)
                    .context("Byte count overflow")?;
                if plan
                    .destination_identity
                    .filesystem
                    .eq_ignore_ascii_case("exfat")
                {
                    publication_reserve = publication_reserve.max(entry.bytes);
                }
            }
        }
        ensure!(
            platform::available_space(&plan.destination_root)?
                >= needed
                    .saturating_add(publication_reserve)
                    .saturating_add(16 * 1024 * 1024),
            "Not enough free destination space (including safe publication staging)"
        );
        state.status = "running".into();
        state.last_error = None;
        save_json(&journal_path, &state, true)?;
        let mut report = RunReport {
            files: plan.entries.len(),
            ..Default::default()
        };
        let mut buffer = vec![0; BUFFER];
        for (i, entry) in plan.entries.iter().enumerate() {
            check_mounts(plan, source_mount, destination_mount)?;
            let src = paths::join(&plan.source_root, &entry.spec.source)?;
            let dst = paths::join(&plan.destination_root, &entry.spec.destination)?;
            let part = job.join(format!("{i:08}.part"));
            platform::reject_link(&part)?;
            if dst.try_exists()? {
                if state.files[i].state == FileState::Publishing {
                    publish_exclusively(
                        plan,
                        i,
                        &part,
                        &dst,
                        &journal_path,
                        &mut state,
                        &mut observer,
                        &mut report,
                        source_mount,
                        destination_mount,
                    )?;
                    state.files[i].state = FileState::Published;
                    save_json(&journal_path, &state, true)?;
                    fs::remove_file(&part)?;
                    platform::sync_directory(&job)?;
                    continue;
                }
                // Recheck just before adopting the existing file, not only in preflight.
                ensure!(
                    state.files[i].state != FileState::Pending
                        && snapshot(&dst)?.0 == entry.bytes
                        && (!plan.content_verification || hash_file(&dst)? == entry.sha256),
                    "Destination conflict or changed content"
                );
                state.files[i].state = FileState::Published;
                state.files[i].copied = entry.bytes;
                if plan.content_verification {
                    report.skipped_verified_files += 1;
                } else {
                    report.skipped_unverified_files += 1;
                }
                save_json(&journal_path, &state, true)?;
                continue;
            }
            let offset = if part.try_exists()? && plan.content_verification {
                snapshot(&part)?.0
            } else {
                0
            };
            ensure!(
                offset <= entry.bytes,
                "Partial file exceeds expected length"
            );
            if offset > 0 {
                ensure!(
                    hash_file(&part)? == hash_prefix(&src, Some(offset))?,
                    "Partial copy was modified; refusing to append"
                );
            }
            report.resumed_bytes += offset;
            let mut input = File::open(&src)?;
            input.seek(SeekFrom::Start(offset))?;
            let mut output = OpenOptions::new()
                .create(true)
                .truncate(!plan.content_verification)
                .write(true)
                .open(&part)?;
            output.seek(SeekFrom::Start(offset))?;
            let mut copied = offset;
            loop {
                check_mounts(plan, source_mount, destination_mount)?;
                let n = input.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                ensure!(
                    copied.saturating_add(n as u64) <= entry.bytes,
                    "Source grew during copying"
                );
                emit(&mut observer, Event::BeforeWrite { file: i, copied })?;
                output.write_all(&buffer[..n])?;
                platform::sync_file(&output)?;
                copied += n as u64;
                report.bytes_written += n as u64;
                state.files[i].copied = copied;
                save_json(&journal_path, &state, true)?;
                emit(
                    &mut observer,
                    Event::Checkpoint {
                        file: i,
                        copied,
                        total: entry.bytes,
                    },
                )?;
            }
            ensure!(
                copied == entry.bytes,
                "Source ended before its expected length"
            );
            output.set_modified(UNIX_EPOCH + Duration::from_nanos(entry.modified_ns))?;
            platform::sync_file(&output)?;
            drop(output);
            drop(input);
            if plan.content_verification {
                emit(&mut observer, Event::BeforeVerify { file: i })?;
                // Source recheck and destination readback may overlap, but publication
                // is impossible until both have matched the planned SHA-256 digest.
                verification::map(
                    2,
                    readers.min(2),
                    |which, stop| {
                        if which == 0 {
                            ensure!(
                                snapshot(&part)?.0 == entry.bytes
                                    && hash_prefix_cancel(&part, None, stop)? == entry.sha256,
                                "Destination readback hash mismatch"
                            );
                            Ok(())
                        } else {
                            validate_source_cancel(plan, entry, stop)
                        }
                    },
                    |_, done| {
                        observer(&Event::VerificationProgress {
                            phase: "verifying".into(),
                            file: i,
                            done,
                            total: 2,
                        })
                    },
                )?;

                state.files[i].state = FileState::Verified;
            } else {
                validate_source(plan, entry)?;
                ensure!(snapshot(&part)?.0 == entry.bytes, "Copy length mismatch");
                state.files[i].state = FileState::ReadyUnverified;
            }
            save_json(&journal_path, &state, true)?;
            check_mounts(plan, source_mount, destination_mount)?;
            fs::create_dir_all(dst.parent().unwrap())?;
            // Re-evaluate every existing path component before publication.
            paths::join(&plan.destination_root, &entry.spec.destination)?;
            emit(&mut observer, Event::BeforePublish { file: i })?;
            if let Err(error) = platform::publish(&part, &dst) {
                if !platform::unsupported(&error) {
                    return Err(error);
                }
                publish_exclusively(
                    plan,
                    i,
                    &part,
                    &dst,
                    &journal_path,
                    &mut state,
                    &mut observer,
                    &mut report,
                    source_mount,
                    destination_mount,
                )?;
            }
            platform::sync_directory(dst.parent().unwrap())?;
            platform::sync_directory(&job)?;
            emit(&mut observer, Event::Published { file: i })?;
            state.files[i].state = FileState::Published;
            save_json(&journal_path, &state, true)?;
            if part.try_exists()? {
                fs::remove_file(&part)?;
                platform::sync_directory(&job)?;
            }
        }
        // Verify again before declaring the whole task complete, including early files.
        verification::map(
            plan.entries.len(),
            readers,
            |i, stop| {
                check_mounts(plan, source_mount, destination_mount)?;
                let entry = &plan.entries[i];
                let dst = paths::join(&plan.destination_root, &entry.spec.destination)?;
                ensure!(
                    snapshot(&dst)?.0 == entry.bytes
                        && (!plan.content_verification
                            || hash_prefix_cancel(&dst, None, stop)? == entry.sha256),
                    "A published destination changed before completion"
                );
                Ok(())
            },
            |i, done| {
                observer(&Event::VerificationProgress {
                    phase: if plan.content_verification {
                        "final_verifying"
                    } else {
                        "final_checking"
                    }
                    .into(),
                    file: i,
                    done,
                    total: plan.entries.len(),
                })
            },
        )?;
        check_mounts(plan, source_mount, destination_mount)?;
        report.content_verified = plan.content_verification;
        report.complete_groups = group_status(plan, &state).values().filter(|v| **v).count();
        state.status = "complete".into();
        save_json(&journal_path, &state, true)?;
        emit(&mut observer, Event::Complete)?;
        Ok(report)
    })();
    if let Err(ref error) = result {
        state.status = if error.downcast_ref::<Paused>().is_some() {
            "paused"
        } else {
            "blocked"
        }
        .into();
        state.last_error = Some(format!("{error:#}"));
        // A disconnected destination may also make the journal unavailable. Do
        // not create replacement root folders or hide the original copy error.
        if check_mounts(plan, source_mount, destination_mount).is_ok() {
            let _ = save_json(&journal_path, &state, true);
        }
    }
    drop(lock); // The OS releases the destination lock after crashes as well.
    result
}

pub fn group_status(plan: &Plan, state: &Journal) -> BTreeMap<String, bool> {
    let mut groups = BTreeMap::new();
    for (entry, progress) in plan.entries.iter().zip(&state.files) {
        let complete = groups.entry(entry.spec.group.clone()).or_insert(true);
        *complete &= progress.state == FileState::Published;
    }
    groups
}

fn validate_publication(
    dst: &Path,
    entry: &Entry,
    progress: &FileProgress,
    part: &Path,
    content_verification: bool,
) -> Result<()> {
    platform::reject_link(dst)?;
    ensure!(
        progress.state == FileState::Publishing,
        "Unexpected file at final path"
    );
    ensure!(
        progress.publishing_identity.as_deref() == Some(platform::file_identity(dst)?.as_str()),
        "Publication file identity changed; refusing to modify it"
    );
    let length = snapshot(dst)?.0;
    ensure!(
        length <= entry.bytes,
        "Publication file exceeds expected size"
    );
    ensure!(
        !content_verification || hash_file(dst)? == hash_prefix(part, Some(length))?,
        "Publication prefix changed; refusing to append"
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn publish_exclusively(
    plan: &Plan,
    index: usize,
    part: &Path,
    dst: &Path,
    journal_path: &Path,
    state: &mut Journal,
    observer: &mut impl FnMut(&Event) -> Result<Control>,
    report: &mut RunReport,
    source_mount: u64,
    destination_mount: u64,
) -> Result<()> {
    let entry = &plan.entries[index];
    let mut output;
    let offset;
    let existing = dst.try_exists()?;
    if existing {
        validate_publication(
            dst,
            entry,
            &state.files[index],
            part,
            plan.content_verification,
        )?;
    }
    if existing && plan.content_verification {
        offset = snapshot(dst)?.0;
        output = OpenOptions::new().write(true).open(dst)?;
        ensure!(
            platform::handle_identity(&output)?
                == state.files[index].publishing_identity.clone().unwrap(),
            "Publication identity changed"
        );
        report.resumed_publication_bytes += offset;
    } else {
        ensure!(
            platform::available_space(&plan.destination_root)?
                >= entry.bytes.saturating_add(16 * 1024 * 1024),
            "Insufficient space for safe exclusive publication"
        );
        output = if existing {
            let file = OpenOptions::new().write(true).open(dst)?;
            ensure!(
                platform::handle_identity(&file)?
                    == state.files[index].publishing_identity.clone().unwrap(),
                "Publication identity changed"
            );
            // Without a verified prefix, restart this owned, incomplete publication.
            file.set_len(0)?;
            file
        } else {
            OpenOptions::new().create_new(true).write(true).open(dst)?
        };
        // ExFAT changes its inode-like ID when an empty file receives its first
        // allocation. Establish the stable ID after writing the initial block.
        // A crash before ownership is journaled fails closed on the next run.
        offset = entry.bytes.min(BUFFER as u64);
        if offset > 0 {
            let mut first = vec![0u8; offset as usize];
            File::open(part)?.read_exact(&mut first)?;
            output.write_all(&first)?;
            report.publication_bytes += offset;
        }
        // Establish the preserved timestamp before recording identity: on macOS,
        // moving mtime before creation time can also move the recorded birth time.
        output.set_modified(UNIX_EPOCH + Duration::from_nanos(entry.modified_ns))?;
        platform::sync_file(&output)?;
        state.files[index].state = FileState::Publishing;
        state.files[index].publishing_identity = Some(platform::handle_identity(&output)?);
        ensure!(
            platform::file_identity(dst)?
                == state.files[index].publishing_identity.clone().unwrap(),
            "New publication file was replaced"
        );
        state.files[index].published_bytes = offset;
        // No existing file is overwritten; the new file was created exclusively.
        save_json(journal_path, state, true)?;
        platform::sync_directory(dst.parent().unwrap())?;
        emit(
            observer,
            Event::PublishingCheckpoint {
                file: index,
                copied: offset,
                total: entry.bytes,
            },
        )?;
    }
    let mut input = File::open(part)?;
    input.seek(SeekFrom::Start(offset))?;
    output.seek(SeekFrom::Start(offset))?;
    let mut copied = offset;
    let mut buffer = vec![0; BUFFER];
    loop {
        check_mounts(plan, source_mount, destination_mount)?;
        ensure!(
            platform::file_identity(dst)?
                == state.files[index].publishing_identity.clone().unwrap(),
            "Publication path was replaced"
        );
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        ensure!(
            copied.saturating_add(n as u64) <= entry.bytes,
            "Staging file grew unexpectedly"
        );
        output.write_all(&buffer[..n])?;
        platform::sync_file(&output)?;
        copied += n as u64;
        report.publication_bytes += n as u64;
        state.files[index].published_bytes = copied;
        save_json(journal_path, state, true)?;
        emit(
            observer,
            Event::PublishingCheckpoint {
                file: index,
                copied,
                total: entry.bytes,
            },
        )?;
    }
    ensure!(copied == entry.bytes, "Publication ended early");
    output.set_modified(UNIX_EPOCH + Duration::from_nanos(entry.modified_ns))?;
    platform::sync_file(&output)?;
    drop(output);
    ensure!(
        platform::file_identity(dst)? == state.files[index].publishing_identity.clone().unwrap(),
        "Publication path was replaced"
    );
    ensure!(
        snapshot(dst)?.0 == entry.bytes
            && (!plan.content_verification || hash_file(dst)? == entry.sha256),
        "Exclusive publication readback mismatch"
    );
    validate_source(plan, entry)?;
    Ok(())
}

#[cfg(test)]
mod unchecked_publication_tests {
    use super::*;
    #[test]
    fn publication_preserves_timestamps_and_resumes_according_to_verification_mode() -> Result<()> {
        for enabled in [false, true] {
            let source = tempfile::tempdir()?;
            let destination = tempfile::tempdir()?;
            let bytes = vec![37u8; BUFFER + 1024];
            fs::write(source.path().join("media.bin"), &bytes)?;
            let plan = prepare_with_settings(
                source.path(),
                destination.path(),
                vec![CopySpec {
                    source: "media.bin".into(),
                    destination: "media.bin".into(),
                    group: "media".into(),
                }],
                verification::Settings {
                    enabled,
                    readers: Some(1),
                },
                |_, _, _| Ok(Control::Continue),
            )?;
            let job = task_directory(&plan)?;
            fs::create_dir_all(&job)?;
            let part = job.join("00000000.part");
            fs::write(&part, &bytes)?;
            let dst = destination.path().join("media.bin");
            let mut state = Journal {
                version: FORMAT_VERSION,
                plan_digest: digest(&plan)?,
                status: "running".into(),
                files: vec![FileProgress {
                    state: FileState::ReadyUnverified,
                    copied: bytes.len() as u64,
                    publishing_identity: None,
                    published_bytes: 0,
                }],
                last_error: None,
            };
            let source_mount = platform::mount_token(source.path())?;
            let destination_mount = platform::mount_token(destination.path())?;
            let mut report = RunReport::default();
            assert!(
                publish_exclusively(
                    &plan,
                    0,
                    &part,
                    &dst,
                    &job.join("journal.json"),
                    &mut state,
                    &mut |_| Ok(Control::Pause),
                    &mut report,
                    source_mount,
                    destination_mount
                )
                .is_err()
            );
            if !enabled {
                OpenOptions::new()
                    .write(true)
                    .open(&dst)?
                    .write_all(b"damaged prefix")?;
            }
            publish_exclusively(
                &plan,
                0,
                &part,
                &dst,
                &job.join("journal.json"),
                &mut state,
                &mut |_| Ok(Control::Continue),
                &mut report,
                source_mount,
                destination_mount,
            )?;
            assert_eq!(fs::read(&dst)?, bytes);
            assert_eq!(
                report.resumed_publication_bytes,
                if enabled { BUFFER as u64 } else { 0 }
            );
            assert!(!report.content_verified);
        }
        Ok(())
    }
}
