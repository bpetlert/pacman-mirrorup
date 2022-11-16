use crate::mirror::TargetDb;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// Arch Linux mirrors status's data source
    #[arg(
        short = 'S',
        long,
        value_name = "URL",
        default_value = "https://www.archlinux.org/mirrors/status/json/"
    )]
    pub source_url: String,

    /// Choose speed test target database file
    #[arg(
        short = 't',
        long,
        value_name = "REPO-NAME",
        ignore_case = true,
        default_value = "Community",
        value_enum
    )]
    pub target_db: TargetDb,

    /// Mirror list output file
    #[arg(short = 'o', long, value_name = "OUTPUT-FILE")]
    pub output_file: Option<PathBuf>,

    /// Statistics output file
    #[arg(short = 's', long, value_name = "STATS-FILE")]
    pub stats_file: Option<PathBuf>,

    /// Maximum number of synced mirrors to check,
    /// 0 = check all synced mirrors
    #[arg(short = 'c', long, value_name = "NUMBER", default_value = "100")]
    pub max_check: u32,

    /// Limit the list to the n mirrors with the highest score.
    #[arg(short = 'm', long, value_name = "NUMBER", default_value = "10")]
    pub mirrors: u32,

    /// The maximum number of threads to use when measure transfer rate
    #[arg(short = 'T', long, value_name = "NUMBER", default_value = "5")]
    pub threads: usize,

}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, FromArgMatches};

    #[test]
    fn test_args() {
        // Default arguments
        let args = Arguments::from_arg_matches(
            &Arguments::command().get_matches_from(vec![env!("CARGO_CRATE_NAME")]),
        )
        .expect("Paring argument");
        assert_eq!(
            args.source_url,
            "https://www.archlinux.org/mirrors/status/json/".to_owned()
        );
        assert_eq!(args.target_db, TargetDb::Community);
        assert_eq!(args.output_file, None);
        assert_eq!(args.max_check, 100);
        assert_eq!(args.mirrors, 10);
        assert_eq!(args.threads, 5);
        assert_eq!(args.stats_file, None);

        // Full long arguments
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "--source-url",
            "https://www.archlinux.org/mirrors/status/json/",
            "--target-db",
            "community",
            "--output-file",
            "/tmp/mirrorlist",
            "--max-check",
            "200",
            "--mirrors",
            "20",
            "--threads",
            "20",
            "--stats-file",
            "/tmp/stats",
        ]))
        .expect("Paring argument");
        assert_eq!(
            args.source_url,
            "https://www.archlinux.org/mirrors/status/json/".to_owned()
        );
        assert_eq!(args.target_db, TargetDb::Community);
        assert_eq!(args.output_file, Some(PathBuf::from("/tmp/mirrorlist")));
        assert_eq!(args.max_check, 200);
        assert_eq!(args.mirrors, 20);
        assert_eq!(args.threads, 20);
        assert_eq!(args.stats_file, Some(PathBuf::from("/tmp/stats")));

        // Full short arguments
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "-S",
            "https://www.archlinux.org/mirrors/status/json/",
            "-t",
            "community",
            "-o",
            "/tmp/mirrorlist",
            "-c",
            "200",
            "-m",
            "20",
            "-T",
            "20",
            "-s",
            "/tmp/stats",
        ]))
        .expect("Paring argument");
        assert_eq!(
            args.source_url,
            "https://www.archlinux.org/mirrors/status/json/".to_owned()
        );
        assert_eq!(args.target_db, TargetDb::Community);
        assert_eq!(args.output_file, Some(PathBuf::from("/tmp/mirrorlist")));
        assert_eq!(args.max_check, 200);
        assert_eq!(args.mirrors, 20);
        assert_eq!(args.threads, 20);
        assert_eq!(args.stats_file, Some(PathBuf::from("/tmp/stats")));
    }
}
