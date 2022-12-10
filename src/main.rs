mod args;
mod exclude;
mod mirror;

use std::{io, process::ExitCode};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use mimalloc::MiMalloc;
use tracing::{debug, error};
use tracing_subscriber::EnvFilter;

use crate::{
    args::Arguments,
    exclude::{merge_exclude_mirror_list, read_exclude_from},
    mirror::{Evaluation, Filter, Mirrors, MirrorsStatus, Statistics, ToPacmanMirrorList},
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn run() -> Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or(EnvFilter::try_new("pacman_mirrorup=info")?);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .with_writer(io::stderr)
        .try_init()
        .map_err(|err| anyhow!("{err:#}"))
        .context("Failed to initialize tracing subscriber")?;

    let arguments = Arguments::parse();
    debug!("Run with {:?}", arguments);

    if let Some(output_file) = &arguments.output_file {
        if output_file.exists() {
            bail!("`{}` is exist.", output_file.display());
        }
    }

    if let Some(stats_file) = &arguments.stats_file {
        if stats_file.exists() {
            bail!("`{}` is exist.", stats_file.display());
        }
    }

    rayon::ThreadPoolBuilder::new()
        .num_threads(arguments.threads)
        .build_global()
        .with_context(|| {
            format!(
                "Failed to set number of rayon threads to {}",
                arguments.threads
            )
        })?;

    // Merge all excluded mirrors from --exclude and --exclude-from option
    let excluded_mirrors: Option<Vec<String>> = {
        let list1 = arguments.exclude.unwrap_or_default();
        let list2 = match arguments.exclude_from {
            Some(exclude_file) => read_exclude_from(&exclude_file)?,
            None => Vec::new(),
        };
        merge_exclude_mirror_list(vec![list1, list2])?
    };
    debug!("Excluded mirrors: {excluded_mirrors:?}");

    let mirrors_status: MirrorsStatus = MirrorsStatus::from_online_json(&arguments.source_url)
        .with_context(|| {
            format!(
                "Failed to fetch mirrors status from `{}`",
                arguments.source_url
            )
        })?;

    let best_synced_mirrors: Mirrors = mirrors_status
        .best_synced_mirrors(arguments.max_check, excluded_mirrors)
        .context("Could not filter best synced mirrors")?;

    let best_mirrors: Mirrors = best_synced_mirrors
        .evaluate(arguments.mirrors, arguments.target_db)
        .context("Failed to evaluate mirror")?;

    if let Some(output_file) = &arguments.output_file {
        // Write to file
        best_mirrors
            .to_mirrorlist_file(output_file, &arguments.source_url)
            .with_context(|| {
                format!(
                    "Could not write to mirrorlist file `{}`",
                    output_file.display()
                )
            })?;
    } else {
        // Write to stdout
        print!(
            "{}",
            &best_mirrors
                .to_pacman_mirror_list()
                .context("Could not create pacman mirror list format")?
        );
    }

    if let Some(stats_file) = &arguments.stats_file {
        best_mirrors
            .to_csv(stats_file)
            .with_context(|| format!("Failed to save stats file `{}`", stats_file.display()))?;
    }

    Ok(())
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        error!("{err:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
