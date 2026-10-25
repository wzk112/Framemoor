//! Read-only scan benchmark: source, ExifTool, output JSON, optional reader count.
use anyhow::{Context, Result, ensure};
use std::{fs, io::Write, path::Path, time::Instant};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let source = args.get(1).context("source folder required")?;
    let tool = args.get(2).context("ExifTool path required")?;
    let output = args.get(3).context("output JSON path required")?;
    let readers = args
        .get(4)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(framemoor_core::workflow::DEFAULT_SCAN_READERS);
    let source_root = fs::canonicalize(source)?;
    let output_parent = fs::canonicalize(
        Path::new(output)
            .parent()
            .context("output parent required")?,
    )?;
    ensure!(
        !output_parent.starts_with(&source_root),
        "The report must be outside the source folder"
    );
    let start = Instant::now();
    let catalog = framemoor_core::workflow::scan_observed(
        Path::new(source),
        Path::new(tool),
        readers,
        |inventory| {
            eprintln!(
                "inventory: {:.3}s, {} files",
                start.elapsed().as_secs_f64(),
                inventory.file_count
            )
        },
        |done, total| {
            eprintln!("{:.3}s {done}/{total}", start.elapsed().as_secs_f64());
        },
    )?;
    println!(
        "{} files, {} groups, {:.3} seconds",
        catalog.file_count,
        catalog.groups.len(),
        start.elapsed().as_secs_f64()
    );
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?
        .write_all(&serde_json::to_vec_pretty(&catalog)?)?;
    Ok(())
}
