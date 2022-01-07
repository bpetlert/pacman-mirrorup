use anyhow::{anyhow, Result};
use clap::Parser;
use log::{debug, error};
use std::{env, process};

mod args;
mod mirror;

use args::Arguments;
use mirror::{Evaluation, Filter, Mirrors, MirrorsStatus, Statistics, ToPacmanMirrorList};

fn run_app() -> Result<()> {
    let arguments = Arguments::parse();

    let mut log_builder = pretty_env_logger::formatted_builder();
    if let Ok(value) = env::var("RUST_LOG") {
        log_builder.parse_filters(&value);
    } else {
        log_builder.filter_level(arguments.verbose.log_level().unwrap().to_level_filter());
    }
    log_builder.init();

    debug!("Run with {:?}", arguments);

    if let Some(output_file) = &arguments.output_file {
        if output_file.exists() {
            return Err(anyhow!("`{}` is exist.", output_file.to_str().unwrap()));
        }
    }

    if let Some(stats_file) = &arguments.stats_file {
        if stats_file.exists() {
            return Err(anyhow!("`{}` is exist.", stats_file.to_str().unwrap()));
        }
    }

    rayon::ThreadPoolBuilder::new()
        .num_threads(arguments.threads)
        .build_global()
        .unwrap();

    let mirrors_status: MirrorsStatus = MirrorsStatus::from_online_json(&arguments.source_url)?;
    let best_synced_mirrors: Mirrors = mirrors_status.best_synced_mirrors()?;
    let best_mirrors: Mirrors =
        best_synced_mirrors.evaluate(arguments.mirrors, arguments.target_db)?;

    if let Some(output_file) = &arguments.output_file {
        // Write to file
        best_mirrors.to_mirrorlist_file(output_file, &arguments.source_url)?;
    } else {
        // Write to stdout
        print!("{}", &best_mirrors.to_pacman_mirror_list());
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
