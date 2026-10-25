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
