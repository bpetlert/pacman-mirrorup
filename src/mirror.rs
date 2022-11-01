use anyhow::{bail, Result};
use rayon::prelude::*;
use reqwest::{self, blocking::Client};
use serde::{Deserialize, Serialize};
use std::{
    convert::TryInto,
    fmt::Write,
    fs::OpenOptions,
    io::BufWriter,
    path::Path,
    time::{Duration, Instant},
};
use tracing::{debug, warn};

static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_HOMEPAGE"),
    ")"
);

#[derive(clap::ValueEnum, PartialEq, Eq, Debug, Clone, Copy)]
pub enum TargetDb {
    Core,
    Community,
    Extra,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct MirrorsStatus {
    cutoff: usize,
    last_check: String,
    num_checks: usize,
    check_frequency: usize,
    urls: Mirrors,
    version: usize,
}

#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct Mirror {
    url: String,
    protocol: String,
    last_sync: Option<String>,
    completion_pct: f64,
    delay: Option<usize>,
    duration_avg: Option<f64>,
    duration_stddev: Option<f64>,
    score: Option<f64>,
    active: bool,
    country: String,
    country_code: String,
    isos: bool,
    ipv4: bool,
    ipv6: bool,
    details: String,

    // pacman-mirrorup data
    transfer_rate: Option<f64>,
    weighted_score: Option<f64>,
}

pub type Mirrors = Vec<Mirror>;

impl MirrorsStatus {
    /// Fetch mirrors status from server
    pub fn from_online_json<T>(url: T) -> Result<Self>
    where
        T: reqwest::IntoUrl,
    {
        let client = Client::builder()
            .user_agent(APP_USER_AGENT)
            .gzip(true)
            .http2_prior_knowledge()
            .use_rustls_tls()
            .build()?;
        let response = client.get(url).send()?;
        let mirrors_status: MirrorsStatus = response.json()?;
        Ok(mirrors_status)
    }
}

pub trait Filter {
    /// Filter mirror by
    ///     ==> active
    ///     ==> protocol(http/https)
    ///     ==> completion_pct(==1.0)
    ///     ==> delay(< 3600)
    fn best_synced_mirrors(&self, max_check: u32) -> Result<Mirrors>;
}

impl Filter for MirrorsStatus {
    fn best_synced_mirrors(&self, max_check: u32) -> Result<Mirrors> {
        let mut mirrors: Mirrors = self
            .urls
            .iter()
            .filter(|m| m.active)
            .filter(|m| m.protocol == "http" || m.protocol == "https")
            .filter(|m| (m.completion_pct - 1.0).abs() < std::f64::EPSILON)
            .filter(|m| match m.delay {
                Some(d) => d < 3600,
                None => false,
            })
            .cloned()
            .collect();

        // Sort by delay value ascending
        mirrors.sort_by(|a, b| a.delay.cmp(&b.delay));

        if max_check != 0 {
            // Take only N synced mirrors
            mirrors.truncate(max_check.try_into()?);
        }

        if mirrors.is_empty() {
            bail!("No best synced mirrors");
        }

        Ok(mirrors)
    }
}

trait Benchmark {
    /// Measure time (in seconds) it took to connect (from user's geography)
    /// and retrive the '[core,community,extra]/os/x86_64/[core,community,extra].db' file from the given URL.
    fn measure_duration(&mut self, target_db: TargetDb) -> Result<()>;
}

impl Benchmark for Mirror {
    fn measure_duration(&mut self, target_db: TargetDb) -> Result<()> {
        let url: String = match target_db {
            TargetDb::Core => [&self.url, "core/os/x86_64/core.db"].join(""),
            TargetDb::Community => [&self.url, "community/os/x86_64/community.db"].join(""),
            TargetDb::Extra => [&self.url, "extra/os/x86_64/extra.db"].join(""),
        };

        self.transfer_rate = None;

        let client = Client::builder()
            .user_agent(APP_USER_AGENT)
            .no_gzip()
            .no_proxy()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .use_rustls_tls()
            .build()?;

        let start = Instant::now();
        let response = client.get(&url).send()?;
        if response.status().is_success() {
            let duration: f64 = start.elapsed().as_millis() as f64;
            let transfer_time: f64 = duration / 1000.0_f64;

            let file_size: f64 = match response.content_length() {
                Some(fs) => fs as f64,
                None => {
                    debug!("Transfer Rate: {} => None", url);
                    return Ok(());
                }
            };

            let transfer_rate = file_size / transfer_time;
            self.transfer_rate = Some(transfer_rate);
            debug!("Transfer Rate: {} => {}", url, transfer_rate);
        } else {
            debug!("Transfer Rate: {} => None", url);
        }

        Ok(())
    }
}

impl Benchmark for Mirrors {
    fn measure_duration(&mut self, target_db: TargetDb) -> Result<()> {
        self.par_iter_mut().for_each(|mirror| {
            if let Err(err) = mirror.measure_duration(target_db) {
                warn!("{}", err);
            }
        });

        Ok(())
    }
}

pub trait Statistics {
    /// Calculate weighted score
    fn score(&mut self);

    /// Sort descending order by weighted score
    fn sort_by_weighted_score(&mut self);

    /// Select n mirrors
    fn select(&mut self, n: u32);

    /// Save evaluated mirrors to CSV file
    fn to_csv(&self, path: &Path) -> Result<()>;
}

impl Statistics for Mirrors {
    fn score(&mut self) {
        let max_score: f64 = self
            .iter()
            .map(|mirror| mirror.score.unwrap_or(std::f64::NAN))
            .fold(0.0_f64, |mut max, val| {
                if val > max {
                    max = val;
                }
                max
            });
        self.iter_mut().for_each(|mirror| {
            let transfer_rate: f64 = mirror.transfer_rate.unwrap_or(0.0_f64);
            let score: f64 = mirror.score.unwrap_or(std::f64::NAN);
            mirror.weighted_score = Some(transfer_rate * (max_score - score));
        });
    }

    fn sort_by_weighted_score(&mut self) {
        self.sort_by(|a, b| {
            let aa: f64 = a.weighted_score.unwrap_or(0.0_f64);
            let bb: f64 = b.weighted_score.unwrap_or(0.0_f64);
            aa.partial_cmp(&bb).unwrap().reverse()
        });
    }

    fn select(&mut self, n: u32) {
        self.truncate(n.try_into().expect("u32 to usize"));
    }

    fn to_csv(&self, path: &Path) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(false)
            .open(path)?;
        let mut wtr = csv::Writer::from_writer(file);
        for mirror in self.iter() {
            wtr.serialize(mirror)?;
        }
        wtr.flush()?;
        Ok(())
    }
}

pub trait Evaluation {
    /// Returns the n best mirrors based on mirror score
    fn evaluate(&self, n: u32, target_db: TargetDb) -> Result<Mirrors>;
}

impl Evaluation for Mirrors {
    fn evaluate(&self, n: u32, target_db: TargetDb) -> Result<Mirrors> {
        let mut mirrors: Mirrors = self.clone();
        let _ = mirrors.measure_duration(target_db);
        mirrors.score();
        mirrors.sort_by_weighted_score();
        mirrors.select(n);

        if mirrors.is_empty() {
            bail!("No best mirrors");
        }

        Ok(mirrors)
    }
}

pub trait ToPacmanMirrorList {
    /// Convert to pacman mirror list format
    fn to_pacman_mirror_list(&self) -> Result<String>;

    /// Write to mirrorlist file
    fn to_mirrorlist_file(&self, path: &Path, source_url: &str) -> Result<()>;

    fn header(&self, source_url: &str) -> Result<String> {
        let now = chrono::Local::now();
        Ok(format!(
            "\
            #\n\
            # /etc/pacman.d/mirrorlist\n\
            #\n\
            #\n\
            # Arch Linux mirrorlist generated by pacman-mirrorup\n\
            #\n\
            # pacman-mirrorup: https://github.com/bpetlert/pacman-mirrorup\n\
            # source: {}\n\
            # when: {}\n\
            #\n\
            \n\
            ",
            source_url,
            now.to_rfc2822()
        ))
    }
}

impl ToPacmanMirrorList for Mirror {
    fn to_pacman_mirror_list(&self) -> Result<String> {
        Ok(format!("Server = {url}$repo/os/$arch", url = self.url))
    }

    fn to_mirrorlist_file(&self, _path: &Path, _source_url: &str) -> Result<()> {
        unreachable!()
    }
}

impl ToPacmanMirrorList for Mirrors {
    fn to_pacman_mirror_list(&self) -> Result<String> {
        let mut list = String::new();
        for mirror in self.iter() {
            writeln!(&mut list, "{}", mirror.to_pacman_mirror_list()?)?;
        }
        Ok(list)
    }

    fn to_mirrorlist_file(&self, path: &Path, source_url: &str) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(false)
            .open(path)?;
        let mut file = BufWriter::new(file);
        std::io::Write::write_all(&mut file, self.header(source_url)?.as_bytes())?;
        std::io::Write::write_all(&mut file, self.to_pacman_mirror_list()?.as_bytes())?;
        std::io::Write::flush(&mut file)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn test_deserialize_mirrors_status() {
        let mirrors_status_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/mirrors_status.json"
        ));
        let _: MirrorsStatus =
            serde_json::from_str(mirrors_status_raw).expect("Deserialized mirror status");
    }

    #[test]
    fn test_best_synced_mirrors() {
        let mirrors_status_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/mirrors_status.json"
        ));
        let mirrors_status: MirrorsStatus =
            serde_json::from_str(mirrors_status_raw).expect("Deserialized mirror status");
        let mirrors: Mirrors = mirrors_status
            .best_synced_mirrors(100)
            .expect("Get best synced mirrors");

        mirrors.iter().for_each(|m| {
            // Only active mirror
            assert!(m.active);

            // Only protocol HTTP/HTTPS
            assert!(m.protocol == "http" || m.protocol == "https");

            // 100% sync
            assert!((m.completion_pct - 1.0).abs() < std::f64::EPSILON);

            // delay < 3600
            assert_ne!(m.delay, None);
            assert!(m.delay.expect("delay value") < 3600);
        });

        // Sort by delay value ascending
        for n in 1..mirrors.len() {
            assert!(mirrors[n - 1].delay <= mirrors[n].delay);
        }

        // Ensure only 100 mirrors returned
        assert_eq!(
            mirrors.len(),
            100,
            "Number of mirrors returned = {}",
            mirrors.len()
        );
    }

    #[test]
    fn test_messure_duration() {
        let mirrors_status_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/mirrors_status.json"
        ));
        let mirrors_status: MirrorsStatus =
            serde_json::from_str(mirrors_status_raw).expect("Deserialized mirror status");
        let mut mirrors: Mirrors = mirrors_status
            .best_synced_mirrors(100)
            .expect("Get best synced mirrors");
        mirrors.truncate(10);
        let _ = mirrors.measure_duration(TargetDb::Core);
        mirrors.iter().for_each(|m| {
            assert_ne!(m.transfer_rate, None, "Failed host = {}", m.url);
        });
    }

    #[test]
    fn test_score() {
        let mirrors_status_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/mirrors_status.json"
        ));
        let mirrors_status: MirrorsStatus =
            serde_json::from_str(mirrors_status_raw).expect("Deserialized mirror status");
        let mut mirrors: Mirrors = mirrors_status
            .best_synced_mirrors(100)
            .expect("Get best synced mirrors");
        mirrors.iter_mut().for_each(|m| {
            m.transfer_rate = m.duration_avg;
        });
        mirrors.score();
        let sum: f64 = mirrors
            .iter()
            .map(|m| m.weighted_score.expect("Weighted score value"))
            .sum();
        assert!(
            (sum - 45.90635811739935).abs() < std::f64::EPSILON,
            "sum = {}",
            sum
        );
    }

    #[test]
    fn test_sort_by_weighted_score() {
        let mirrors_status_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/mirrors_status.json"
        ));
        let mirrors_status: MirrorsStatus =
            serde_json::from_str(mirrors_status_raw).expect("Deserialized mirror status");
        let mut mirrors: Mirrors = mirrors_status
            .best_synced_mirrors(100)
            .expect("Get best synced mirrors");
        mirrors.iter_mut().for_each(|m| {
            m.transfer_rate = m.duration_avg;
        });
        mirrors.score();
        mirrors.sort_by_weighted_score();

        // 1st mirror
        let first: f64 = mirrors
            .first()
            .expect("First mirror")
            .weighted_score
            .expect("Weighted score value");
        assert!(
            (first - 1.1922723128592017).abs() < std::f64::EPSILON,
            "first weighted score = {}",
            first
        );

        // Last mirror
        let last: f64 = mirrors
            .last()
            .expect("Last mirror")
            .weighted_score
            .expect("Weighted score value");
        assert!(
            (last - 0.0_f64).abs() < std::f64::EPSILON,
            "last weighted score = {}",
            last
        );

        // sort by weighted score descending
        for n in 1..mirrors.len() {
            assert!(mirrors[n - 1].weighted_score >= mirrors[n].weighted_score);
        }
    }

    #[test]
    fn test_select_n_mirrors() {
        let mirrors_status_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/mirrors_status.json"
        ));
        let mirrors_status: MirrorsStatus =
            serde_json::from_str(mirrors_status_raw).expect("Deserialized mirror status");
        let mut mirrors: Mirrors = mirrors_status
            .best_synced_mirrors(100)
            .expect("Get best synced mirrors");
        mirrors.select(20);
        assert_eq!(mirrors.len(), 20);
    }

    #[test]
    fn test_mirrorlist_file_header() {
        let header_format = Regex::new(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/mirrorlist_header.regex"
        )))
        .expect("Creating regex");
        let mirror: Mirror = Default::default();
        let header = mirror
            .header("https://www.archlinux.org/mirrors/status/json/")
            .unwrap();
        assert!(
            header_format.is_match(&header),
            "\nheader-format:\n{}\n\nheader:\n{}",
            &header_format.as_str(),
            &header
        );
    }

    #[test]
    fn test_to_pacman_mirror_list() {
        let mirrors_status_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/mirrors_status.json"
        ));
        let mirrors_status: MirrorsStatus =
            serde_json::from_str(mirrors_status_raw).expect("Deserialized mirror status");
        let mirrors: Mirrors = mirrors_status.urls;
        let mirror_format =
            Regex::new(r"Server\x20=\x20(http(s?)|rsync)://(\S+\.\S+/)(\$repo/os/\$arch)")
                .expect("Creating regex");

        // Check Mirror
        for mirror in mirrors.iter() {
            assert!(mirror_format.is_match(&mirror.to_pacman_mirror_list().unwrap()));
        }

        // Check Mirrors
        for line in mirrors.to_pacman_mirror_list().unwrap().lines() {
            assert!(mirror_format.is_match(line));
        }
    }
}
