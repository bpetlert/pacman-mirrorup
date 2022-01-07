use crate::mirror::TargetDb;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
pub struct Arguments {
    /// Arch Linux mirrors status's data source
    #[clap(
        short = 'S',
        long,
        default_value = "https://www.archlinux.org/mirrors/status/json/"
    )]
    pub source_url: String,

    /// Choose speed test target database file (Core, Community, or Extra)
    #[clap(
        short = 't',
        long,
        ignore_case = true,
        default_value = "Community",
        arg_enum
    )]
    pub target_db: TargetDb,

    /// Mirror list output file
    #[clap(short = 'o', long, parse(from_os_str))]
    pub output_file: Option<PathBuf>,

    /// Limit the list to the n mirrors with the highest score.
    #[clap(short = 'm', long, default_value = "10")]
    pub mirrors: u32,

    /// The maximum number of threads to use when measure transfer rate
    #[clap(short = 'T', long, default_value = "5")]
    pub threads: usize,

    /// Statistics output file
    #[clap(short = 's', long, parse(from_os_str))]
    pub stats_file: Option<PathBuf>,

    #[clap(flatten)]
    pub verbose: Verbosity,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{FromArgMatches, IntoApp};

    #[test]
    fn test_args() {
        // Default arguments
        let args =
            Arguments::from_arg_matches(&Arguments::into_app().get_matches_from(vec!["test"]))
                .unwrap();
        assert_eq!(
            args.source_url,
            "https://www.archlinux.org/mirrors/status/json/".to_owned()
        );
        assert_eq!(args.target_db, TargetDb::Community);
        assert_eq!(args.output_file, None);
        assert_eq!(args.mirrors, 10);
        assert_eq!(args.threads, 5);
        assert_eq!(args.stats_file, None);
        assert_eq!(args.verbose.log_level(), Some(log::Level::Error));

        // Full long arguments
        let args = Arguments::from_arg_matches(&Arguments::into_app().get_matches_from(vec![
            "test",
            "--source-url",
            "https://www.archlinux.org/mirrors/status/json/",
            "--target-db",
            "community",
            "--output-file",
            "/tmp/mirrorlist",
            "--mirrors",
            "20",
            "--threads",
            "20",
            "--stats-file",
            "/tmp/stats",
            "--verbose",
            "--verbose",
            "--verbose",
            "--verbose",
        ]))
        .unwrap();
        assert_eq!(
            args.source_url,
            "https://www.archlinux.org/mirrors/status/json/".to_owned()
        );
        assert_eq!(args.target_db, TargetDb::Community);
        assert_eq!(args.output_file, Some(PathBuf::from("/tmp/mirrorlist")));
        assert_eq!(args.mirrors, 20);
        assert_eq!(args.threads, 20);
        assert_eq!(args.stats_file, Some(PathBuf::from("/tmp/stats")));
        assert_eq!(args.verbose.log_level(), Some(log::Level::Trace));

        // Full short arguments
        let args = Arguments::from_arg_matches(&Arguments::into_app().get_matches_from(vec![
            "test",
            "-S",
            "https://www.archlinux.org/mirrors/status/json/",
            "-t",
            "community",
            "-o",
            "/tmp/mirrorlist",
            "-m",
            "20",
            "-T",
            "20",
            "-s",
            "/tmp/stats",
            "-vvvv",
        ]))
        .unwrap();
        assert_eq!(
            args.source_url,
            "https://www.archlinux.org/mirrors/status/json/".to_owned()
        );
        assert_eq!(args.target_db, TargetDb::Community);
        assert_eq!(args.output_file, Some(PathBuf::from("/tmp/mirrorlist")));
        assert_eq!(args.mirrors, 20);
        assert_eq!(args.threads, 20);
        assert_eq!(args.stats_file, Some(PathBuf::from("/tmp/stats")));
        assert_eq!(args.verbose.log_level(), Some(log::Level::Trace));

        // Quiet flag
        let args = Arguments::from_arg_matches(
            &Arguments::into_app().get_matches_from(vec!["test", "-qq"]),
        )
        .unwrap();
        assert_eq!(args.verbose.log_level(), None);
    }
}
