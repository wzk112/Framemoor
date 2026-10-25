use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use framemoor_core::{Control, CopySpec, Event};
use std::{fs::File, io::Write, path::PathBuf};

#[derive(Parser)]
#[command(
    version,
    name = "framemoor",
    about = "Framemoor / 泊影 — verified photo and video import"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all non-hidden regular files; preserve unknown formats and directory structure.
    Scan {
        #[arg(long)]
        source: PathBuf,
    },
    /// Prepare a read-only-source copy plan. Destination folder must already exist.
    Plan {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        destination: PathBuf,
        #[arg(long)]
        manifest: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Execute or resume a saved plan. Never replace an existing destination file.
    Run {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        events: bool,
    },
    /// Inspect recorded state (not a substitute for re-verification).
    Status {
        #[arg(long)]
        plan: PathBuf,
    },
    /// Read source and destination again and verify all SHA-256 digests.
    Verify {
        #[arg(long)]
        plan: PathBuf,
    },
}

fn main() {
    if let Err(error) = execute() {
        eprintln!(
            "{}",
            serde_json::json!({"ok":false,"error":format!("{error:#}")})
        );
        std::process::exit(1);
    }
}

fn execute() -> Result<()> {
    match Cli::parse().command {
        Command::Scan { source } => println!(
            "{}",
            serde_json::to_string_pretty(&framemoor_core::discover(&source)?)?
        ),
        Command::Plan {
            source,
            destination,
            manifest,
            out,
        } => {
            let files: Vec<CopySpec> = if let Some(path) = manifest {
                serde_json::from_reader(File::open(path)?).context("Invalid selection manifest")?
            } else {
                framemoor_core::discover(&source)?
            };
            let plan = framemoor_core::prepare(&source, &destination, files)?;
            framemoor_core::save_plan(&out, &plan)?;
            println!(
                "{}",
                serde_json::json!({"ok":true,"id":plan.id,"files":plan.entries.len(),"bytes":plan.entries.iter().map(|e| e.bytes).sum::<u64>(),"plan":out})
            );
        }
        Command::Run { plan, events } => {
            let plan = framemoor_core::load_plan(&plan)?;
            let report = framemoor_core::run(&plan, |event| {
                if events
                    && !matches!(
                        event,
                        Event::BeforeWrite { .. }
                            | Event::BeforeVerify { .. }
                            | Event::BeforePublish { .. }
                    )
                {
                    println!("{}", serde_json::to_string(event)?);
                    std::io::stdout().flush()?;
                }
                Ok(Control::Continue)
            })?;
            println!("{}", serde_json::to_string(&report)?);
        }
        Command::Status { plan } => {
            let plan = framemoor_core::load_plan(&plan)?;
            let status = framemoor_core::status(&plan)?;
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        Command::Verify { plan } => println!(
            "{}",
            serde_json::to_string(&framemoor_core::verify(&framemoor_core::load_plan(&plan)?)?)?
        ),
    }
    Ok(())
}
