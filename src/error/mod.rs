use std::fmt;
use std::num::{ParseFloatError, ParseIntError};
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum CliError {
    Generic(String),
    IoError(std::io::Error),
    BoxError(Box<dyn std::error::Error>),
    ParseUtf8(FromUtf8Error),
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    SerdeError(serde_json::error::Error),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CliError::Generic(ref e) => {
                log::error!("Failed with error: {}", e);
                e.fmt(f)
            },
            CliError::IoError(ref e) => {
                e.fmt(f)
            },
            CliError::BoxError(ref e) => {
                e.fmt(f)
            },
            CliError::ParseUtf8(ref e) => {
                log::error!("Failed to parse utf8 string");
                e.fmt(f)
            },
            CliError::ParseFloat(ref e) => {
                log::error!("Failed to parse input as floating point value");
                e.fmt(f)
            },
            CliError::ParseInt(ref e) => {
                log::error!("Failed to parse input as integer value");
                e.fmt(f)
            },
            CliError::SerdeError(ref e) => {
                log::error!("Encountered serde error: {}", e);
                e.fmt(f)
            },

        }
    }
}

impl From<std::string::String> for CliError {
    fn from(err: String) -> CliError {
        CliError::Generic(err)
    }
}

impl From<FromUtf8Error> for CliError {
    fn from(err: FromUtf8Error) -> CliError {
        CliError::ParseUtf8(err)
    }
}

impl From<ParseFloatError> for CliError {
    fn from(err: ParseFloatError) -> CliError {
        CliError::ParseFloat(err)
    }
}

impl From<ParseIntError> for CliError {
    fn from(err: ParseIntError) -> CliError {
        CliError::ParseInt(err)
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> CliError {
        CliError::IoError(err)
    }
}

impl From<Box<dyn std::error::Error>> for CliError {
    fn from(err: Box<dyn std::error::Error>) -> CliError {
        CliError::BoxError(err)
    }
}

impl From<serde_json::error::Error> for CliError {
    fn from(err: serde_json::error::Error) -> CliError {
        CliError::SerdeError(err)
    }
}