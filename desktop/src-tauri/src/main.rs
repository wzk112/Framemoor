#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::{Context, Result, ensure};
use framemoor_core::{
    self as core, Control, Event,
    workflow::{self, Catalog, Options, Preview},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
mod power;

#[derive(Clone, Serialize, Deserialize)]
struct Task {
    id: String,
    status: String,
    phase: String,
    file: usize,
    total_files: usize,
    current_path: String,
    copied: u64,
    total_bytes: u64,
    completed_files: usize,
    error: Option<String>,
    report: Option<core::RunReport>,
    preview: Preview,
    #[serde(default)]
    verification: core::verification::Settings,
    #[serde(default)]
    verification_readers: usize,
    #[serde(default)]
    verification_done: usize,
    #[serde(default)]
    verification_total: usize,
    #[serde(default)]
    sleep_prevented: bool,
    #[serde(default)]
    system_warning: Option<String>,
}
struct Shared {
    data: PathBuf,
    exiftool: PathBuf,
    busy: AtomicBool,
    pause: AtomicBool,
    scan: Mutex<ScanSession>,
    catalogs: Mutex<HashMap<String, Catalog>>,
    previews: Mutex<HashMap<String, Preview>>,
    tasks: Mutex<HashMap<String, Task>>,
    settings: Mutex<Value>,
}
#[derive(Clone, Serialize)]
struct ScanInventoryEvent<'a> {
    scan_id: &'a str,
    #[serde(flatten)]
    inventory: &'a workflow::ScanInventory,
}
#[derive(Default)]
struct ScanSession {
    active: Option<(String, Arc<AtomicBool>)>,
    // Keep one pending request to cover cancel arriving before scan_source is
    // scheduled. IDs are generated afresh for each UI request, never reused.
    pending_cancel: Option<String>,
}
impl ScanSession {
    fn begin(&mut self, id: String) -> Arc<AtomicBool> {
        let cancelled = self.pending_cancel.as_deref() == Some(&id);
        if cancelled {
            self.pending_cancel = None;
        }
        let flag = Arc::new(AtomicBool::new(cancelled));
        self.active = Some((id, flag.clone()));
        flag
    }
    fn cancel(&mut self, id: &str) {
        if let Some((active, flag)) = &self.active
            && active == id
        {
            flag.store(true, Ordering::SeqCst);
        } else {
            self.pending_cancel = Some(id.to_owned());
        }
    }
    fn finish(&mut self, id: &str) -> bool {
        if self.active.as_ref().is_some_and(|(active, _)| active == id) {
            return self.active.take().unwrap().1.load(Ordering::SeqCst);
        }
        false
    }
    fn cancel_active(&self) {
        if let Some((_, flag)) = &self.active {
            flag.store(true, Ordering::SeqCst);
        }
    }
}
type State = Arc<Shared>;
fn chrome_color(theme: &tauri::Theme) -> tauri::window::Color {
    if *theme == tauri::Theme::Dark {
        tauri::window::Color(43, 45, 50, 255)
    } else {
        tauri::window::Color(247, 248, 250, 255)
    }
}
fn failure(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::create_dir_all(path.parent().context("Missing parent")?)?;
    let temp = path.with_extension("tmp");
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    core::platform::replace_journal(&temp, path)?;
    Ok(())
}

#[cfg(test)]
mod encoding_tests {
    use super::*;

    #[test]
    fn persisted_settings_keep_unicode_when_created_and_replaced() -> Result<()> {
        let scratch =
            std::env::temp_dir().join(format!("framemoor-{}-中文📷", uuid::Uuid::new_v4()));
        let result = (|| -> Result<()> {
            let path = scratch.join("设置_日本語_한국어.json");
            for language in ["zh", "en"] {
                let expected = json!({"language":language,"source":"中文/日本語/한국어/العربية/Русский/📷/𠮷",
                    "presets":[{"name":"繁體 café cafe\u{0301}","rules":{"directory_levels":["{date}_{content}","照片"]}}]});
                write_json(&path, &expected)?;
                let bytes = fs::read(&path)?;
                assert!(std::str::from_utf8(&bytes)?.contains("中文"));
                assert_eq!(serde_json::from_slice::<Value>(&bytes)?, expected);
                assert!(!path.with_extension("tmp").exists());
            }
            Ok(())
        })();
        let _ = fs::remove_dir_all(&scratch);
        result
    }
}

#[cfg(test)]
mod desktop_action_tests {
    use super::*;

    #[test]
    fn windows_shell_paths_preserve_unicode_and_unc_names() -> Result<()> {
        assert_eq!(
            windows_destination_argument(Path::new(r"\\?\C:\目标 中文📷"))?,
            r"C:\目标 中文📷"
        );
        assert_eq!(
            windows_destination_argument(Path::new(r"\\?\UNC\server\share\中文"))?,
            r"\\server\share\中文"
        );
        assert_eq!(
            windows_destination_argument(Path::new(r"C:\Camera"))?,
            r"C:\Camera"
        );
        assert!(windows_destination_argument(Path::new(r"\\?\Volume{unknown}\Camera")).is_err());
        Ok(())
    }

    #[test]
    fn cancellation_is_scoped_and_handles_requests_before_registration() {
        let mut scans = ScanSession::default();
        scans.cancel("early");
        let early = scans.begin("early".into());
        assert!(early.load(Ordering::SeqCst));
        assert!(scans.finish("early"));

        let next = scans.begin("next".into());
        scans.cancel("early");
        assert!(!next.load(Ordering::SeqCst));
        scans.cancel("next");
        assert!(next.load(Ordering::SeqCst));
        assert!(scans.finish("next"));

        let fresh = scans.begin("fresh".into());
        assert!(!fresh.load(Ordering::SeqCst));
        assert!(!scans.finish("fresh"));
        assert!(scans.active.is_none());
    }

    #[test]
    fn opening_destination_requires_complete_task_and_original_identity() -> Result<()> {
        let scratch = std::env::temp_dir().join(format!("framemoor-open-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(scratch.join("目标 中文📷"))?;
        fs::create_dir_all(scratch.join("other"))?;
        let result = (|| -> Result<()> {
            let destination = fs::canonicalize(scratch.join("目标 中文📷"))?;
            let identity = core::platform::root_identity(&destination)?;
            assert_eq!(
                checked_destination("complete", &destination, &identity)?,
                destination
            );
            assert!(checked_destination("running", &destination, &identity).is_err());
            assert!(checked_destination("paused", &destination, &identity).is_err());
            let other = fs::canonicalize(scratch.join("other"))?;
            assert!(checked_destination("complete", &other, &identity).is_err());
            fs::remove_dir(&destination)?;
            assert!(checked_destination("complete", &destination, &identity).is_err());
            assert!(
                !destination.exists(),
                "Opening a missing destination must not recreate it"
            );
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&other, &destination)?;
                assert!(checked_destination("complete", &destination, &identity).is_err());
            }
            Ok(())
        })();
        let _ = fs::remove_dir_all(&scratch);
        result
    }
}
fn save_task(state: &State, task: &Task) -> Result<()> {
    write_json(
        &state.data.join("tasks").join(&task.id).join("task.json"),
        task,
    )
}
fn update(app: &tauri::AppHandle, state: &State, task: &Task, persist: bool) {
    state
        .tasks
        .lock()
        .unwrap()
        .insert(task.id.clone(), task.clone());
    if persist && let Err(e) = save_task(state, task) {
        eprintln!("Task history persistence: {e}");
    }
    let _ = app.emit("task-progress", task);
}
fn apply_theme(window: &tauri::WebviewWindow, appearance: &str) -> tauri::Result<()> {
    let theme = match appearance {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        _ => None,
    };
    window.set_theme(theme)?;
    window.set_background_color(Some(chrome_color(&window.theme()?)))
}
#[tauri::command]
fn test_notification(app: tauri::AppHandle, state: tauri::State<State>) -> Result<(), String> {
    let english = state
        .settings
        .lock()
        .unwrap()
        .get("language")
        .and_then(Value::as_str)
        == Some("en");
    app.notification()
        .builder()
        .title(if english {
            "Framemoor"
        } else {
            "泊影 Framemoor"
        })
        .body(if english {
            "Test notification. Completed imports will notify you here."
        } else {
            "测试通知。导入完成后会在这里提醒你。"
        })
        .show()
        .map_err(failure)
}
#[tauri::command]
fn bootstrap(state: tauri::State<State>) -> Value {
    let mut tasks: Vec<_> = state.tasks.lock().unwrap().values().cloned().collect();
    tasks.sort_by_cached_key(|task| {
        std::cmp::Reverse(
            fs::metadata(state.data.join("tasks").join(&task.id).join("task.json"))
                .and_then(|metadata| metadata.modified())
                .ok(),
        )
    });
    let locations = available_locations();
    json!({"settings":state.settings.lock().unwrap().clone(),"tasks":tasks,"locations":locations,"metadata_ready":state.exiftool.is_file(),"version":env!("CARGO_PKG_VERSION")})
}
fn available_locations() -> Vec<String> {
    let mut locations = Vec::new();
    #[cfg(target_os = "macos")]
    if let Ok(items) = fs::read_dir("/Volumes") {
        for item in items.flatten() {
            if item.path().is_dir()
                && !item.file_name().to_string_lossy().starts_with('.')
                && item.file_name() != "Recovery"
            {
                locations.push(item.path().to_string_lossy().to_string());
            }
        }
    }
    #[cfg(target_os = "windows")]
    for letter in b'C'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        if Path::new(&drive).is_dir() {
            locations.push(drive);
        }
    }
    locations.sort();
    locations.dedup();
    locations
}
#[tauri::command]
async fn refresh_locations() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(available_locations)
        .await
        .map_err(failure)
}
#[tauri::command]
async fn verification_recommendation(
    source: String,
    destination: String,
) -> Result<core::verification::Recommendation, String> {
    tauri::async_runtime::spawn_blocking(move || {
        core::verification::recommendation(Path::new(&source), Path::new(&destination))
    })
    .await
    .map_err(failure)
}
#[tauri::command]
fn selection_summary(
    state: tauri::State<State>,
    catalog_id: String,
    options: Options,
) -> Result<workflow::SelectionSummary, String> {
    let catalogs = state.catalogs.lock().unwrap();
    let catalog = catalogs.get(&catalog_id).ok_or("catalog_expired")?;
    workflow::selection_summary(catalog, &options, chrono::Utc::now()).map_err(failure)
}
#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<State>,
    settings: Value,
) -> Result<(), String> {
    let mut guard = state.settings.lock().unwrap();
    write_json(&state.data.join("settings.json"), &settings).map_err(failure)?;
    let theme = settings
        .get("appearance")
        .and_then(Value::as_str)
        .unwrap_or("system")
        .to_owned();
    *guard = settings;
    drop(guard);
    if let Some(window) = app.get_webview_window("main") {
        apply_theme(&window, &theme).map_err(failure)?;
    }
    Ok(())
}
#[tauri::command]
async fn scan_source(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
    source: String,
    scan_id: Option<String>,
) -> Result<Catalog, String> {
    let scan_id = scan_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    uuid::Uuid::parse_str(&scan_id).map_err(|_| "invalid_scan_id")?;
    let state = state.inner().clone();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("busy".into());
    }
    let cancelled = state.scan.lock().unwrap().begin(scan_id.clone());
    // A failed new scan must not leave a previous catalog or preview usable.
    state.catalogs.lock().unwrap().clear();
    state.previews.lock().unwrap().clear();
    let worker = state.clone();
    let event_scan_id = scan_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        workflow::scan_cancellable(
            Path::new(&source),
            &worker.exiftool,
            workflow::default_scan_readers(),
            &cancelled,
            |inventory| {
                if !cancelled.load(Ordering::SeqCst) {
                    let _ = app.emit(
                        "scan-inventory",
                        ScanInventoryEvent {
                            scan_id: &event_scan_id,
                            inventory,
                        },
                    );
                }
            },
            |done, total| {
                if !cancelled.load(Ordering::SeqCst) {
                    let _ = app.emit(
                        "scan-progress",
                        json!({"scan_id":event_scan_id,"done":done,"total":total}),
                    );
                }
            },
        )
    })
    .await;
    // Finalization and cancellation are ordered by this lock. Never register a
    // partial/cancelled catalog or release busy before worker cleanup finishes.
    let mut session = state.scan.lock().unwrap();
    let result = if session.finish(&scan_id) {
        Err("scan_cancelled".into())
    } else {
        result
            .map_err(failure)
            .and_then(|result| result.map_err(failure))
    };
    if let Ok(catalog) = &result {
        state
            .catalogs
            .lock()
            .unwrap()
            .insert(catalog.id.clone(), catalog.clone());
    }
    state.busy.store(false, Ordering::SeqCst);
    result
}
#[tauri::command]
fn cancel_scan(state: tauri::State<State>, scan_id: String) -> Result<(), String> {
    uuid::Uuid::parse_str(&scan_id).map_err(|_| "invalid_scan_id")?;
    state.scan.lock().unwrap().cancel(&scan_id);
    Ok(())
}
async fn compute_preview(
    state: &State,
    catalog_id: String,
    options: Options,
) -> Result<Preview, String> {
    let catalog = state
        .catalogs
        .lock()
        .unwrap()
        .get(&catalog_id)
        .cloned()
        .ok_or("scan_required")?;
    tauri::async_runtime::spawn_blocking(move || {
        workflow::preview(&catalog, options, chrono::Utc::now())
    })
    .await
    .map_err(failure)?
    .map_err(failure)
}
// Live directory layouts use the authoritative path rules but never register an
// importable preview or replace the user's explicitly reviewed import plan.
#[tauri::command]
async fn preview_layout(
    state: tauri::State<'_, State>,
    catalog_id: String,
    options: Options,
) -> Result<Preview, String> {
    compute_preview(state.inner(), catalog_id, options).await
}
#[tauri::command]
async fn build_preview(
    state: tauri::State<'_, State>,
    catalog_id: String,
    options: Options,
) -> Result<Preview, String> {
    let result = compute_preview(state.inner(), catalog_id, options).await?;
    let mut previews = state.previews.lock().unwrap();
    previews.clear();
    previews.insert(result.id.clone(), result.clone());
    Ok(result)
}
fn execute(app: tauri::AppHandle, state: State, mut task: Task) {
    let prevent_sleep = state
        .settings
        .lock()
        .unwrap()
        .get("prevent_sleep")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    task.system_warning = None;
    let sleep_guard = if prevent_sleep {
        match power::SleepGuard::acquire("Framemoor import") {
            Ok(guard) => Some(guard),
            Err(error) => {
                eprintln!("Sleep prevention: {error}");
                task.system_warning = Some("sleep_prevention_unavailable".into());
                None
            }
        }
    } else {
        None
    };
    task.sleep_prevented = sleep_guard.is_some();
    let result = (|| -> Result<core::RunReport> {
        let readers = task
            .verification
            .resolve(&task.preview.source, &task.preview.destination)?;
        task.verification_readers = readers;
        task.verification_done = 0;
        task.verification_total = 0;
        update(&app, &state, &task, true);
        let plan_path = state.data.join("tasks").join(&task.id).join("plan.json");
        let plan = if plan_path.is_file() {
            core::load_plan(&plan_path)?
        } else {
            let specs = workflow::validate_snapshot(&task.preview)?;
            task.phase = if task.verification.enabled {
                "preparing"
            } else {
                "checking"
            }
            .into();
            update(&app, &state, &task, true);
            let (source, destination) = (
                task.preview.source.clone(),
                task.preview.destination.clone(),
            );
            let plan = core::prepare_with_settings(
                &source,
                &destination,
                specs,
                task.verification,
                |index, _, path| {
                    task.file = index;
                    task.current_path = path.into();
                    update(&app, &state, &task, false);
                    Ok(if state.pause.load(Ordering::SeqCst) {
                        Control::Pause
                    } else {
                        Control::Continue
                    })
                },
            )?;
            // A preview is a frozen path/size/date selection. Hashing must not quietly
            // accept metadata changes that happened after preview validation.
            workflow::validate_snapshot(&task.preview)?;
            core::save_plan(&plan_path, &plan)?;
            plan
        };
        ensure!(
            plan.content_verification == task.verification.enabled,
            "Task verification mode does not match its plan"
        );
        let mut amounts = vec![0u64; plan.entries.len()];
        let mut completed = vec![false; plan.entries.len()];
        let mut last = std::time::Instant::now();
        core::run_with_readers(&plan, readers, |event| {
            if !matches!(event, Event::VerificationProgress { .. }) {
                task.verification_total = 0;
            }
            let index = match event {
                Event::VerificationProgress {
                    phase,
                    file,
                    done,
                    total,
                } => {
                    task.phase = phase.clone();
                    task.verification_done = *done;
                    task.verification_total = *total;
                    Some(*file)
                }
                Event::Preflight { file, .. } => {
                    task.phase = "checking".into();
                    Some(*file)
                }
                Event::BeforeWrite { file, copied } => {
                    task.phase = "copying".into();
                    amounts[*file] = *copied;
                    Some(*file)
                }
                Event::Checkpoint { file, copied, .. } => {
                    task.phase = "copying".into();
                    amounts[*file] = *copied;
                    Some(*file)
                }
                Event::BeforeVerify { file } => {
                    task.phase = "verifying".into();
                    Some(*file)
                }
                Event::BeforePublish { file } => {
                    task.phase = "publishing".into();
                    Some(*file)
                }
                Event::PublishingCheckpoint { file, .. } => {
                    task.phase = "publishing".into();
                    Some(*file)
                }
                Event::Published { file } => {
                    completed[*file] = true;
                    amounts[*file] = plan.entries[*file].bytes;
                    Some(*file)
                }
                Event::Complete => None,
            };
            if let Some(index) = index {
                task.file = index;
                task.current_path = plan.entries[index].spec.source.clone();
            }
            task.copied = amounts.iter().sum();
            task.completed_files = completed.iter().filter(|v| **v).count();
            if last.elapsed().as_millis() >= 100
                || !matches!(
                    event,
                    Event::Checkpoint { .. }
                        | Event::PublishingCheckpoint { .. }
                        | Event::VerificationProgress { .. }
                )
            {
                update(&app, &state, &task, false);
                last = std::time::Instant::now();
            }
            Ok(if state.pause.load(Ordering::SeqCst) {
                Control::Pause
            } else {
                Control::Continue
            })
        })
    })();
    drop(sleep_guard);
    task.sleep_prevented = false;
    match result {
        Ok(report) => {
            task.status = "complete".into();
            task.phase = "complete".into();
            task.completed_files = task.total_files;
            task.copied = task.total_bytes;
            task.report = Some(report);
            task.error = None;
        }
        Err(e) => {
            task.status = if state.pause.load(Ordering::SeqCst) {
                "paused"
            } else {
                "failed"
            }
            .into();
            task.error = Some(format!("{e:#}"));
        }
    }
    let settings = state.settings.lock().unwrap().clone();
    if task.status == "complete"
        && settings
            .get("completion_notification")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        let english = settings.get("language").and_then(Value::as_str) == Some("en");
        let title = if english {
            "Import complete"
        } else {
            "导入完成"
        };
        let body = match (english, task.verification.enabled) {
            (true, true) => format!("{} files copied and verified.", task.total_files),
            (false, true) => format!("{} 个文件已复制并通过校验。", task.total_files),
            (true, false) => format!("{} files copied. Content not verified.", task.total_files),
            (false, false) => format!("{} 个文件已复制，内容未经校验。", task.total_files),
        };
        if let Err(error) = app.notification().builder().title(title).body(body).show() {
            eprintln!("Completion notification: {error}");
            task.system_warning = Some("notification_unavailable".into());
        }
    }
    state.busy.store(false, Ordering::SeqCst);
    update(&app, &state, &task, true);
}
#[tauri::command]
fn start_import(
    app: tauri::AppHandle,
    state: tauri::State<State>,
    preview_id: String,
    acknowledged: bool,
    verification: Option<core::verification::Settings>,
) -> Result<Task, String> {
    let verification = verification.unwrap_or_default();
    verification.validate().map_err(failure)?;
    let preview = state
        .previews
        .lock()
        .unwrap()
        .get(&preview_id)
        .cloned()
        .ok_or("preview_expired")?;
    if !preview.blockers.is_empty() {
        return Err("preview_blocked".into());
    }
    if !preview.warnings.is_empty() && !acknowledged {
        return Err("acknowledgement_required".into());
    }
    // The task journal belongs outside the source tree as well.
    let data_identity = core::platform::root_identity(&state.data).map_err(failure)?;
    core::paths::disjoint(&preview.source_identity, &data_identity).map_err(failure)?;
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("busy".into());
    }
    state.pause.store(false, Ordering::SeqCst);
    let task = Task {
        id: uuid::Uuid::new_v4().to_string(),
        status: "running".into(),
        phase: if verification.enabled {
            "preparing"
        } else {
            "checking"
        }
        .into(),
        file: 0,
        total_files: preview.rows.len(),
        current_path: String::new(),
        copied: 0,
        total_bytes: preview.bytes,
        completed_files: 0,
        error: None,
        report: None,
        preview,
        verification,
        verification_readers: 0,
        verification_done: 0,
        verification_total: 0,
        sleep_prevented: false,
        system_warning: None,
    };
    if let Err(e) = save_task(&state, &task) {
        state.busy.store(false, Ordering::SeqCst);
        return Err(failure(e));
    }
    update(&app, &state, &task, false);
    let (worker, record) = (state.inner().clone(), task.clone());
    tauri::async_runtime::spawn_blocking(move || execute(app, worker, record));
    Ok(task)
}
#[tauri::command]
fn pause_import(state: tauri::State<State>) {
    state.pause.store(true, Ordering::SeqCst);
}
#[tauri::command]
fn resume_task(
    app: tauri::AppHandle,
    state: tauri::State<State>,
    task_id: String,
) -> Result<Task, String> {
    let mut task = state
        .tasks
        .lock()
        .unwrap()
        .get(&task_id)
        .cloned()
        .ok_or("task_missing")?;
    if task.status == "complete" {
        return Err("task_complete".into());
    }
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("busy".into());
    }
    state.pause.store(false, Ordering::SeqCst);
    task.status = "running".into();
    task.error = None;
    update(&app, &state, &task, true);
    let (worker, record) = (state.inner().clone(), task.clone());
    tauri::async_runtime::spawn_blocking(move || execute(app, worker, record));
    Ok(task)
}
#[tauri::command]
fn get_task(state: tauri::State<State>, task_id: String) -> Result<Task, String> {
    state
        .tasks
        .lock()
        .unwrap()
        .get(&task_id)
        .cloned()
        .ok_or("task_missing".into())
}
fn checked_destination(
    status: &str,
    destination: &Path,
    identity: &core::platform::RootIdentity,
) -> Result<PathBuf> {
    ensure!(status == "complete", "task_not_complete");
    core::platform::reject_link(destination).context("destination_unavailable")?;
    let canonical = fs::canonicalize(destination).context("destination_unavailable")?;
    ensure!(
        canonical == destination && canonical.is_dir(),
        "destination_changed"
    );
    ensure!(
        core::platform::root_identity(&canonical)? == *identity,
        "destination_changed"
    );
    Ok(canonical)
}
#[cfg(any(windows, test))]
fn windows_destination_argument(destination: &Path) -> Result<String> {
    let value = destination.to_str().context("open_destination_failed")?;
    // Rust's canonical paths use the Win32 extended prefix; Explorer expects
    // the normal drive/UNC spelling. Identity was checked before this conversion.
    if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
        return Ok(format!(r"\\{rest}"));
    }
    if let Some(rest) = value.strip_prefix(r"\\?\") {
        ensure!(
            rest.as_bytes().get(1) == Some(&b':'),
            "open_destination_failed"
        );
        return Ok(rest.to_owned());
    }
    Ok(value.to_owned())
}
#[tauri::command]
async fn open_task_destination(
    state: tauri::State<'_, State>,
    task_id: String,
) -> Result<(), String> {
    let task = state
        .tasks
        .lock()
        .unwrap()
        .get(&task_id)
        .cloned()
        .ok_or("task_missing")?;
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let destination = checked_destination(
            &task.status,
            &task.preview.destination,
            &task.preview.destination_identity,
        )?;
        #[cfg(target_os = "macos")]
        {
            let status = std::process::Command::new("/usr/bin/open")
                .arg(&destination)
                .status()
                .context("open_destination_failed")?;
            ensure!(status.success(), "open_destination_failed");
        }
        #[cfg(target_os = "windows")]
        {
            // Explorer may hand the request to an existing process and return
            // a nonzero status. Successful spawn is the supported handoff.
            let system_root = std::env::var_os("SystemRoot").context("open_destination_failed")?;
            let mut child =
                std::process::Command::new(PathBuf::from(system_root).join("explorer.exe"))
                    .arg(windows_destination_argument(&destination)?)
                    .spawn()
                    .context("open_destination_failed")?;
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Ok(())
    })
    .await
    .map_err(failure)?
    .map_err(failure)
}
#[tauri::command]
fn export_report(state: tauri::State<State>, task_id: String, path: String) -> Result<(), String> {
    let result = (|| -> Result<()> {
        let task = state
            .tasks
            .lock()
            .unwrap()
            .get(&task_id)
            .cloned()
            .context("task_missing")?;
        let path = Path::new(&path);
        let parent = fs::canonicalize(path.parent().context("invalid_report_path")?)?;
        core::paths::disjoint(
            &task.preview.source_identity,
            &core::platform::root_identity(&parent)?,
        )?;
        ensure!(!path.exists(), "report_exists");
        let plan_path = state.data.join("tasks").join(&task_id).join("plan.json");
        let plan = if plan_path.is_file() {
            Some(core::load_plan(&plan_path)?)
        } else {
            None
        };
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        use std::io::Write;
        output.write_all(&serde_json::to_vec_pretty(
            &json!({"task":task,"plan":plan}),
        )?)?;
        output.sync_all()?;
        Ok(())
    })();
    result.map_err(failure)
}
fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_background_color(Some(chrome_color(&window.theme()?)))?;
            }
            let data = std::env::var_os("FRAMEMOOR_DATA_DIR")
                .or_else(|| std::env::var_os("CARD_INGEST_DATA_DIR"))
                .map(PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?);
            fs::create_dir_all(data.join("tasks"))?;
            let resources = app.path().resource_dir()?;
            let exiftool = std::env::var_os("FRAMEMOOR_EXIFTOOL")
                .or_else(|| std::env::var_os("CARD_INGEST_EXIFTOOL"))
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    #[cfg(target_os = "windows")]
                    {
                        resources.join("exiftool-win/exiftool.exe")
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        resources.join("exiftool/exiftool")
                    }
                });
            let settings: Value = fs::read(data.join("settings.json"))
                .ok()
                .and_then(|v| serde_json::from_slice(&v).ok())
                .unwrap_or(json!({"language":"zh","sources":[],"destinations":[],"presets":[]}));
            if let Some(window) = app.get_webview_window("main") {
                apply_theme(
                    &window,
                    settings
                        .get("appearance")
                        .and_then(Value::as_str)
                        .unwrap_or("system"),
                )?;
            }
            let mut tasks = HashMap::new();
            for entry in fs::read_dir(data.join("tasks"))?.flatten() {
                if let Ok(bytes) = fs::read(entry.path().join("task.json"))
                    && let Ok(mut task) = serde_json::from_slice::<Task>(&bytes)
                {
                    if task.status == "running" {
                        task.status = "interrupted".into();
                    }
                    task.sleep_prevented = false;
                    tasks.insert(task.id.clone(), task);
                }
            }
            app.manage(Arc::new(Shared {
                data,
                exiftool,
                busy: AtomicBool::new(false),
                pause: AtomicBool::new(false),
                scan: Mutex::new(ScanSession::default()),
                catalogs: Mutex::new(HashMap::new()),
                previews: Mutex::new(HashMap::new()),
                tasks: Mutex::new(tasks),
                settings: Mutex::new(settings),
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            refresh_locations,
            verification_recommendation,
            save_settings,
            scan_source,
            cancel_scan,
            build_preview,
            preview_layout,
            selection_summary,
            start_import,
            pause_import,
            resume_task,
            get_task,
            open_task_destination,
            export_report,
            test_notification
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::ThemeChanged(theme) = event {
                let _ = window.set_background_color(Some(chrome_color(theme)));
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<State>();
                if state.busy.load(Ordering::SeqCst) {
                    api.prevent_close();
                    state.pause.store(true, Ordering::SeqCst);
                    state.scan.lock().unwrap().cancel_active();
                    let _ = window.emit("close-deferred", ());
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Failed to launch Framemoor");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            let state = app.state::<State>();
            if state.busy.load(Ordering::SeqCst) {
                api.prevent_exit();
                state.pause.store(true, Ordering::SeqCst);
                state.scan.lock().unwrap().cancel_active();
                let _ = app.emit("close-deferred", ());
            }
        }
    });
}
