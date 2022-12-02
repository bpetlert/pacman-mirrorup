mod args;
mod mirror;

use std::{
    fs::File,
    io::{self, BufRead},
    path::Path,
    process::ExitCode,
};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use mimalloc::MiMalloc;
use tracing::{debug, error};
use tracing_subscriber::EnvFilter;

use crate::{
    args::Arguments,
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

/// Load excluded mirror list form file
fn read_exclude_from(file: &Path) -> Result<Vec<String>> {
    let lines =
        io::BufReader::new(File::open(file).with_context(|| {
            format!("Could not open excluded mirror file `{}`", file.display())
        })?)
        .lines();

    let excluded_mirrors: Vec<String> = lines
        .into_iter()
        .filter_map(|line| line.ok())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    Ok(excluded_mirrors)
}

/// Merge all excluded mirror lists, sort and remove duplicates
fn merge_exclude_mirror_list(lists: Vec<Vec<String>>) -> Result<Option<Vec<String>>> {
    let mut list: Vec<String> = lists.into_iter().flatten().collect();
    list.sort();
    list.dedup();

    if list.is_empty() {
        Ok(None)
    } else {
        Ok(Some(list))
    }
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        error!("{err:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclude_from_file() {
        let excluded_mirrors = read_exclude_from(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/",
            "tests/",
            "excluded_mirrors.conf"
        )))
        .unwrap();

        assert_eq!(excluded_mirrors.len(), 2);
        assert_eq!(excluded_mirrors[0], "ban.this.mirror");
        assert_eq!(excluded_mirrors[1], "ban.this-mirror.also");
    }

    #[test]
    fn merge_exclude_mirror() {
        let list1 = vec![
            "block.this.mirror".to_string(),
            "ban.this-mirror.also".to_string(),
            "ban.this.mirror".to_string(),
        ];

        let list2 = read_exclude_from(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/",
            "tests/",
            "excluded_mirrors.conf"
        )))
        .unwrap();

        let list = merge_exclude_mirror_list(vec![list1, list2])
            .unwrap()
            .unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0], "ban.this-mirror.also");
        assert_eq!(list[1], "ban.this.mirror");
        assert_eq!(list[2], "block.this.mirror");
    }
}
