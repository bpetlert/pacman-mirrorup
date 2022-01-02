use crate::mirror::TargetDb;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, PartialEq, Debug)]
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

    /// Increment verbosity level once per call
    /// [error, -v: warn, -vv: info, -vvv: debug, -vvvv: trace]
    #[clap(short = 'v', long, parse(from_occurrences))]
    pub verbose: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{FromArgMatches, IntoApp};

    #[test]
    fn test_args() {
        // Default arguments
        assert_eq!(
            Arguments {
                source_url: "https://www.archlinux.org/mirrors/status/json/".to_string(),
                target_db: TargetDb::Community,
                output_file: None,
                mirrors: 10,
                threads: 5,
                stats_file: None,
                verbose: 0
            },
            Arguments::from_arg_matches(&Arguments::into_app().get_matches_from(vec!["test"]))
                .unwrap()
        );

        // Full long arguments
        assert_eq!(
            Arguments {
                source_url: "https://www.archlinux.org/mirrors/status/json/".to_string(),
                target_db: TargetDb::Community,
                output_file: Some(PathBuf::from("/tmp/mirrorlist")),
                mirrors: 20,
                threads: 20,
                stats_file: Some(PathBuf::from("/tmp/stats")),
                verbose: 4
            },
            Arguments::from_arg_matches(&Arguments::into_app().get_matches_from(vec![
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
                "--verbose"
            ]))
            .unwrap()
        );

        // Full short arguments
        assert_eq!(
            Arguments {
                source_url: "https://www.archlinux.org/mirrors/status/json/".to_string(),
                target_db: TargetDb::Community,
                output_file: Some(PathBuf::from("/tmp/mirrorlist")),
                mirrors: 20,
                threads: 20,
                stats_file: Some(PathBuf::from("/tmp/stats")),
                verbose: 4
            },
            Arguments::from_arg_matches(&Arguments::into_app().get_matches_from(vec![
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
                "-vvvv"
            ]))
            .unwrap()
        );
    }
}
