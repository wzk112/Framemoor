//! Read-only source hashing benchmark; does not copy, publish, or save a plan.
use anyhow::{Context, Result};
use framemoor_core::{Control, CopySpec};
use std::{path::PathBuf, time::Instant};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let source = PathBuf::from(args.get(1).context("source")?);
    let destination = PathBuf::from(
        args.get(2)
            .context("existing scratch folder outside source")?,
    );
    let readers: usize = args.get(3).context("readers")?.parse()?;
    let specs: Vec<CopySpec> = framemoor_core::discover(&source)?
        .into_iter()
        .filter(|s| s.source.to_ascii_lowercase().ends_with(".cr3"))
        .take(32)
        .collect();
    let started = Instant::now();
    let plan =
        framemoor_core::prepare_with_readers(&source, &destination, specs, readers, |_, _, _| {
            Ok(Control::Continue)
        })?;
    println!(
        "{}",
        serde_json::json!({"readers":readers,"seconds":started.elapsed().as_secs_f64(),"recommendation":framemoor_core::verification::recommendation(&source,&destination),"files":plan.entries.len(),"bytes":plan.entries.iter().map(|e|e.bytes).sum::<u64>(),"hashes":plan.entries.iter().map(|e|(&e.spec.source,&e.sha256)).collect::<Vec<_>>()})
    );
    Ok(())
}
