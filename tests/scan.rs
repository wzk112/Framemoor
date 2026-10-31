// A deterministic metadata process exercises the scanner without requiring a
// particular camera fixture or a Windows ExifTool installation.
#![cfg(unix)]
use framemoor_core::workflow;
use std::{fs, path::Path};

fn tool(directory: &Path) -> std::path::PathBuf {
    let path = directory.join("metadata.pl");
    fs::write(
        &path,
        r#"
use strict; use warnings; use JSON::PP;
my $fast = grep { $_ eq '-fast' } @ARGV;
my @records;
for my $path (grep { -f $_ } @ARGV) {
    open my $fh, '<', $path or die $!;
    local $/; my $date = <$fh>;
    my $type = $path =~ /\.cr3$/i ? 'CR3' : 'MP4';
    die 'Video must use full metadata reader' if $fast && $type eq 'MP4';
    my $fallback = $date =~ s/^FALLBACK://;
    my $record = {'SourceFile'=>$path, 'File:FileType'=>$type,
        'IFD0:Model'=>'Canon EOS R5m2', 'Canon:TimeZone'=>'+10:00',
        'Track1:CompressorID'=>'hvc1'};
    $record->{'ExifIFD:DateTimeOriginal'}=$date unless $fallback && $fast;
    push @records, $record;
}
print encode_json(\@records);
"#,
    )
    .unwrap();
    path
}

#[test]
fn every_scan_rereads_even_when_size_and_modified_time_are_preserved() {
    let source = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    let tool = tool(tools.path());
    let file = source.path().join("one.CR3");
    fs::write(&file, "2026:09:19 18:00:00").unwrap();
    let modified = fs::metadata(&file).unwrap().modified().unwrap();
    let first = workflow::scan(source.path(), &tool, |_, _| {}).unwrap();
    fs::write(&file, "2026:10:06 19:00:00").unwrap();
    fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let second = workflow::scan(source.path(), &tool, |_, _| {}).unwrap();
    assert_eq!(
        first.groups[0].files[0].captured.as_deref(),
        Some("2026:09:19 18:00:00")
    );
    assert_eq!(
        second.groups[0].files[0].captured.as_deref(),
        Some("2026:10:06 19:00:00")
    );
    fs::remove_file(&file).unwrap();
    fs::write(source.path().join("new.MP4"), "2026:10:26 10:00:00").unwrap();
    let third = workflow::scan(source.path(), &tool, |_, _| {}).unwrap();
    assert_eq!(third.file_count, 1);
    assert_eq!(third.groups[0].files[0].path, "new.MP4");
    assert_eq!(third.groups[0].files[0].codec, "HEVC");
}

#[test]
fn parallel_batches_preserve_all_files_and_fall_back_for_missing_cr3_metadata() {
    let source = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    let tool = tool(tools.path());
    for i in 0..137 {
        fs::write(
            source.path().join(format!("photo{i:03}.CR3")),
            "2026:10:06 18:00:00",
        )
        .unwrap();
    }
    fs::write(
        source.path().join("fallback.cr3"),
        "FALLBACK:2026:10:25 19:00:00",
    )
    .unwrap();
    fs::write(source.path().join("video.MP4"), "2026:10:25 20:00:00").unwrap();
    let mut counts = Vec::new();
    let catalog = workflow::scan(source.path(), &tool, |done, total| {
        assert_eq!(total, 139);
        counts.push(done);
    })
    .unwrap();
    assert_eq!(catalog.file_count, 139);
    assert_eq!(catalog.groups.len(), 139);
    assert!(counts.windows(2).all(|p| p[0] <= p[1]));
    assert_eq!(counts.last(), Some(&139));
    let fallback = catalog
        .groups
        .iter()
        .flat_map(|g| &g.files)
        .find(|f| f.path == "fallback.cr3")
        .unwrap();
    assert_eq!(fallback.captured.as_deref(), Some("2026:10:25 19:00:00"));
    assert_eq!(fallback.offset.as_deref(), Some("+10:00"));
    assert!(fallback.issues.is_empty());
}

#[test]
fn changing_a_file_or_adding_a_file_during_scan_rejects_the_catalog() {
    for add in [false, true] {
        let source = tempfile::tempdir().unwrap();
        let tools = tempfile::tempdir().unwrap();
        let tool = tool(tools.path());
        fs::write(source.path().join("one.CR3"), "2026:10:06 18:00:00").unwrap();
        let mut changed = false;
        let result = workflow::scan(source.path(), &tool, |done, _| {
            if done > 0 && !changed {
                changed = true;
                fs::write(
                    source.path().join(if add { "new.CR3" } else { "one.CR3" }),
                    "changed",
                )
                .unwrap();
            }
        });
        assert!(result.unwrap_err().to_string().contains("source_changed"));
    }
}

fn tracked_tool(directory: &Path) -> std::path::PathBuf {
    let path = tool(directory);
    let original = fs::read_to_string(&path).unwrap();
    let tracking = r#"
use Fcntl qw(:flock SEEK_SET); use FindBin qw($RealBin);
my $started=0;
sub track {
    my ($delta)=@_;
    open my $fh, '+>>', "$RealBin/readers.txt" or die $!;
    flock($fh, LOCK_EX) or die $!; seek($fh,0,SEEK_SET);
    my $text=<$fh> || '0 0'; my ($active,$peak)=split / /,$text;
    $active+=$delta; $peak=$active if $active>$peak;
    seek($fh,0,SEEK_SET); truncate($fh,0); print $fh "$active $peak"; close $fh;
}
track(1); $started=1;
END { track(-1) if $started; }
select(undef,undef,undef,0.10);
"#;
    fs::write(
        &path,
        format!("{tracking}\n{original}").replace(
            "my $date = <$fh>;",
            "my $date = <$fh>; die 'Injected reader failure' if $date eq 'FAIL';",
        ),
    )
    .unwrap();
    path
}

#[test]
fn reader_limits_preserve_exact_results_and_release_processes_after_errors() {
    let source = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    let tool = tracked_tool(tools.path());
    for i in 0..513 {
        fs::write(
            source.path().join(format!("p{i:03}.CR3")),
            "2026:10:06 18:00:00",
        )
        .unwrap();
    }
    let mut reference = None;
    for readers in [1, 4, 8] {
        fs::write(tools.path().join("readers.txt"), "0 0").unwrap();
        let catalog =
            workflow::scan_with_readers(source.path(), &tool, readers, |_, _| {}).unwrap();
        let mut value = serde_json::to_value(catalog).unwrap();
        value.as_object_mut().unwrap().remove("id");
        if let Some(ref expected) = reference {
            assert_eq!(&value, expected);
        } else {
            reference = Some(value);
        }
        let counts: Vec<usize> = fs::read_to_string(tools.path().join("readers.txt"))
            .unwrap()
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(counts[0], 0);
        assert!(counts[1] <= readers);
        if readers > 1 {
            assert!(counts[1] > 1);
        }
    }
    fs::write(source.path().join("p000.CR3"), "FAIL").unwrap();
    fs::write(tools.path().join("readers.txt"), "0 0").unwrap();
    assert!(workflow::scan_with_readers(source.path(), &tool, 4, |_, _| {}).is_err());
    assert!(
        fs::read_to_string(tools.path().join("readers.txt"))
            .unwrap()
            .starts_with("0 ")
    );
    assert!(workflow::scan_with_readers(source.path(), &tool, 0, |_, _| {}).is_err());
    assert!(workflow::scan_with_readers(source.path(), &tool, 9, |_, _| {}).is_err());
}

#[test]
fn preliminary_inventory_precedes_reads_and_is_not_a_catalog() {
    let source = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    let tool = tracked_tool(tools.path());
    for name in ["a.CR3", "b.MP4", "b.CPF"] {
        fs::write(source.path().join(name), "2026:10:06 18:00:00").unwrap();
    }
    let mut inventory_seen = false;
    let catalog = workflow::scan_observed(
        source.path(),
        &tool,
        4,
        |inventory| {
            assert!(!tools.path().join("readers.txt").exists());
            assert_eq!(
                (
                    inventory.file_count,
                    inventory.photo_groups,
                    inventory.video_groups,
                    inventory.other_groups
                ),
                (3, 1, 1, 0)
            );
            let value = serde_json::to_value(inventory).unwrap();
            assert!(value.get("id").is_none());
            assert!(value.get("groups").is_none());
            inventory_seen = true;
        },
        |_, _| {},
    )
    .unwrap();
    assert!(inventory_seen);
    assert_eq!(catalog.file_count, 3);
}

#[test]
fn cancellation_before_reads_and_after_final_progress_never_returns_a_catalog() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let source = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    let tool = tracked_tool(tools.path());
    fs::write(source.path().join("中文.CR3"), "2026:10:06 18:00:00").unwrap();
    let cancelled = AtomicBool::new(true);
    let error = workflow::scan_cancellable(
        source.path(),
        &tool,
        4,
        &cancelled,
        |_| panic!("Cancelled scan must not publish an inventory"),
        |_, _| {},
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "scan_cancelled");
    assert!(!tools.path().join("readers.txt").exists());

    cancelled.store(false, Ordering::SeqCst);
    let error = workflow::scan_cancellable(
        source.path(),
        &tool,
        4,
        &cancelled,
        |_| cancelled.store(true, Ordering::SeqCst),
        |_, _| {},
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "scan_cancelled");
    assert!(!tools.path().join("readers.txt").exists());

    cancelled.store(false, Ordering::SeqCst);
    let error = workflow::scan_cancellable(
        source.path(),
        &tool,
        4,
        &cancelled,
        |_| {},
        |done, total| {
            if done == total {
                cancelled.store(true, Ordering::SeqCst);
            }
        },
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "scan_cancelled");
    assert!(
        fs::read_to_string(tools.path().join("readers.txt"))
            .unwrap()
            .starts_with("0 ")
    );
    // Cancellation is per request; a fresh scan still returns all source files.
    assert_eq!(
        workflow::scan(source.path(), &tool, |_, _| {})
            .unwrap()
            .file_count,
        1
    );
}

#[test]
fn cancellation_stops_all_active_metadata_processes_before_returning() {
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        time::{Duration, Instant},
    };
    let source = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    let tool = tools.path().join("slow-metadata.pl");
    fs::write(
        &tool,
        r#"
use strict; use warnings; use FindBin qw($RealBin); use Fcntl qw(:flock);
open my $pids, '>>', "$RealBin/pids.txt" or die $!;
flock($pids, LOCK_EX) or die $!; print $pids "$$\n"; close $pids;
$|=1; print ' ' x 131072; print STDERR ' ' x 131072;
sleep 60; print '[]';
"#,
    )
    .unwrap();
    for i in 0..32 {
        fs::write(source.path().join(format!("视频{i:02}.MP4")), b"fixture").unwrap();
    }
    let cancelled = AtomicBool::new(false);
    let started = Instant::now();
    let pids = std::thread::scope(|scope| {
        let canceller = scope.spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(5);
            let pids = loop {
                let pids: Vec<i32> = fs::read_to_string(tools.path().join("pids.txt"))
                    .unwrap_or_default()
                    .lines()
                    .filter_map(|line| line.parse().ok())
                    .collect();
                if pids.len() == 4 || Instant::now() >= deadline {
                    break pids;
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            cancelled.store(true, Ordering::SeqCst);
            pids
        });
        let error =
            workflow::scan_cancellable(source.path(), &tool, 4, &cancelled, |_| {}, |_, _| {})
                .unwrap_err();
        assert_eq!(error.to_string(), "scan_cancelled");
        canceller.join().unwrap()
    });
    assert_eq!(
        pids.len(),
        4,
        "Fixture must exercise all four active readers"
    );
    assert!(started.elapsed() < Duration::from_secs(10));
    for pid in pids {
        assert_eq!(
            unsafe { libc::kill(pid, 0) },
            -1,
            "Metadata child {pid} survived cancellation"
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
    assert_eq!(fs::read_dir(source.path()).unwrap().count(), 32);
    assert_eq!(
        fs::read(source.path().join("视频00.MP4")).unwrap(),
        b"fixture"
    );
}

#[test]
fn cancellation_after_launcher_exit_stops_helper_holding_output_pipes() {
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        time::{Duration, Instant},
    };
    let source = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    fs::write(source.path().join("one.MP4"), b"fixture").unwrap();
    let tool = tools.path().join("launcher.pl");
    fs::write(
        &tool,
        r#"
use strict; use warnings; use FindBin qw($RealBin);
my $launcher=$$;
my $child=fork(); die $! unless defined($child);
exit 0 if $child;
open my $pid, '>', "$RealBin/helper.txt" or die $!;
print $pid "$launcher $$"; close $pid;
$|=1; print ' ' x 131072; print STDERR ' ' x 131072;
sleep 15; print '[]';
"#,
    )
    .unwrap();
    let cancelled = AtomicBool::new(false);
    let started = Instant::now();
    let helper = std::thread::scope(|scope| {
        let canceller = scope.spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(5);
            let helper = loop {
                let pids: Vec<i32> = fs::read_to_string(tools.path().join("helper.txt"))
                    .unwrap_or_default()
                    .split_whitespace()
                    .filter_map(|v| v.parse().ok())
                    .collect();
                // Wait until the scanner has reaped the launcher. The helper
                // still owns the inherited pipes when cancellation is issued.
                if pids.len() == 2 && unsafe { libc::kill(pids[0], 0) } == -1 {
                    break Some(pids[1]);
                }
                if Instant::now() >= deadline {
                    break None;
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            cancelled.store(true, Ordering::SeqCst);
            helper
        });
        let error =
            workflow::scan_cancellable(source.path(), &tool, 1, &cancelled, |_| {}, |_, _| {})
                .unwrap_err();
        assert_eq!(error.to_string(), "scan_cancelled");
        canceller
            .join()
            .unwrap()
            .expect("Launcher must exit before cancellation")
    });
    assert!(started.elapsed() < Duration::from_secs(10));
    let deadline = Instant::now() + Duration::from_secs(2);
    while unsafe { libc::kill(helper, 0) } == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        unsafe { libc::kill(helper, 0) },
        -1,
        "Orphan metadata helper survived cancellation"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
    assert_eq!(fs::read(source.path().join("one.MP4")).unwrap(), b"fixture");
}
