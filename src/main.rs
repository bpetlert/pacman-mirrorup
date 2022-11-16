use anyhow::{bail, Context, Result};
use clap::Parser;
use mimalloc::MiMalloc;
use std::io;
use tracing::debug;
use tracing_subscriber::EnvFilter;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod args;
mod mirror;

use args::Arguments;
use mirror::{Evaluation, Filter, Mirrors, MirrorsStatus, Statistics, ToPacmanMirrorList};

fn main() -> Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or(EnvFilter::try_new("pacman_mirrorup=info")?);
    if let Err(err) = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .with_writer(io::stderr)
        .try_init()
    {
        bail!("Failed to initialize tracing subscriber: {err}");
    }

    let arguments = Arguments::parse();
    debug!("Run with {:?}", arguments);

    if let Some(output_file) = &arguments.output_file {
        if output_file.exists() {
            bail!(
                "`{}` is exist.",
                output_file
                    .to_str()
                    .context("Failed to convert path to string")?
            );
        }
    }

    if let Some(stats_file) = &arguments.stats_file {
        if stats_file.exists() {
            bail!(
                "`{}` is exist.",
                stats_file
                    .to_str()
                    .context("Failed to convert path to string")?
            );
        }
    }

    rayon::ThreadPoolBuilder::new()
        .num_threads(arguments.threads)
        .build_global()
        .context(format!(
            "Failed to set number of rayon threads to {}",
            arguments.threads
        ))?;

    let mirrors_status: MirrorsStatus = MirrorsStatus::from_online_json(&arguments.source_url)?;
    let best_synced_mirrors: Mirrors =
        mirrors_status.best_synced_mirrors(arguments.max_check, arguments.exclude)?;
    let best_mirrors: Mirrors =
        best_synced_mirrors.evaluate(arguments.mirrors, arguments.target_db)?;

    if let Some(output_file) = &arguments.output_file {
        // Write to file
        best_mirrors.to_mirrorlist_file(output_file, &arguments.source_url)?;
    } else {
        // Write to stdout
        print!("{}", &best_mirrors.to_pacman_mirror_list()?);
    }

    if let Some(stats_file) = &arguments.stats_file {
        best_mirrors.to_csv(stats_file)?;
    }

    Ok(())
}
