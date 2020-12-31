use crate::mirror::TargetDb;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, PartialEq, Debug)]
#[structopt(author, about)]
pub struct Arguments {
    /// Arch Linux mirrors status's data source
    #[structopt(
        short = "S",
        long = "source-url",
        default_value = "https://www.archlinux.org/mirrors/status/json/"
    )]
    pub source_url: String,

    /// Choose speed test target database file (Core, Community, or Extra)
    #[structopt(
        short = "t",
        long = "target-db",
        case_insensitive = true,
        default_value = "Community"
    )]
    pub target_db: TargetDb,

    /// Mirror list output file
    #[structopt(short = "o", long = "output-file", parse(from_os_str))]
    pub output_file: Option<PathBuf>,

    /// Limit the list to the n mirrors with the highest score.
    #[structopt(short = "m", long = "mirrors", default_value = "10")]
    pub mirrors: u32,

    /// The maximum number of threads to use when measure transfer rate
    #[structopt(short = "T", long = "threads", default_value = "5")]
    pub threads: usize,

    /// Statistics output file
    #[structopt(short = "s", long = "stats-file", parse(from_os_str))]
    pub stats_file: Option<PathBuf>,

    /// Increment verbosity level once per call
    /// [error, -v: warn, -vv: info, -vvv: debug, -vvvv: trace]
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    pub verbose: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

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
            Arguments::from_clap(&Arguments::clap().get_matches_from(&["test",]))
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
            Arguments::from_clap(&Arguments::clap().get_matches_from(&[
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
            Arguments::from_clap(&Arguments::clap().get_matches_from(&[
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
        );
    }
}
