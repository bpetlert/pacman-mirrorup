use std::path::PathBuf;

use clap::Parser;

use crate::mirror::TargetDb;

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
        default_value = "Extra",
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

    /// Exclude a mirror
    #[arg(long, value_name = "DOMAIN-NAME")]
    pub exclude: Option<Vec<String>>,

    /// Read exclude mirrors from FILE
    #[arg(long, value_name = "FILE")]
    pub exclude_from: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, FromArgMatches};

    #[test]
    fn default_args() {
        let args = Arguments::from_arg_matches(
            &Arguments::command().get_matches_from(vec![env!("CARGO_CRATE_NAME")]),
        )
        .unwrap();

        assert_eq!(
            args.source_url,
            "https://www.archlinux.org/mirrors/status/json/".to_owned()
        );
        assert_eq!(args.target_db, TargetDb::Extra);
        assert_eq!(args.output_file, None);
        assert_eq!(args.stats_file, None);
        assert_eq!(args.max_check, 100);
        assert_eq!(args.mirrors, 10);
        assert_eq!(args.threads, 5);
        assert_eq!(args.exclude, None);
        assert_eq!(args.exclude_from, None);
    }

    #[test]
    fn long_args() {
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "--source-url",
            "https://www.archlinux.org/mirrors/status/json/",
            "--target-db",
            "extra",
            "--output-file",
            "/tmp/mirrorlist",
            "--stats-file",
            "/tmp/stats",
            "--max-check",
            "200",
            "--mirrors",
            "20",
            "--threads",
            "20",
        ]))
        .unwrap();

        assert_eq!(
            args.source_url,
            "https://www.archlinux.org/mirrors/status/json/".to_owned()
        );
        assert_eq!(args.target_db, TargetDb::Extra);
        assert_eq!(args.output_file, Some(PathBuf::from("/tmp/mirrorlist")));
        assert_eq!(args.stats_file, Some(PathBuf::from("/tmp/stats")));
        assert_eq!(args.max_check, 200);
        assert_eq!(args.mirrors, 20);
        assert_eq!(args.threads, 20);
    }

    #[test]
    fn short_args() {
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "-S",
            "https://www.archlinux.org/mirrors/status/json/",
            "-t",
            "extra",
            "-o",
            "/tmp/mirrorlist",
            "-s",
            "/tmp/stats",
            "-c",
            "200",
            "-m",
            "20",
            "-T",
            "20",
        ]))
        .unwrap();

        assert_eq!(
            args.source_url,
            "https://www.archlinux.org/mirrors/status/json/".to_owned()
        );
        assert_eq!(args.target_db, TargetDb::Extra);
        assert_eq!(args.output_file, Some(PathBuf::from("/tmp/mirrorlist")));
        assert_eq!(args.stats_file, Some(PathBuf::from("/tmp/stats")));
        assert_eq!(args.max_check, 200);
        assert_eq!(args.mirrors, 20);
        assert_eq!(args.threads, 20);
    }

    #[test]
    fn exclude_mirror() {
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "--exclude",
            "ban.this.mirror",
        ]))
        .unwrap();

        assert_eq!(args.exclude.unwrap(), vec!["ban.this.mirror"]);
    }

    #[test]
    fn exclude_mirrors() {
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "--exclude",
            "ban.this.mirror",
            "--exclude",
            "ban.this-mirror.also",
        ]))
        .unwrap();

        assert_eq!(
            args.exclude.unwrap(),
            vec!["ban.this.mirror", "ban.this-mirror.also"]
        );
    }

    #[test]
    fn exclude_from() {
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "--exclude-from",
            "/path/to/excluded-mirror.conf",
        ]))
        .unwrap();

        assert_eq!(
            args.exclude_from.unwrap(),
            PathBuf::from("/path/to/excluded-mirror.conf")
        );
    }
}
