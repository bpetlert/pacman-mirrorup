use std::{
    convert::TryInto,
    fmt::Write,
    fs::OpenOptions,
    io::BufWriter,
    ops::{
        Deref,
        DerefMut,
    },
    path::Path,
    thread::sleep,
    time::{
        Duration,
        Instant,
    },
};

use anyhow::{
    Context,
    Result,
    bail,
};
use rayon::prelude::*;
use serde::{
    Deserialize,
    Serialize,
};
use tracing::{
    debug,
    info,
};
use ureq::{
    Agent,
    Body,
    Error,
    http::Response,
};
use url::Url;

use crate::exclude::ExcludedMirrors;

static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_HOMEPAGE"),
    ")"
);

pub const DEFAULT_SOURCE_URL: &str = "https://archlinux.org/mirrors/status/json/";

#[derive(clap::ValueEnum, PartialEq, Eq, Debug, Clone, Copy)]
pub enum TargetDb {
    Core,
    Extra,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct MirrorsStatus {
    cutoff: u64,
    last_check: String,
    num_checks: u64,
    check_frequency: u64,
    urls: Mirrors,
    version: u64,
}

#[derive(Default, Deserialize, Clone, Debug)]
pub struct Mirrors(Vec<Mirror>);

#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct Mirror {
    pub url: String,
    protocol: String,
    last_sync: Option<String>,
    completion_pct: Option<f64>,
    delay: Option<i64>,
    duration_avg: Option<f64>,
    duration_stddev: Option<f64>,
    score: Option<f64>,
    active: bool,
    pub country: String,
    pub country_code: String,
    isos: bool,
    ipv4: bool,
    ipv6: bool,
    details: String,

    // pacman-mirrorup data
    transfer_rate: Option<f64>,
    weighted_score: Option<f64>,
}

impl Deref for Mirrors {
    type Target = Vec<Mirror>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Mirrors {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<Mirror> for Mirrors {
    fn from_iter<T: IntoIterator<Item = Mirror>>(iter: T) -> Self {
        let mut mirrors: Self = Self::default();
        for i in iter {
            mirrors.0.push(i);
        }
        mirrors
    }
}

impl MirrorsStatus {
    /// Fetch mirrors status from server
    pub fn from_online_json(url: &str) -> Result<Self> {
        let mut response: Response<Body> = {
            let config = Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(5)))
                .user_agent(APP_USER_AGENT)
                .build();
            let agent: Agent = config.into();

            let max_retries = 5;
            let base_delay = Duration::from_secs(1);
            let mut attempts = 0;

            loop {
                match agent
                    .get(url)
                    .call()
                    .with_context(|| format!("Failed to fetch `{url}`"))
                {
                    Ok(result) => break result,
                    Err(err) => {
                        attempts += 1;
                        if attempts >= max_retries {
                            bail!("{err:#}");
                        }

                        // Exponential backoff
                        let delay = base_delay * 2_u32.pow(attempts as u32 - 1);
                        sleep(delay);
                    }
                }
            }
        };

        let mirrors_status: MirrorsStatus = response
            .body_mut()
            .read_json::<MirrorsStatus>()
            .context("Failed to deserialize the response body as MirrorsStatus")?;

        Ok(mirrors_status)
    }
}

pub trait Filter {
    /// Filter mirror by
    ///     ==> active
    ///     ==> protocol(http/https)
    ///     ==> completion_pct(==1.0)
    ///     ==> delay(< 3600)
    fn best_synced_mirrors(
        &self,
        max_check: Option<u32>,
        excluded_mirrors: Option<ExcludedMirrors>,
    ) -> Result<Mirrors>;
}

impl Filter for MirrorsStatus {
    fn best_synced_mirrors(
        &self,
        max_check: Option<u32>,
        excluded_mirrors: Option<ExcludedMirrors>,
    ) -> Result<Mirrors> {
        // Primary filter
        let mut mirrors: Mirrors = self
            .urls
            .iter()
            .filter(|m| m.active)
            .filter(|m| m.protocol == "http" || m.protocol == "https")
            .filter(|m| m.completion_pct.is_some())
            .filter(|m| (m.completion_pct.unwrap() - 1.0_f64).abs() < f64::EPSILON)
            .filter(|m| match m.delay {
                Some(d) => d < 3600,
                None => false,
            })
            .cloned()
            .collect();

        // Secondary filter: excluded mirrors
        if let Some(exclude) = excluded_mirrors {
            mirrors.retain(|m| !exclude.is_exclude(m));
        }

        // Sort by delay value ascending
        mirrors.sort_by(|a, b| a.delay.cmp(&b.delay));

        if let Some(max_check) = max_check {
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
    /// and retrive the '[core,extra]/os/x86_64/[core,extra].db' file from the given URL.
    fn measure_duration(&mut self, target_db: TargetDb) -> Result<()>;
}

impl Benchmark for Mirror {
    fn measure_duration(&mut self, target_db: TargetDb) -> Result<()> {
        let url: Url = Url::parse(&self.url)?;
        let url: Url = match target_db {
            TargetDb::Core => url.join("core/os/x86_64/core.db")?,
            TargetDb::Extra => url.join("extra/os/x86_64/extra.db")?,
        };

        self.transfer_rate = None;

        let config = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .build();
        let agent: Agent = config.into();

        let start = Instant::now();

        match agent
            .get(url.as_str())
            .header("User-Agent", APP_USER_AGENT)
            .call()
        {
            Ok(response) => {
                let transfer_time: f64 = start.elapsed().as_secs_f64();

                if let Some(file_size) = response.body().content_length() {
                    let transfer_rate = (file_size as f64) / transfer_time;
                    self.transfer_rate = Some(transfer_rate);
                    debug!("Transfer Rate: {url} => {transfer_rate}");
                } else {
                    debug!("Transfer Rate: {url} => None");
                    return Ok(());
                }
            }
            Err(Error::StatusCode(code)) => {
                bail!(format!("Failed to fetch `{url}, HTTP status code: {code}`"))
            }
            Err(_) => debug!("Transfer Rate: {url} => None"),
        }

        Ok(())
    }
}

impl Benchmark for Mirrors {
    fn measure_duration(&mut self, target_db: TargetDb) -> Result<()> {
        self.par_iter_mut().for_each(|mirror| {
            if let Err(err) = mirror
                .measure_duration(target_db)
                .context("Failed to measure transfer rate")
            {
                info!("{err:#}");
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
        // According to [Mirror Status](https://archlinux.org/mirrors/status/) for Mirror Score: lower is better,
        // while transfer_score: higher is better. The weighting cannot apply directly. The mirror scores are needed
        // to reverse using max score as based first.
        let max_score: f64 = self
            .iter()
            .map(|mirror| mirror.score.unwrap_or(f64::NAN))
            .reduce(f64::max)
            .unwrap_or(0.0_f64);

        self.iter_mut().for_each(|mirror| {
            let transfer_rate: f64 = mirror.transfer_rate.unwrap_or(0.0_f64);
            let score: f64 = mirror.score.unwrap_or(f64::NAN);
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
            .create_new(true)
            .write(true)
            .open(path)
            .with_context(|| format!("Could not create file `{}`", path.display()))?;

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
            .create_new(true)
            .write(true)
            .open(path)
            .with_context(|| format!("Could not create file `{}`", path.display()))?;

        let mut file = BufWriter::new(file);
        std::io::Write::write_all(&mut file, self.header(source_url)?.as_bytes())?;
        std::io::Write::write_all(&mut file, self.to_pacman_mirror_list()?.as_bytes())?;
        std::io::Write::flush(&mut file)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;
    use crate::exclude::ExcludeKind;

    #[test]
    fn test_online_deserialize_mirrors_status() {
        let mirrors: MirrorsStatus = MirrorsStatus::from_online_json(DEFAULT_SOURCE_URL).unwrap();
        assert!(mirrors.urls.len() >= 100);
    }

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
            .best_synced_mirrors(Some(100), None)
            .expect("Get best synced mirrors");

        mirrors.iter().for_each(|m| {
            // Only active mirror
            assert!(m.active);

            // Only protocol HTTP/HTTPS
            assert!(m.protocol == "http" || m.protocol == "https");

            // 100% sync
            assert!((m.completion_pct.unwrap() - 1.0_f64).abs() < f64::EPSILON);

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
    fn exclude_mirrors() {
        let mirrors_status_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/mirrors_status.json"
        ));
        let mirrors_status: MirrorsStatus =
            serde_json::from_str(mirrors_status_raw).expect("Deserialized mirror status");

        let mut excluded_mirrors = ExcludedMirrors::new();
        excluded_mirrors.add(ExcludeKind::Domain("mirrors.kernel.org".to_string()));
        excluded_mirrors.add(ExcludeKind::Domain("mirror.xtom.com.hk".to_string()));

        let mirrors: Mirrors = mirrors_status
            .best_synced_mirrors(None, Some(excluded_mirrors))
            .expect("Get best synced mirrors");

        assert_eq!(
            mirrors.len(),
            347,
            "Number of mirrors returned = {}",
            mirrors.len()
        );

        mirrors.iter().for_each(|m| {
            assert!(!m.url.contains("mirrors.kernel.org") && !m.url.contains("mirror.xtom.com.hk"));
        });
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
            .best_synced_mirrors(Some(100), None)
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
            .best_synced_mirrors(Some(100), None)
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
            (sum - 139.0737774108812).abs() < f64::EPSILON,
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
            .best_synced_mirrors(Some(100), None)
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
            (first - 2.738996062208527).abs() < f64::EPSILON,
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
            (last - 0.0_f64).abs() < f64::EPSILON,
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
            .best_synced_mirrors(Some(100), None)
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
            Regex::new(r"Server\x20=\x20(http(s?)|rsync|ftp)://(\S+\.\S+/)(\$repo/os/\$arch)")
                .expect("Creating regex");

        // Check Mirror
        for mirror in mirrors.iter() {
            assert!(
                mirror_format.is_match(&mirror.to_pacman_mirror_list().unwrap()),
                "Pacman mirrorlist: {:#?}",
                mirror.to_pacman_mirror_list().unwrap()
            );
        }

        // Check Mirrors
        for line in mirrors.to_pacman_mirror_list().unwrap().lines() {
            assert!(mirror_format.is_match(line));
        }
    }
}
