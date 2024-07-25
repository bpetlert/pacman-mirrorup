use std::{
    ops::{Deref, DerefMut},
    path::Path,
    sync::LazyLock,
};

use anyhow::{Context, Result};
use regex::{Regex, RegexSet};
use url::Url;

use crate::mirror::Mirror;

#[derive(Debug, Eq, PartialEq)]
pub enum ExcludeKind {
    Ignore,
    Domain(String),
    NegateDomain(String),
    Country(String),
    NegateCountry(String),
    CountryCode(String),
    NegateCountryCode(String),
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

        static EXCLUDE_SET_RE: LazyLock<RegexSet> = LazyLock::new(|| {
            RegexSet::new([
                r"(?P<negate>!?)domain\s*=\s*(?P<domain>\S*)", // Domain
                r"(?P<negate>!?)country\s*=\s*(?P<country>\S*)", // Country
                r"(?P<negate>!?)country_code\s*=\s*(?P<country_code>\S*)", // Country Code
            ])
            .expect("Create exclude regex set")
        });

        static EXCLUDE_CAPTURE_RE: LazyLock<Vec<Regex>> = LazyLock::new(|| {
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
            let cap = EXCLUDE_CAPTURE_RE[DOMAIN].captures(&line).unwrap();
            if cap["negate"].is_empty() {
                return Ok(ExcludeKind::Domain(cap["domain"].to_string()));
            } else {
                return Ok(ExcludeKind::NegateDomain(cap["domain"].to_string()));
            }
        } else if matches.matched(COUNTRY) {
            let cap = EXCLUDE_CAPTURE_RE[COUNTRY].captures(&line).unwrap();
            if cap["negate"].is_empty() {
                return Ok(ExcludeKind::Country(cap["country"].to_string()));
            } else {
                return Ok(ExcludeKind::NegateCountry(cap["country"].to_string()));
            }
        } else if matches.matched(COUNTRY_CODE) {
            let cap = EXCLUDE_CAPTURE_RE[COUNTRY_CODE].captures(&line).unwrap();
            if cap["negate"].is_empty() {
                return Ok(ExcludeKind::CountryCode(cap["country_code"].to_string()));
            } else {
                return Ok(ExcludeKind::NegateCountryCode(
                    cap["country_code"].to_string(),
                ));
            }
        }

        // When no keyword found, return domain as default
        //
        if let Some(domain) = line.strip_prefix('!') {
            return Ok(ExcludeKind::NegateDomain(domain.to_string()));
        }
        Ok(ExcludeKind::Domain(line))
    }
}

#[derive(Debug, PartialEq)]
pub struct ExcludedMirrors(Vec<ExcludeKind>);

impl ExcludedMirrors {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add(&mut self, exclude: ExcludeKind) {
        if exclude != ExcludeKind::Ignore {
            self.push(exclude);
        }
    }

    pub fn add_from(&mut self, file: &Path) -> Result<()> {
        let excluded_list = std::fs::read_to_string(file)
            .with_context(|| format!("Could not open excluded mirror file `{}`", file.display()))?;

        for line in excluded_list.lines() {
            let exclude = ExcludeKind::try_from(line)?;
            self.add(exclude);
        }

        Ok(())
    }

    pub fn is_exclude(&self, mirror: &Mirror) -> bool {
        let domain_name = Url::parse(&mirror.url)
            .unwrap()
            .domain()
            .unwrap()
            .to_lowercase();
        let country = mirror.country.to_lowercase();
        let country_code = mirror.country_code.to_lowercase();

        for exclude_kind in self.iter().rev() {
            match exclude_kind {
                ExcludeKind::Ignore => continue,
                ExcludeKind::Domain(d) if d == &domain_name => return true,
                ExcludeKind::NegateDomain(d) if d == &domain_name => return false,
                ExcludeKind::Country(c) if c == &country => return true,
                ExcludeKind::NegateCountry(c) if c == &country => return false,
                ExcludeKind::CountryCode(cc) if cc == &country_code => return true,
                ExcludeKind::NegateCountryCode(cc) if cc == &country_code => return false,
                _ => continue,
            }
        }

        false
    }
}

impl Deref for ExcludedMirrors {
    type Target = Vec<ExcludeKind>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ExcludedMirrors {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_parse_exclude_kind() {
        // Comment
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

        // Space
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

        // Negate
        assert_eq!(
            ExcludeKind::try_from("!domain = ban.this.mirror").unwrap(),
            ExcludeKind::NegateDomain("ban.this.mirror".to_string())
        );
        assert_eq!(
            ExcludeKind::try_from("!country = SomeCountry").unwrap(),
            ExcludeKind::NegateCountry("somecountry".to_string())
        );
        assert_eq!(
            ExcludeKind::try_from("!country_code = SC").unwrap(),
            ExcludeKind::NegateCountryCode("sc".to_string())
        );
        assert_eq!(
            ExcludeKind::try_from("!ban.this.mirror").unwrap(),
            ExcludeKind::NegateDomain("ban.this.mirror".to_string())
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
                "excluded_mirrors"
            )))
            .unwrap();

        assert_eq!(excluded_mirrors.len(), 8);

        assert_eq!(
            *excluded_mirrors.deref(),
            vec![
                ExcludeKind::Domain("ban.this.mirror".to_string()),
                ExcludeKind::Domain("ban.this-mirror.also".to_string()),
                ExcludeKind::Domain("ban.this.mirror.too".to_string()),
                ExcludeKind::Domain("ban.this-mirror.too.really".to_string()),
                ExcludeKind::NegateDomain("this.mirror.is.not.ban".to_string()),
                ExcludeKind::Country("somecountry".to_string()),
                ExcludeKind::CountryCode("sc".to_string()),
                ExcludeKind::NegateDomain("mirror2.in.somecountry".to_string()),
            ]
        );
    }

    #[test]
    fn test_is_exclude() {
        let mut excluded_mirrors = ExcludedMirrors::new();
        excluded_mirrors
            .add_from(Path::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/",
                "tests/",
                "excluded_mirrors"
            )))
            .unwrap();

        // This mirror is included
        let mut mirror = Mirror::default();
        mirror.url = "https://this.mirror.is.not.ban/".to_string();
        mirror.country = "SomeCountryA".to_string();
        mirror.country_code = "SCA".to_string();
        assert!(!excluded_mirrors.is_exclude(&mirror));

        // Ban by country
        let mut mirror1 = Mirror::default();
        mirror1.url = "https://mirror1.in.somecountry/".to_string();
        mirror1.country = "SomeCountry".to_string();
        mirror1.country_code = "SC".to_string();
        assert!(excluded_mirrors.is_exclude(&mirror1));

        // This mirror is included
        let mut mirror2 = Mirror::default();
        mirror2.url = "https://mirror2.in.somecountry/".to_string();
        mirror2.country = "SomeCountry".to_string();
        mirror2.country_code = "SC".to_string();
        assert!(!excluded_mirrors.is_exclude(&mirror2));
    }
}
