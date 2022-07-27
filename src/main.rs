use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use std::process;
use tracing::{debug, error};
use tracing_subscriber::EnvFilter;

mod args;
mod mirror;

use args::Arguments;
use mirror::{Evaluation, Filter, Mirrors, MirrorsStatus, Statistics, ToPacmanMirrorList};

fn init_log() -> Result<()> {
    let filter = match EnvFilter::try_from_env("RUST_LOG") {
        Ok(f) => f,
        Err(_) => EnvFilter::try_new("pacman_mirrorup=warn")?,
    };

    if let Err(err) = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .try_init()
    {
        bail!("Failed to initialize tracing subscriber: {err}");
    }

    Ok(())
}

fn run_app() -> Result<()> {
    let arguments = Arguments::parse();
    init_log().context("Failed to initialize logging")?;
    debug!("Run with {:?}", arguments);

    if let Some(output_file) = &arguments.output_file {
        if output_file.exists() {
            return Err(anyhow!(
                "`{}` is exist.",
                output_file
                    .to_str()
                    .context("Failed to convert path to string")?
            ));
        }
    }

    if let Some(stats_file) = &arguments.stats_file {
        if stats_file.exists() {
            return Err(anyhow!(
                "`{}` is exist.",
                stats_file
                    .to_str()
                    .context("Failed to convert path to string")?
            ));
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
    let best_synced_mirrors: Mirrors = mirrors_status.best_synced_mirrors()?;
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

fn main() {
    process::exit(match run_app() {
        Ok(_) => 0,
        Err(err) => {
            error!("{}", err);
            1
        }
    });
}
