mod args;
mod exclude;
mod mirror;

use std::{
    io::{self, Write},
    process::ExitCode,
};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use mimalloc::MiMalloc;
use tracing::{debug, error};
use tracing_subscriber::EnvFilter;

use crate::{
    args::Arguments,
    exclude::{ExcludeKind, ExcludedMirrors},
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
    let excluded_mirrors: Option<ExcludedMirrors> = 'excluded_mirrors: {
        if arguments.exclude.is_none() && arguments.exclude_from.is_none() {
            break 'excluded_mirrors None;
        }

        let mut exclude = ExcludedMirrors::new();

        if let Some(f) = arguments.exclude_from {
            exclude.add_from(&f)?;
        }

        // Add excluded list from --exclude option after --exclude-from option,
        // since the last occurrence of excluded pattern will overrride previous one.
        if let Some(list) = arguments.exclude {
            for m in list {
                exclude.add(ExcludeKind::try_from(m.as_str())?);
            }
        }

        Some(exclude)
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
        .best_synced_mirrors(Some(arguments.max_check), excluded_mirrors)
        .context("Could not filter best synced mirrors")?;

    let best_mirrors: Mirrors = best_synced_mirrors
        .evaluate(arguments.mirrors, arguments.target_db)
        .context("Failed to evaluate mirror")?;

    // Save stats file
    if let Some(stats_file) = &arguments.stats_file {
        best_mirrors
            .to_csv(stats_file)
            .with_context(|| format!("Failed to save stats file `{}`", stats_file.display()))?;
    }

    // Save mirrors to file
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
        return Ok(());
    }

    // Write to stdout
    let mirror_list: String = best_mirrors
        .to_pacman_mirror_list()
        .context("Could not create pacman mirror list format")?;
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    if let Err(err) =
        writeln!(stdout, "{mirror_list}").context("Could not write mirror list to STDOUT")
    {
        if let Some(io_err) = err.downcast_ref::<io::Error>() {
            match io_err.kind() {
                io::ErrorKind::BrokenPipe => return Ok(()),
                _ => bail!("{err:#}"),
            }
        } else {
            bail!("{err:#}");
        }
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
