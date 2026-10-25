use anyhow::{Result, bail};
use framemoor_core::{Control, CopySpec, Event, FileState, Plan};
use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
};
use tempfile::TempDir;

struct Fixture {
    _src: TempDir,
    _dst: TempDir,
    plan: Plan,
}

impl Fixture {
    fn new(size: usize, companion: bool) -> Result<Self> {
        let src = tempfile::tempdir()?;
        let dst = if let Some(root) = std::env::var_os("CARD_INGEST_TEST_DESTINATION") {
            tempfile::Builder::new()
                .prefix("core-test-")
                .tempdir_in(root)?
        } else {
            tempfile::tempdir()?
        };
        let data: Vec<_> = (0..size).map(|n| (n % 251) as u8).collect();
        fs::write(src.path().join("clip.bin"), data)?;
        let mut specs = vec![CopySpec {
            source: "clip.bin".into(),
            destination: "日期/clip.bin".into(),
            group: "clip".into(),
        }];
        if companion {
            fs::write(src.path().join("clip.CPF"), b"synthetic companion")?;
            specs.push(CopySpec {
                source: "clip.CPF".into(),
                destination: "日期/clip.CPF".into(),
                group: "clip".into(),
            });
        }
        let plan = framemoor_core::prepare(src.path(), dst.path(), specs)?;
        Ok(Self {
            _src: src,
            _dst: dst,
            plan,
        })
    }
    fn part(&self, index: usize) -> PathBuf {
        self.plan
            .destination_root
            .join(".card-ingest")
            .join(&self.plan.id)
            .join(format!("{index:08}.part"))
    }
    fn dst(&self) -> PathBuf {
        self.plan.destination_root.join("日期/clip.bin")
    }
    fn run(&self) -> Result<framemoor_core::RunReport> {
        framemoor_core::run(&self.plan, |_| Ok(Control::Continue))
    }
}

#[test]
fn copy_group_and_idempotent_rerun() -> Result<()> {
    let f = Fixture::new(1024 * 1024, true)?;
    let r = f.run()?;
    assert_eq!(r.complete_groups, 1);
    framemoor_core::verify(&f.plan)?;
    let r = f.run()?;
    assert_eq!(r.bytes_written + r.publication_bytes, 0);
    assert_eq!(r.skipped_verified_files, 2);
    assert_eq!(framemoor_core::status(&f.plan)?.unwrap().status, "complete");
    Ok(())
}

#[test]
fn paused_copy_resumes_from_verified_prefix() -> Result<()> {
    let f = Fixture::new(20 * 1024 * 1024, false)?;
    assert!(
        framemoor_core::run(&f.plan, |e| Ok(if matches!(e, Event::Checkpoint { .. }) {
            Control::Pause
        } else {
            Control::Continue
        }))
        .is_err()
    );
    assert!(!f.dst().exists());
    assert_eq!(framemoor_core::status(&f.plan)?.unwrap().status, "paused");
    let r = f.run()?;
    assert_eq!(r.resumed_bytes, 8 * 1024 * 1024);
    framemoor_core::verify(&f.plan)?;
    Ok(())
}

#[test]
fn mutated_source_with_same_size_is_rejected() -> Result<()> {
    let f = Fixture::new(1024, false)?;
    OpenOptions::new()
        .write(true)
        .open(f.plan.source_root.join("clip.bin"))?
        .write_all(b"bad")?;
    assert!(f.run().is_err());
    assert!(!f.dst().exists());
    Ok(())
}

#[test]
fn corrupt_partial_is_never_appended() -> Result<()> {
    let f = Fixture::new(10 * 1024 * 1024, false)?;
    assert!(
        framemoor_core::run(&f.plan, |e| Ok(if matches!(e, Event::Checkpoint { .. }) {
            Control::Pause
        } else {
            Control::Continue
        }))
        .is_err()
    );
    OpenOptions::new()
        .write(true)
        .open(f.part(0))?
        .write_all(b"bad")?;
    assert!(f.run().is_err());
    assert!(!f.dst().exists());
    Ok(())
}

#[test]
fn corrupt_target_readback_is_rejected() -> Result<()> {
    let f = Fixture::new(1024, false)?;
    let r = framemoor_core::run(&f.plan, |e| {
        if matches!(e, Event::BeforeVerify { .. }) {
            OpenOptions::new()
                .write(true)
                .open(f.part(0))?
                .write_all(b"bad")?;
        }
        Ok(Control::Continue)
    });
    assert!(r.is_err());
    assert!(!f.dst().exists());
    Ok(())
}

#[test]
fn existing_final_file_is_not_overwritten() -> Result<()> {
    let f = Fixture::new(1024, false)?;
    fs::create_dir_all(f.dst().parent().unwrap())?;
    fs::write(f.dst(), b"keep")?;
    assert!(f.run().is_err());
    assert_eq!(fs::read(f.dst())?, b"keep");
    Ok(())
}

#[test]
fn publication_race_is_not_overwritten() -> Result<()> {
    let f = Fixture::new(1024, false)?;
    let r = framemoor_core::run(&f.plan, |e| {
        if matches!(e, Event::BeforePublish { .. }) {
            fs::write(f.dst(), b"keep-late-arrival")?;
        }
        Ok(Control::Continue)
    });
    assert!(r.is_err());
    assert_eq!(fs::read(f.dst())?, b"keep-late-arrival");
    Ok(())
}

#[test]
fn published_file_survives_journal_gap() -> Result<()> {
    let f = Fixture::new(1024, false)?;
    assert!(
        framemoor_core::run(&f.plan, |e| {
            if matches!(e, Event::Published { .. }) {
                bail!("Injected interruption after publication");
            }
            Ok(Control::Continue)
        })
        .is_err()
    );
    assert!(f.dst().exists());
    f.run()?;
    framemoor_core::verify(&f.plan)?;
    Ok(())
}

#[test]
fn companion_failure_keeps_group_incomplete() -> Result<()> {
    let f = Fixture::new(1024, true)?;
    assert!(
        framemoor_core::run(&f.plan, |e| {
            if matches!(e, Event::BeforeWrite { file: 1, .. }) {
                bail!("Injected companion failure");
            }
            Ok(Control::Continue)
        })
        .is_err()
    );
    let state = framemoor_core::status(&f.plan)?.unwrap();
    assert!(!framemoor_core::group_status(&f.plan, &state)["clip"]);
    f.run()?;
    assert!(
        framemoor_core::group_status(&f.plan, &framemoor_core::status(&f.plan)?.unwrap())["clip"]
    );
    Ok(())
}

#[test]
fn concurrent_task_is_blocked_by_os_lock() -> Result<()> {
    let f = Fixture::new(1024, false)?;
    let mut attempted = false;
    framemoor_core::run(&f.plan, |e| {
        if matches!(e, Event::Checkpoint { .. }) && !attempted {
            attempted = true;
            let err = f.run().unwrap_err();
            assert!(format!("{err:#}").contains("Another import"));
        }
        Ok(Control::Continue)
    })?;
    assert!(attempted);
    Ok(())
}

#[test]
fn changed_completed_destination_is_rejected() -> Result<()> {
    let f = Fixture::new(1024, false)?;
    f.run()?;
    OpenOptions::new()
        .write(true)
        .open(f.dst())?
        .write_all(b"bad")?;
    assert!(f.run().is_err());
    assert!(framemoor_core::verify(&f.plan).is_err());
    Ok(())
}

#[test]
fn plan_modification_cannot_reuse_task() -> Result<()> {
    let mut f = Fixture::new(1024, false)?;
    f.run()?;
    f.plan.entries[0].spec.destination = "different.bin".into();
    assert!(f.run().is_err());
    assert!(!f.plan.destination_root.join("different.bin").exists());
    Ok(())
}

#[test]
fn portable_paths_and_directory_conflicts() -> Result<()> {
    for name in [
        "../x",
        "/absolute",
        "a\\b",
        "a//b",
        "CON.txt",
        "COM1",
        "LPT¹.mov",
        "a:",
        "end.",
        "end ",
        ".card-ingest/private",
    ] {
        assert!(framemoor_core::paths::portable_key(name).is_err(), "{name}");
    }
    assert_eq!(
        framemoor_core::paths::portable_key("A/Caf\u{e9}.CR3")?,
        framemoor_core::paths::portable_key("a/cafe\u{301}.cr3")?
    );
    let f = Fixture::new(1024, true)?;
    let mut plan = f.plan.clone();
    plan.entries[0].spec.destination = "x".into();
    plan.entries[1].spec.destination = "x/y".into();
    assert!(framemoor_core::run(&plan, |_| Ok(Control::Continue)).is_err());
    Ok(())
}

#[test]
fn root_overlap_is_rejected_before_writes() -> Result<()> {
    let f = Fixture::new(1024, false)?;
    let child = f.plan.source_root.join("nested");
    fs::create_dir(&child)?;
    assert!(
        framemoor_core::prepare(
            &f.plan.source_root,
            &child,
            vec![f.plan.entries[0].spec.clone()]
        )
        .is_err()
    );
    assert!(!child.join(".card-ingest").exists());
    Ok(())
}

#[test]
fn zero_length_file_and_scan_unknown_formats() -> Result<()> {
    let f = Fixture::new(0, false)?;
    f.run()?;
    framemoor_core::verify(&f.plan)?;
    fs::write(
        f.plan.source_root.join("new.unknown"),
        b"opaque camera data",
    )?;
    fs::write(f.plan.source_root.join(".hidden"), b"hidden")?;
    assert_eq!(framemoor_core::discover(&f.plan.source_root)?.len(), 2);
    Ok(())
}

#[test]
fn exclusive_publication_resume_and_tamper_detection() -> Result<()> {
    let f = Fixture::new(20 * 1024 * 1024, false)?;
    if !f
        .plan
        .destination_identity
        .filesystem
        .eq_ignore_ascii_case("exfat")
    {
        return Ok(());
    }
    assert!(
        framemoor_core::run(&f.plan, |e| Ok(
            if matches!(e, Event::PublishingCheckpoint { .. }) {
                Control::Pause
            } else {
                Control::Continue
            }
        ))
        .is_err()
    );
    let state = framemoor_core::status(&f.plan)?.unwrap();
    assert_eq!(state.files[0].state, FileState::Publishing);
    assert_eq!(fs::metadata(f.dst())?.len(), 8 * 1024 * 1024);
    let r = f.run()?;
    assert_eq!(r.resumed_publication_bytes, 8 * 1024 * 1024);
    framemoor_core::verify(&f.plan)?;
    let f = Fixture::new(20 * 1024 * 1024, false)?;
    assert!(
        framemoor_core::run(&f.plan, |e| Ok(
            if matches!(e, Event::PublishingCheckpoint { .. }) {
                Control::Pause
            } else {
                Control::Continue
            }
        ))
        .is_err()
    );
    OpenOptions::new()
        .write(true)
        .open(f.dst())?
        .write_all(b"tampered")?;
    assert!(f.run().is_err());
    Ok(())
}

#[test]
fn injected_write_failure_retains_partial_for_retry() -> Result<()> {
    let f = Fixture::new(20 * 1024 * 1024, false)?;
    assert!(
        framemoor_core::run(&f.plan, |e| {
            if matches!(e, Event::BeforeWrite {copied,..} if *copied > 0) {
                bail!("Injected disk write failure");
            }
            Ok(Control::Continue)
        })
        .is_err()
    );
    assert!(!f.dst().exists());
    assert_eq!(f.run()?.resumed_bytes, 8 * 1024 * 1024);
    framemoor_core::verify(&f.plan)?;
    Ok(())
}

#[test]
fn output_mtime_preserves_source_with_filesystem_precision() -> Result<()> {
    let f = Fixture::new(1024, false)?;
    f.run()?;
    let src = fs::metadata(f.plan.source_root.join("clip.bin"))?.modified()?;
    let dst = fs::metadata(f.dst())?.modified()?;
    let difference = src
        .duration_since(dst)
        .or_else(|_| dst.duration_since(src))?;
    assert!(difference.as_millis() <= 2000);
    Ok(())
}

#[test]
fn malformed_journal_is_not_silently_reset() -> Result<()> {
    let f = Fixture::new(1024, false)?;
    f.run()?;
    let journal = f.part(0).parent().unwrap().join("journal.json");
    let mut file = File::options().write(true).open(journal)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(b"CORRUPT")?;
    assert!(f.run().is_err());
    Ok(())
}

#[test]
fn plan_cannot_be_written_into_source() -> Result<()> {
    let f = Fixture::new(1, false)?;
    assert!(framemoor_core::save_plan(&f.plan.source_root.join("plan.json"), &f.plan).is_err());
    assert!(!f.plan.source_root.join("plan.json").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn destination_symlink_is_rejected() -> Result<()> {
    let f = Fixture::new(1024, false)?;
    let unrelated = tempfile::tempdir()?;
    std::os::unix::fs::symlink(unrelated.path(), f.plan.destination_root.join("日期"))?;
    assert!(f.run().is_err());
    assert!(!unrelated.path().join("clip.bin").exists());
    Ok(())
}

#[test]
fn parallel_verification_preserves_hashes_and_resume() -> Result<()> {
    for readers in [1, 2, 4, 8] {
        let f = Fixture::new(1024 * 1024, true)?;
        let specs = f.plan.entries.iter().map(|e| e.spec.clone()).collect();
        let p = framemoor_core::prepare_with_readers(
            &f.plan.source_root,
            &f.plan.destination_root,
            specs,
            readers,
            |_, _, _| Ok(Control::Continue),
        )?;
        assert_eq!(
            serde_json::to_value(&p.entries)?,
            serde_json::to_value(&f.plan.entries)?
        );
        let r = framemoor_core::run_with_readers(&p, readers, |_| Ok(Control::Continue))?;
        assert_eq!(r.complete_groups, 1);
        framemoor_core::verify_with_readers(&p, readers)?;
        let r = framemoor_core::run_with_readers(&p, readers, |_| Ok(Control::Continue))?;
        assert_eq!(r.bytes_written + r.publication_bytes, 0);
    }
    Ok(())
}

#[test]
fn pause_during_parallel_readback_keeps_recoverable_part() -> Result<()> {
    let f = Fixture::new(10 * 1024 * 1024, true)?;
    let result = framemoor_core::run_with_readers(&f.plan, 4, |e| {
        Ok(
            if matches!(e, Event::VerificationProgress {phase,..} if phase == "verifying") {
                Control::Pause
            } else {
                Control::Continue
            },
        )
    });
    assert!(result.is_err());
    assert_eq!(framemoor_core::status(&f.plan)?.unwrap().status, "paused");
    assert!(!f.dst().exists());
    assert!(f.part(0).exists());
    framemoor_core::run_with_readers(&f.plan, 4, |_| Ok(Control::Continue))?;
    framemoor_core::verify_with_readers(&f.plan, 4)?;
    Ok(())
}

#[test]
fn concurrent_readback_and_final_pass_reject_corruption() -> Result<()> {
    for tamper_source in [false, true] {
        let f = Fixture::new(1024 * 1024, true)?;
        assert!(
            framemoor_core::run_with_readers(&f.plan, 8, |e| {
                if matches!(e, Event::BeforeVerify { file: 0 }) {
                    let path = if tamper_source {
                        f.plan.source_root.join("clip.bin")
                    } else {
                        f.part(0)
                    };
                    let mtime = fs::metadata(&path)?.modified()?;
                    let mut file = OpenOptions::new().write(true).open(path)?;
                    file.write_all(b"changed")?;
                    file.set_modified(mtime)?;
                }
                Ok(Control::Continue)
            })
            .is_err()
        );
        assert!(!f.dst().exists());
        assert_ne!(framemoor_core::status(&f.plan)?.unwrap().status, "complete");
    }
    let f = Fixture::new(1024, true)?;
    assert!(framemoor_core::run_with_readers(&f.plan, 4, |e| {
        if matches!(e, Event::VerificationProgress {phase,done:0,..} if phase == "final_verifying") {
            OpenOptions::new().write(true).open(f.dst())?.write_all(b"changed")?;
        }
        Ok(Control::Continue)
    }).is_err());
    assert_ne!(framemoor_core::status(&f.plan)?.unwrap().status, "complete");
    Ok(())
}

#[test]
fn sha256_standard_vectors_and_invalid_parallelism() -> Result<()> {
    let f = Fixture::new(0, false)?;
    assert_eq!(
        framemoor_core::hash_file(&f.plan.source_root.join("clip.bin"))?,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    fs::write(f.plan.source_root.join("clip.bin"), b"abc")?;
    assert_eq!(
        framemoor_core::hash_file(&f.plan.source_root.join("clip.bin"))?,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    for readers in [0, 9] {
        assert!(
            framemoor_core::run_with_readers(&f.plan, readers, |_| Ok(Control::Continue)).is_err()
        );
        assert!(!f.plan.destination_root.join(".card-ingest").exists());
        assert!(framemoor_core::verify_with_readers(&f.plan, readers).is_err());
    }
    Ok(())
}

fn unverified_fixture(size: usize) -> Result<Fixture> {
    let mut f = Fixture::new(size, false)?;
    f.plan = framemoor_core::prepare_with_settings(
        &f.plan.source_root,
        &f.plan.destination_root,
        f.plan.entries.iter().map(|e| e.spec.clone()).collect(),
        framemoor_core::verification::Settings {
            enabled: false,
            readers: Some(2),
        },
        |_, _, _| Ok(Control::Continue),
    )?;
    Ok(f)
}
#[test]
fn unchecked_copy_is_explicit_in_plan_report_and_never_claims_content_verification() -> Result<()> {
    let f = unverified_fixture(1024 * 1024)?;
    assert!(!f.plan.content_verification);
    assert!(f.plan.entries.iter().all(|e| e.sha256.is_empty()));
    let report = framemoor_core::run(&f.plan, |event| {
        assert!(!matches!(event, Event::BeforeVerify { .. }));
        if let Event::VerificationProgress { phase, .. } = event {
            assert!(!["verifying", "final_verifying"].contains(&phase.as_str()));
        }
        Ok(Control::Continue)
    })?;
    assert!(!report.content_verified);
    assert_eq!(
        fs::read(f.dst())?,
        fs::read(f.plan.source_root.join("clip.bin"))?
    );
    assert!(framemoor_core::verify(&f.plan).is_err());
    // Same-size content damage is intentionally outside unchecked mode's guarantee.
    OpenOptions::new()
        .write(true)
        .open(f.dst())?
        .write_all(b"changed")?;
    let resumed = f.run()?;
    assert!(!resumed.content_verified);
    assert_eq!(resumed.skipped_verified_files, 0);
    assert_eq!(resumed.skipped_unverified_files, 1);
    Ok(())
}
#[test]
fn unchecked_resume_restarts_partial_file_and_does_not_adopt_a_changed_plan() -> Result<()> {
    let f = unverified_fixture(20 * 1024 * 1024)?;
    assert!(
        framemoor_core::run(&f.plan, |e| Ok(if matches!(e, Event::Checkpoint { .. }) {
            Control::Pause
        } else {
            Control::Continue
        }))
        .is_err()
    );
    OpenOptions::new()
        .write(true)
        .open(f.part(0))?
        .write_all(b"damaged partial")?;
    let mut changed = f.plan.clone();
    changed.content_verification = true;
    changed.entries[0].sha256 = framemoor_core::hash_file(&changed.source_root.join("clip.bin"))?;
    assert!(framemoor_core::run(&changed, |_| Ok(Control::Continue)).is_err());
    let report = f.run()?;
    assert_eq!(report.resumed_bytes, 0);
    assert!(!report.content_verified);
    assert_eq!(
        framemoor_core::hash_file(&f.dst())?,
        framemoor_core::hash_file(&f.plan.source_root.join("clip.bin"))?
    );
    Ok(())
}
#[test]
fn unchecked_mode_retains_conflict_source_change_and_copy_error_checks() -> Result<()> {
    let f = unverified_fixture(64)?;
    fs::create_dir_all(f.dst().parent().unwrap())?;
    fs::write(f.dst(), b"existing")?;
    assert!(f.run().is_err());
    assert_eq!(fs::read(f.dst())?, b"existing");
    let f = unverified_fixture(64)?;
    fs::write(f.plan.source_root.join("clip.bin"), b"changed length")?;
    assert!(f.run().is_err());
    assert!(!f.dst().exists());
    let f = unverified_fixture(64)?;
    assert!(
        framemoor_core::run(&f.plan, |e| {
            if matches!(e, Event::BeforeWrite { .. }) {
                bail!("simulated write error")
            }
            Ok(Control::Continue)
        })
        .is_err()
    );
    assert!(!f.dst().exists());
    let state = framemoor_core::status(&f.plan)?.unwrap();
    assert_ne!(state.status, "complete");
    Ok(())
}
#[test]
fn old_plans_and_verification_settings_remain_verified_by_default() -> Result<()> {
    let f = Fixture::new(16, false)?;
    let json = serde_json::to_value(&f.plan)?;
    assert!(json.get("content_verification").is_none());
    let loaded: Plan = serde_json::from_value(json)?;
    assert!(loaded.content_verification);
    let settings: framemoor_core::verification::Settings = serde_json::from_str("{\"readers\":4}")?;
    assert!(settings.enabled);
    assert!(framemoor_core::verification::Settings::default().enabled);
    let report = f.run()?;
    assert!(report.content_verified);
    let mut legacy_report = serde_json::to_value(report)?;
    legacy_report
        .as_object_mut()
        .unwrap()
        .remove("content_verified");
    assert!(serde_json::from_value::<framemoor_core::RunReport>(legacy_report)?.content_verified);
    Ok(())
}
