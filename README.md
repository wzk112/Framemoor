# Framemoor · 泊影

<img src="desktop/src-tauri/icons/app.svg" width="80" alt="Framemoor icon">

[简体中文](README.zh-CN.md) · English

A desktop tool for importing photos and videos from memory cards and folders. Choose files, configure destination folders and filenames, preview the result, then copy with SHA-256 verification.

**Development preview (0.2).** macOS has been tested with Canon EOS R5 Mark II media. CI tests the installed Windows x64 app through real WebView2 using synthetic media. Physical Windows PCs, card readers, and other camera formats still need validation.

## Download

[GitHub Releases](https://github.com/wzk112/Framemoor/releases) provide macOS Apple Silicon (`.dmg`) and Windows x64 (`-setup.exe`) packages, corresponding source, and SHA-256 checksums. Intel Mac and Windows ARM64 installers are not provided.

- **macOS 13+:** open the DMG and drag the app to Applications. The app is ad-hoc signed, without Apple Developer ID signing or notarization; Gatekeeper may block the first launch.
- **Windows 10 1903+ / Windows 11:** run the installer. WebView2 is required; the installer can download it when missing. The installer is unsigned, so Windows may show an unknown-publisher or SmartScreen warning.

See [release packaging and validation](docs/release-validation.md) for the tested environments and limitations. These are preview releases; keep another copy of important source media.

## Features

- **Import views:** the default simple view groups source, destination, selection and preset in one page, followed by confirmation and import. Switch to the detailed five-step view without changing the draft. Advanced rules remain editable.
- **Source and selection:** mounted storage and folder selection, device refresh, recent paths, date ranges, media types, and video codec filters. Active filters and time adjustments remain visible in the simple view and can be cleared.
- **Folders and filenames:** editable directory levels, separate photo/video naming rules, clickable variables with help, built-in templates, and saved presets. Presets store rules only; applying one clears the shoot description.
- **Preview:** switch between the destination directory tree and file list, filter files with metadata warnings, and review capture times, available space, and conflicts. Changes require a fresh preview before import.
- **Time handling:** camera time, recorded UTC offsets, IANA time zones, and manual minute adjustments. Future dates and time-zone differences are flagged. Embedded media metadata is not edited.
- **Copying:** read-only source access, no overwrite of existing destination files, pause/resume, task history, JSON reports, and opening the completed task’s destination folder. Recognized companion files are grouped with their media.
- **Settings:** Chinese/English, system/light/dark appearance, default destination and preset, completion notifications, sleep prevention during import, and verification concurrency.

Scanning first lists files and classifies them by extension, then reads metadata in the background. Codec filters and import previews wait for metadata to finish. Scanning can be cancelled; a new scan waits for the previous readers to stop. Every scan reads the source again; there is no persistent metadata or checksum cache.

## Verification

SHA-256 verification is **enabled by default**. It reads source content, checks copied content, and performs additional source/destination checks during the task. Parallel reads are configurable; writes and task journal updates remain sequential.

Verification can be disabled in **Settings → Performance**. This can improve speed, depending on storage, but reduces reliability: corruption that preserves file size may go undetected. A persistent warning is shown, and the task, history, notification, and report are marked **unverified**. Interrupted files in this mode restart from the beginning. Each task retains the verification mode selected when its plan was created.

The destination's `.card-ingest` folder stores task state and temporary files; do not move or edit it during an import. A file appearing at its final path does not by itself mean the task is complete. On macOS ExFAT, publishing a file may require an additional exclusive copy, increasing time and temporary space requirements. See [implementation notes](docs/architecture.md).

## Build from source

Requirements: Git, Python 3, Node.js 22.12+ (22 LTS or a newer supported LTS), Rust **1.90.0**, and the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/). macOS requires version 13 or later and Xcode command line tools; Windows requires version 10 1903 or newer, MSVC build tools with the Windows SDK, and WebView2. Toolchain and dependency lockfiles are checked in.

```sh
git clone https://github.com/wzk112/Framemoor.git
cd Framemoor/desktop
npm ci
python3 scripts/prepare-resources.py
python3 scripts/prepare-notices.py
npm run tauri dev
```

The resource script retrieves **ExifTool 13.59** at a fixed source revision and retains its license files. It does not read your media. Use `--source` to provide an existing checkout at the required revision. macOS uses system Perl to run the bundled ExifTool script.

Build a macOS Release app and DMG from `desktop`:

```sh
python3 scripts/prepare-notices.py
npm run tauri build -- --ci --bundles app,dmg
```

Packages are created in `src-tauri/target/release/bundle/`, for the build machine's architecture. `prepare-notices.py` defaults to the Rust host target; pass `--target <triple>` when cross-compiling. The app includes ExifTool, dependency notices and source revision information. For a development app, use `--debug --bundles app`. Both builds use ad-hoc signing and are not notarized.

On Windows x64, run from `desktop`:

```powershell
npm ci
python scripts/prepare-resources.py
python scripts/prepare-windows-resources.py
python scripts/prepare-notices.py
npm run tauri dev -- --config src-tauri/tauri.windows.conf.json
# Release-mode NSIS installer:
npm run tauri build -- --bundles nsis --config src-tauri/tauri.windows.conf.json
```

The Windows resource script downloads the complete ExifTool 13.59 x64 package and verifies its pinned SHA-256. It includes the supporting Perl files and licenses. It adds a UTF-8 process manifest to the CC0 launcher so it can load its runtime from Chinese installation paths; the system locale is not changed. `prepare-notices.py` collects dependency notices for the installer. The installer is created in `desktop/src-tauri/target/release/bundle/nsis/` and installs for the current user. It is unsigned, so Windows may show an unknown-publisher or SmartScreen warning.

Successful [CI runs](https://github.com/wzk112/Framemoor/actions/workflows/ci.yml) provide **Framemoor-macos-ARM64** and **Framemoor-windows-x64** artifacts containing the installer, corresponding source, notices, and SHA-256 values. Windows test evidence is in **windows-native-test-results**. CI artifacts expire; published downloads are kept on GitHub Releases. See [Windows validation](docs/windows-validation.md).

Settings and task history stay in the platform application-data directory under `dev.cardingest.desktop`. For isolated development, set `FRAMEMOOR_DATA_DIR`; `FRAMEMOOR_EXIFTOOL` overrides the metadata tool path. Legacy `CARD_INGEST_*` names remain supported. The app processes media locally and does not upload media or copy logs.

## Command line

From the repository root:

```sh
cargo build --release --locked
# Add target/release to PATH, or invoke the binary there directly.
framemoor scan --source /path/to/source > selection.json
framemoor plan --source /path/to/source --destination /path/to/destination \
  --manifest selection.json --out import-plan.json
framemoor run --plan import-plan.json --events
framemoor status --plan import-plan.json
framemoor verify --plan import-plan.json
```

Inspect and edit `selection.json` before planning; [an example](examples/selection.json) describes the format. Both roots must already exist and must not be nested. `plan` hashes the selected source files. `run` also resumes an existing plan; `status` reads recorded state, while `verify` rereads content. Omitting `--manifest` selects all non-hidden regular files and preserves relative paths. CLI scanning lists files; the desktop metadata workflow is separate.

## Development and validation

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cd desktop
npm ci
npm test
npm run build
npx playwright install chromium
npm run test:ui
python3 scripts/prepare-resources.py
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

[CI](https://github.com/wzk112/Framemoor/actions) checks the Rust core on macOS/Windows, the frontend and browser interactions, and both desktop backends. The Windows job additionally installs the packaged app, exercises its real UI and IPC, and uninstalls it. Browser tests use mock IPC and synthetic fixtures; they do not validate actual storage or native dialogs. Camera-brand recognition is not certification of every model, card layout, or sidecar relationship. Hardware disconnects, power loss, network volumes, native folder dialogs, and Windows 10/11 device coverage still require further testing.

All changes go through pull requests. `main` requires successful checks, resolved conversations, and a linear history; direct pushes, force pushes, and branch deletion are blocked, including for administrators. See [CONTRIBUTING.md](CONTRIBUTING.md).

Unicode paths, content encodings and failure-test boundaries are documented in [Encoding and failure coverage](docs/encoding-validation.md).

## License

Copyright © 2026 wzk112 and contributors. Framemoor's original code is licensed under **GPL-3.0-only**; see [LICENSE](LICENSE). Commercial use and paid distribution are allowed subject to the license, including its source-distribution obligations. Dependencies retain their own licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
