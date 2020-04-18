use std::fmt::{self, Display};
use std::time::Duration;
use std::{error, result};

pub type Result<T, E = Error> = result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    Extraction(ExtractionError, &'static str),
    Network(reqwest::Error),

    /// An error produced by accessors when a rate limit is exceeded.
    ///
    /// The duration given should reflect the wait time required.
    Wait(Duration),
}

#[derive(Debug)]
pub enum ExtractionError {
    Image,
    Metadata,
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Network(e) => Some(e),

            _ => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Extraction(e, message) => match e {
                ExtractionError::Image => write!(f, "Image extraction failure: {}", message),
                ExtractionError::Metadata => write!(f, "Metadata extraction failure: {}", message),
            },

            Error::Network(e) => e.fmt(f),
            Error::Wait(_) => f.write_str("Rate limit exceeded"),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Network(e)
    }
}
