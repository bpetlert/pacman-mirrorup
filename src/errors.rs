use reqwest;
use serde_json;
use std::{error::Error as StdError, fmt, path::PathBuf};
use csv;

#[derive(Debug)]
pub enum ErrorKind {
    /// Generic error
    Msg(String),

    /// An error happened while fetching mirrors status data
    FetchMirrorStatus(reqwest::Error),

    /// An error happened while deserializing JSON
    Json(serde_json::Error),

    /// Output file already exists
    FileAlreadyExists(PathBuf),

    /// I/O error
    IoError(std::io::Error),

    /// CSV error
    CsvError(csv::Error),
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    source: Option<Box<dyn StdError>>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::Msg(ref message) => fmt::Display::fmt(message, f),
            ErrorKind::FetchMirrorStatus(ref e) => fmt::Display::fmt(e, f),
            ErrorKind::Json(ref e) => fmt::Display::fmt(e, f),
            ErrorKind::FileAlreadyExists(ref path) => fmt::Display::fmt(path.to_str().unwrap(), f),
            ErrorKind::IoError(ref e) => fmt::Display::fmt(e, f),
            ErrorKind::CsvError(ref e) => fmt::Display::fmt(e, f),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_ref().map(|c| &**c)
    }
}

impl Error {
    /// Creates generic error
    pub fn msg(value: impl ToString) -> Self {
        Self {
            kind: ErrorKind::Msg(value.to_string()),
            source: None,
        }
    }

    /// Creates fetch mirror-status error
    pub fn fetch_mirror_status(value: reqwest::Error) -> Self {
        Self {
            kind: ErrorKind::FetchMirrorStatus(value),
            source: None,
        }
    }

    /// Creates JSON error
    pub fn json(value: serde_json::Error) -> Self {
        Self {
            kind: ErrorKind::Json(value),
            source: None,
        }
    }

    /// Creates file already exists error
    pub fn file_already_exists(value: PathBuf) -> Self {
        Self {
            kind: ErrorKind::FileAlreadyExists(value),
            source: None,
        }
    }

    /// Creates I/O error
    pub fn io_error(value: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::IoError(value),
            source: None,
        }
    }

    /// Creates CSV error
    pub fn csv_error(value: csv::Error) -> Self {
        Self {
            kind: ErrorKind::CsvError(value),
            source: None,
        }
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Self::msg(e)
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Self::msg(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::fetch_mirror_status(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::json(e)
    }
}

impl From <std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::io_error(e)
    }
}

impl From<csv::Error> for Error {
    fn from (e: csv::Error) -> Self {
        Self::csv_error(e)
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;
