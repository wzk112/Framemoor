# Implementation notes

## Code map

| Path | Responsibility |
| --- | --- |
| `src/lib.rs` | Frozen plans, copying, recovery, verification, task and media-group state |
| `src/workflow.rs` | Metadata, grouping, time interpretation, selection, naming, preview |
| `src/platform.rs` | Volume/file identity, locks, flushes, exclusive publication |
| `src/paths.rs` | Cross-platform path validation and conflicts |
| `src/verification.rs` | Bounded read concurrency and hardware-aware defaults |
| `src/main.rs` | CLI |
| `desktop/src/` | React UI, translations, preferences, and rule editing |
| `desktop/src-tauri/src/` | Native commands, progress, local persistence, and sleep prevention |

## Scan and preview

Each scan enumerates non-hidden regular files without a persistent cache. Extension-based classification is provisional; metadata must finish before an import preview can be frozen. ExifTool processes bounded batches (up to 64 CR3 files or 8 other files); the desktop uses at most four metadata workers, with a lower limit on smaller systems. CR3 uses ExifTool's fast path with a full-read fallback when key fields are absent. Other formats use full metadata reads.

After scanning, volume identity, additions/removals, size, and modification time are rechecked. These checks do not detect all possible content changes; content verification is a separate import stage. Same-directory RAW/JPEG and recognized sidecars are grouped heuristically. Complete professional camera-card relationships are not yet validated across vendors.

Naming previews and frozen import previews share path calculation. The UI discards stale responses. A directory-tree preview is not an executable plan; importing uses the backend's frozen plan. Time adjustments affect selection and naming, not embedded file metadata.

## Copy and recovery

The destination's `.card-ingest` folder holds frozen plans, journals, locks, and staging files. Existing destination files are never replaced. Source/destination roots must be separate, existing directories. Unsafe paths and collisions are rejected.

On macOS APFS, `renamex_np(RENAME_EXCL)` publishes files without replacement. Where this is unsupported, including tested macOS ExFAT volumes, the fallback exclusively creates the destination and copies from staging. It verifies the copy when content verification is enabled. The final path may be visible while state is still `publishing`; only task completion establishes success.

Fallback recovery checks recorded ownership and file identity before resuming. If a crash occurs between file creation and saving ownership, recovery conservatively reports a conflict instead of deleting or replacing an unowned file. Identity must still be verifiable after remounting. Modification timestamps are set before recording identity because filesystem timestamps can affect identity observations.

Windows publication uses `MoveFileExW` without replacement or cross-volume-copy flags. Windows does not implement general directory flushing here. No claim of certified power-loss durability is made.

## Verification modes

With verification enabled, full SHA-256 reads check sources, destinations, resumed prefixes, and final outputs at the relevant stages. SHA-256 on ARM uses runtime-detected hardware instructions with a software fallback. No sampled hashes or persisted checksum cache are used.

The UI offers automatic, 1, 2, 4, or 8 verification readers. Automatic mode conservatively selects one for low CPU/memory, same-volume, or unknown-volume cases, otherwise two. This is not a disk benchmark; different volumes need not be separate physical devices. Writes, flushes, journals, and publication remain sequential. Workers use 8 MiB buffers and cancellation checkpoints; a blocked OS read cannot be forcibly interrupted.

Disabling verification skips source hashes, target readback, and prefix hash comparisons. Size, timestamp, volume, path, free-space, ownership, and I/O checks remain. Incomplete files restart from zero. Same-size corruption can go undetected; all outputs remain explicitly unverified. The mode is frozen in each task. Older plans and reports use the previous verified-mode interpretation for compatibility.

## Local state and platform behavior

The application identifier remains `dev.cardingest.desktop` and the journal folder remains `.card-ingest` to preserve settings and resumable tasks from the earlier Card Ingest name. `FRAMEMOOR_*` environment variables take precedence over legacy names.

Sleep prevention uses a macOS IOKit idle-sleep assertion or a Windows worker thread's `SetThreadExecutionState`. It releases on pause, failure, or completion and does not prevent explicit sleep, closing the lid, or battery protection. Notification API success confirms submission, not that a banner appeared.

Browser fixtures exercise UI behavior through mock IPC. They do not perform native copies. Core tests cover paths, resumed copies, source changes, verification failures, and task recovery; physical power loss, unplugging readers, network volumes, and full Windows GUI behavior still require separate testing.
