use std::{
    ops::{Deref, DerefMut},
    path::Path,
};

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::{Regex, RegexSet};
use url::Url;

use crate::mirror::Mirror;

#[derive(Debug, Eq, PartialEq)]
pub enum ExcludeKind {
    Ignore,
    Domain { value: String, negate: bool },
    Country { value: String, negate: bool },
    CountryCode { value: String, negate: bool },
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
                r"(?P<negate>!*)domain\s*=\s*(?P<domain>\S*)", // Domain
                r"(?P<negate>!*)country\s*=\s*(?P<country>\S*)", // Country
                r"(?P<negate>!*)country_code\s*=\s*(?P<country_code>\S*)", // Country Code
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
            let cap = EXCLUDE_CAPTURE_RE[DOMAIN].captures(&line).unwrap();
            return Ok(ExcludeKind::Domain {
                value: cap["domain"].to_string(),
                negate: !cap["negate"].is_empty(),
            });
        } else if matches.matched(COUNTRY) {
            let cap = EXCLUDE_CAPTURE_RE[COUNTRY].captures(&line).unwrap();
            return Ok(ExcludeKind::Country {
                value: cap["country"].to_string(),
                negate: !cap["negate"].is_empty(),
            });
        } else if matches.matched(COUNTRY_CODE) {
            let cap = EXCLUDE_CAPTURE_RE[COUNTRY_CODE].captures(&line).unwrap();
            return Ok(ExcludeKind::CountryCode {
                value: cap["country_code"].to_string(),
                negate: !cap["negate"].is_empty(),
            });
        }

        // When no keyword found, return domain as default
        //
        if let Some(domain) = line.strip_prefix('!') {
            return Ok(ExcludeKind::Domain {
                value: domain.to_string(),
                negate: true,
            });
        }
        Ok(ExcludeKind::Domain {
            value: line,
            negate: false,
        })
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
                ExcludeKind::Domain { value, negate } => {
                    if *value == domain_name {
                        return !negate;
                    }
                }
                ExcludeKind::Country { value, negate } => {
                    if *value == country {
                        return !negate;
                    }
                }
                ExcludeKind::CountryCode { value, negate } => {
                    if *value == country_code {
                        return !negate;
                    }
                }
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
    use super::*;
    use pretty_assertions::assert_eq;

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
            ExcludeKind::Domain {
                value: "ban.this.mirror".to_string(),
                negate: false
            }
        );
        assert_eq!(
            ExcludeKind::try_from("domain=ban.this.mirror # Comment").unwrap(),
            ExcludeKind::Domain {
                value: "ban.this.mirror".to_string(),
                negate: false
            }
        );

        // Space
        assert_eq!(
            ExcludeKind::try_from("domain = ban.this.mirror").unwrap(),
            ExcludeKind::Domain {
                value: "ban.this.mirror".to_string(),
                negate: false
            }
        );
        assert_eq!(
            ExcludeKind::try_from("domain = ban.this.mirror # Comment").unwrap(),
            ExcludeKind::Domain {
                value: "ban.this.mirror".to_string(),
                negate: false
            }
        );
        assert_eq!(
            ExcludeKind::try_from("country = SomeCountry").unwrap(),
            ExcludeKind::Country {
                value: "somecountry".to_string(),
                negate: false
            }
        );
        assert_eq!(
            ExcludeKind::try_from("country_code = SC").unwrap(),
            ExcludeKind::CountryCode {
                value: "sc".to_string(),
                negate: false
            }
        );

        // Without "domain="
        assert_eq!(
            ExcludeKind::try_from("ban.this.mirror").unwrap(),
            ExcludeKind::Domain {
                value: "ban.this.mirror".to_string(),
                negate: false
            }
        );
        assert_eq!(
            ExcludeKind::try_from("ban.this.mirror # Comment").unwrap(),
            ExcludeKind::Domain {
                value: "ban.this.mirror".to_string(),
                negate: false
            }
        );

        // Negate
        assert_eq!(
            ExcludeKind::try_from("!domain = ban.this.mirror").unwrap(),
            ExcludeKind::Domain {
                value: "ban.this.mirror".to_string(),
                negate: true
            }
        );
        assert_eq!(
            ExcludeKind::try_from("!country = SomeCountry").unwrap(),
            ExcludeKind::Country {
                value: "somecountry".to_string(),
                negate: true
            }
        );
        assert_eq!(
            ExcludeKind::try_from("!country_code = SC").unwrap(),
            ExcludeKind::CountryCode {
                value: "sc".to_string(),
                negate: true
            }
        );
        assert_eq!(
            ExcludeKind::try_from("!ban.this.mirror").unwrap(),
            ExcludeKind::Domain {
                value: "ban.this.mirror".to_string(),
                negate: true
            }
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
                ExcludeKind::Domain {
                    value: "ban.this.mirror".to_string(),
                    negate: false
                },
                ExcludeKind::Domain {
                    value: "ban.this-mirror.also".to_string(),
                    negate: false
                },
                ExcludeKind::Domain {
                    value: "ban.this.mirror.too".to_string(),
                    negate: false,
                },
                ExcludeKind::Domain {
                    value: "ban.this-mirror.too.really".to_string(),
                    negate: false,
                },
                ExcludeKind::Domain {
                    value: "this.mirror.is.not.ban".to_string(),
                    negate: true,
                },
                ExcludeKind::Country {
                    value: "somecountry".to_string(),
                    negate: false
                },
                ExcludeKind::CountryCode {
                    value: "sc".to_string(),
                    negate: false
                },
                ExcludeKind::Domain {
                    value: "mirror2.in.somecountry".to_string(),
                    negate: true,
                }
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
