use std::{
    collections::HashSet,
    fs::File,
    io::{self, BufRead},
    path::Path,
};

use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use regex::{Regex, RegexSet};

const EXCLUDE_PATTERN: [&str; 3] = [
    r"domain\s*=\s*(?P<domain>\S*)",             // Domain
    r"country\s*=\s*(?P<country>\S*)",           // Country
    r"country_code\s*=\s*(?P<country_code>\S*)", // Country Code
];

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

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let set: &RegexSet = {
            static RE: OnceCell<RegexSet> = OnceCell::new();
            RE.get_or_try_init(|| RegexSet::new(EXCLUDE_PATTERN))
                .map_err(|err| anyhow!("{err:#?}"))
                .context("Could not create exclude parser regex")?
        };

        let regexes: &Vec<Regex> = {
            static RE: OnceCell<Vec<Regex>> = OnceCell::new();
            RE.get_or_try_init(|| -> Result<Vec<Regex>, _> {
                let reg: Vec<_> = set
                    .patterns()
                    .iter()
                    .map(|pat| Regex::new(pat).unwrap())
                    .collect();
                Ok::<Vec<regex::Regex>, Self::Error>(reg)
            })
            .map_err(|err: _| anyhow!("{err:#?}"))
            .context("Could not create exclude parser regex")?
        };

        if value.is_empty() {
            return Ok(ExcludeKind::Ignore);
        }

        // Remove comment
        let value = value
            .trim()
            .chars()
            .take_while(|c| *c != '#' && *c != ';')
            .collect::<String>();
        let value = value.trim();

        if value.is_empty() {
            return Ok(ExcludeKind::Ignore);
        }

        let matches = set.matches(value);

        if matches.matched(0) {
            let domain: String = regexes[0].captures(value).unwrap()["domain"].to_string();
            return Ok(ExcludeKind::Domain(domain.to_lowercase()));
        } else if matches.matched(1) {
            let country: String = regexes[1].captures(value).unwrap()["country"].to_string();
            return Ok(ExcludeKind::Country(country.to_lowercase()));
        } else if matches.matched(2) {
            let country_code: String =
                regexes[2].captures(value).unwrap()["country_code"].to_string();
            return Ok(ExcludeKind::CountryCode(country_code.to_lowercase()));
        }

        Ok(ExcludeKind::Domain(value.to_string().to_lowercase()))
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
        .into_iter()
        .filter_map(|line| line.ok())
        .collect();

        for line in lines {
            let exclude = ExcludeKind::try_from(line.as_str())?;
            self.add(exclude);
        }

        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.exclude_list.is_empty()
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
