use crate::errors;
use rayon::prelude::*;
use reqwest::{self, StatusCode};
use serde_derive::{Deserialize, Serialize};
use std::{
    convert::TryInto,
    time::{Duration, Instant},
};

#[derive(Deserialize, Debug)]
pub struct MirrorsStatus {
    cutoff: usize,
    last_check: String,
    num_checks: usize,
    check_frequency: usize,
    urls: Mirrors,
    version: usize,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
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
    pub fn from_online_json<T>(url: T) -> errors::Result<Self>
    where
        T: reqwest::IntoUrl,
    {
        let response = reqwest::blocking::get(url)?;
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
    fn best_synced_mirrors(&self) -> Mirrors;
}

impl Filter for MirrorsStatus {
    fn best_synced_mirrors(&self) -> Mirrors {
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

        // Take only 100 mirrors
        mirrors.truncate(100);

        mirrors
    }
}

trait Benchmark {
    /// Measure time (in seconds) it took to connect (from user's geography) and retrive the 'core/os/x86_64/core.db' file from the given URL.
    fn measure_duration(&mut self);
}

impl Benchmark for Mirror {
    fn measure_duration(&mut self) {
        let url: String = [&self.url, "core/os/x86_64/core.db"].join("");
        let client: reqwest::blocking::Client = reqwest::blocking::Client::builder()
            .no_gzip()
            .no_proxy()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .use_rustls_tls()
            .build()
            .unwrap();

        let start = Instant::now();
        let response = client.get(&url).send();
        match response {
            Ok(resp) => match resp.status() {
                StatusCode::OK => {
                    let duration: f64 = start.elapsed().as_millis() as f64;
                    let transfer_time: f64 = duration / 1000.0_f64;
                    let file_size: f64 = resp.content_length().unwrap() as f64;
                    self.transfer_rate = Some(file_size / transfer_time);
                }
                _ => self.transfer_rate = None,
            },
            Err(_) => self.transfer_rate = None,
        }
    }
}

impl Benchmark for Mirrors {
    fn measure_duration(&mut self) {
        self.par_iter_mut().for_each(|mirror| {
            mirror.measure_duration();
        });
    }
}

trait Statistics {
    /// Calculate weighted score
    fn score(&mut self);

    /// Sort descending order by weighted score
    fn sort_by_weighted_score(&mut self);

    /// Select n mirrors
    fn select(&mut self, n: usize);
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

    fn select(&mut self, n: usize) {
        self.truncate(n.try_into().unwrap());
    }
}

pub trait Evaluation {
    /// Returns the n best mirrors based on mirror score
    fn evaluate(&self, n: usize) -> Mirrors;
}

impl Evaluation for Mirrors {
    fn evaluate(&self, n: usize) -> Mirrors {
        let mut mirrors: Mirrors = self.clone();
        mirrors.measure_duration();
        mirrors.score();
        mirrors.sort_by_weighted_score();
        mirrors.select(n);
        mirrors
    }
}

pub trait ToPacmanMirrorList {
    fn to_pacman_mirror_list(&self) -> String;
}

impl ToPacmanMirrorList for Mirror {
    fn to_pacman_mirror_list(&self) -> String {
        format!("Server = {url}$repo/os/$arch", url = self.url)
    }
}

impl ToPacmanMirrorList for Mirrors {
    fn to_pacman_mirror_list(&self) -> String {
        let mut list = String::new();
        for mirror in self.iter() {
            let line = mirror.to_pacman_mirror_list() + "\n";
            list.push_str(&line);
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn test_deserialize_mirrors_status() {
        let mirrors_status_raw = include_str!("mirrors_status_json_test.raw");
        let _: MirrorsStatus = serde_json::from_str(mirrors_status_raw).unwrap();
    }

    #[test]
    fn test_best_synced_mirrors() {
        let mirrors_status_raw = include_str!("mirrors_status_json_test.raw");
        let mirrors_status: MirrorsStatus = serde_json::from_str(mirrors_status_raw).unwrap();
        let mirrors: Mirrors = mirrors_status.best_synced_mirrors();

        mirrors.iter().for_each(|m| {
            // Only active mirror
            assert!(m.active);

            // Only protocol HTTP/HTTPS
            assert!(m.protocol == "http" || m.protocol == "https");

            // 100% sync
            assert!((m.completion_pct - 1.0).abs() < std::f64::EPSILON);

            // delay < 3600
            assert_ne!(m.delay, None);
            assert!(m.delay.unwrap() < 3600);
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
        let mirrors_status_raw = include_str!("mirrors_status_json_test.raw");
        let mirrors_status: MirrorsStatus = serde_json::from_str(mirrors_status_raw).unwrap();
        let mut mirrors: Mirrors = mirrors_status.best_synced_mirrors();
        mirrors.truncate(30);
        mirrors.measure_duration();
        mirrors.iter().for_each(|m| {
            assert_ne!(m.transfer_rate, None, "Failed host = {}", m.url);
        });
    }

    #[test]
    fn test_score() {
        let mirrors_status_raw = include_str!("mirrors_status_json_test.raw");
        let mirrors_status: MirrorsStatus = serde_json::from_str(mirrors_status_raw).unwrap();
        let mut mirrors: Mirrors = mirrors_status.best_synced_mirrors();
        mirrors.iter_mut().for_each(|m| {
            m.transfer_rate = m.duration_avg;
        });
        mirrors.score();
        let sum: f64 = mirrors.iter().map(|m| m.weighted_score.unwrap()).sum();
        assert!(
            (sum - 67.038_115_183_421_11).abs() < std::f64::EPSILON,
            "sum = {}",
            sum
        );
    }

    #[test]
    fn test_sort_by_weighted_score() {
        let mirrors_status_raw = include_str!("mirrors_status_json_test.raw");
        let mirrors_status: MirrorsStatus = serde_json::from_str(mirrors_status_raw).unwrap();
        let mut mirrors: Mirrors = mirrors_status.best_synced_mirrors();
        mirrors.iter_mut().for_each(|m| {
            m.transfer_rate = m.duration_avg;
        });
        mirrors.score();
        mirrors.sort_by_weighted_score();

        // 1st mirror
        let first: f64 = mirrors.first().unwrap().weighted_score.unwrap();
        assert!(
            (first - 1.055_667_912_102_490_1).abs() < std::f64::EPSILON,
            "first weighted score = {}",
            first
        );

        // latest mirror
        let last: f64 = mirrors.last().unwrap().weighted_score.unwrap();
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
        let mirrors_status_raw = include_str!("mirrors_status_json_test.raw");
        let mirrors_status: MirrorsStatus = serde_json::from_str(mirrors_status_raw).unwrap();
        let mut mirrors: Mirrors = mirrors_status.best_synced_mirrors();
        mirrors.select(20);
        assert_eq!(mirrors.len(), 20);
    }

    #[test]
    fn test_to_pacman_mirror_list() {
        let mirrors_status_raw = include_str!("mirrors_status_json_test.raw");
        let mirrors_status: MirrorsStatus = serde_json::from_str(mirrors_status_raw).unwrap();
        let mirrors: Mirrors = mirrors_status.urls;
        let mirror_format =
            Regex::new(r"Server\x20=\x20(http(s?)|rsync)://(\S+\.\S+/)(\$repo/os/\$arch)").unwrap();

        // Check Mirror
        for mirror in mirrors.iter() {
            assert!(mirror_format.is_match(&mirror.to_pacman_mirror_list()));
        }

        // Check Mirrors
        for line in mirrors.to_pacman_mirror_list().lines() {
            assert!(mirror_format.is_match(&line));
        }
    }
}
