use std::{
    collections::HashSet,
    fs::File,
    io::{self, BufRead},
    path::Path,
};

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::{Regex, RegexSet};

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum ExcludeKind {
    Ignore,
    Domain(String),
    Country(String),
    CountryCode(String),
}

#[derive(Debug)]
pub struct ExcludedMirrors {
    pub exclude_list: HashSet<ExcludeKind>,
}

impl TryFrom<&str> for ExcludeKind {
    type Error = anyhow::Error;

    /// Convert a line to exclude pattern
    fn try_from(line: &str) -> Result<Self, Self::Error> {
        if line.is_empty() {
            return Ok(ExcludeKind::Ignore);
        }

        // Remove comment
        let line = line
            .trim()
            .chars()
            .take_while(|c| *c != '#' && *c != ';')
            .collect::<String>();

        let line = line.trim().to_lowercase();

        if line.is_empty() {
            return Ok(ExcludeKind::Ignore);
        }

        static EXCLUDE_SET_RE: Lazy<RegexSet> = Lazy::new(|| {
            RegexSet::new([
                r"domain\s*=\s*(?P<domain>\S*)",             // Domain
                r"country\s*=\s*(?P<country>\S*)",           // Country
                r"country_code\s*=\s*(?P<country_code>\S*)", // Country Code
            ])
            .expect("Create exclude regex set")
        });

        static EXCLUDE_CAPTURE_RE: Lazy<Vec<Regex>> = Lazy::new(|| {
            EXCLUDE_SET_RE
                .patterns()
                .iter()
                .map(|pat| Regex::new(pat).expect("Create exclude capture regex"))
                .collect()
        });

        const DOMAIN: usize = 0;
        const COUNTRY: usize = 1;
        const COUNTRY_CODE: usize = 2;

        let matches = EXCLUDE_SET_RE.matches(&line);

        if matches.matched(DOMAIN) {
            return Ok(ExcludeKind::Domain(
                EXCLUDE_CAPTURE_RE[DOMAIN].captures(&line).unwrap()["domain"].to_string(),
            ));
        } else if matches.matched(COUNTRY) {
            return Ok(ExcludeKind::Country(
                EXCLUDE_CAPTURE_RE[COUNTRY].captures(&line).unwrap()["country"].to_string(),
            ));
        } else if matches.matched(COUNTRY_CODE) {
            return Ok(ExcludeKind::CountryCode(
                EXCLUDE_CAPTURE_RE[COUNTRY_CODE].captures(&line).unwrap()["country_code"]
                    .to_string(),
            ));
        }

        // When no keyword found, return domain as default
        Ok(ExcludeKind::Domain(line))
    }
}

impl ExcludedMirrors {
    pub fn new() -> Self {
        Self {
            exclude_list: HashSet::new(),
        }
    }

    pub fn add(&mut self, exclude: ExcludeKind) {
        if exclude != ExcludeKind::Ignore {
            self.exclude_list.insert(exclude);
        }
    }

    pub fn add_from(&mut self, file: &Path) -> Result<()> {
        let lines: Vec<String> = io::BufReader::new(File::open(file).with_context(|| {
            format!("Could not open excluded mirror file `{}`", file.display())
        })?)
        .lines()
        .map_while(Result::ok)
        .collect();

        for line in lines {
            let exclude = ExcludeKind::try_from(line.as_str())?;
            self.add(exclude);
        }

        Ok(())
    }

    pub fn is_exclude(&self, kind: &ExcludeKind) -> bool {
        self.exclude_list.contains(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exclude_kind() {
        assert_eq!(
            ExcludeKind::try_from("# This is comment").unwrap(),
            ExcludeKind::Ignore
        );
        assert_eq!(
            ExcludeKind::try_from(" # This is comment").unwrap(),
            ExcludeKind::Ignore
        );

        assert_eq!(
            ExcludeKind::try_from("; This is comment").unwrap(),
            ExcludeKind::Ignore
        );

        // No space
        assert_eq!(
            ExcludeKind::try_from("domain=ban.this.mirror").unwrap(),
            ExcludeKind::Domain("ban.this.mirror".to_string())
        );
        assert_eq!(
            ExcludeKind::try_from("domain=ban.this.mirror # Comment").unwrap(),
            ExcludeKind::Domain("ban.this.mirror".to_string())
        );

        assert_eq!(
            ExcludeKind::try_from("domain = ban.this.mirror").unwrap(),
            ExcludeKind::Domain("ban.this.mirror".to_string())
        );
        assert_eq!(
            ExcludeKind::try_from("domain = ban.this.mirror # Comment").unwrap(),
            ExcludeKind::Domain("ban.this.mirror".to_string())
        );

        assert_eq!(
            ExcludeKind::try_from("country = SomeCountry").unwrap(),
            ExcludeKind::Country("somecountry".to_string())
        );

        assert_eq!(
            ExcludeKind::try_from("country_code = SC").unwrap(),
            ExcludeKind::CountryCode("sc".to_string())
        );

        // Without "domain="
        assert_eq!(
            ExcludeKind::try_from("ban.this.mirror").unwrap(),
            ExcludeKind::Domain("ban.this.mirror".to_string())
        );
        assert_eq!(
            ExcludeKind::try_from("ban.this.mirror # Comment").unwrap(),
            ExcludeKind::Domain("ban.this.mirror".to_string())
        );
    }

    #[test]
    fn test_exclude_from_file() {
        let mut excluded_mirrors = ExcludedMirrors::new();
        excluded_mirrors
            .add_from(Path::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/",
                "tests/",
                "excluded_mirrors.conf"
            )))
            .unwrap();

        assert_eq!(excluded_mirrors.exclude_list.len(), 4);

        assert!(excluded_mirrors
            .exclude_list
            .contains(&ExcludeKind::Domain("ban.this.mirror".to_string())));

        assert!(excluded_mirrors
            .exclude_list
            .contains(&ExcludeKind::Domain("ban.this-mirror.also".to_string())));

        assert!(excluded_mirrors
            .exclude_list
            .contains(&ExcludeKind::Country("somecountry".to_string())));

        assert!(excluded_mirrors
            .exclude_list
            .contains(&ExcludeKind::CountryCode("sc".to_string())));
    }
}
