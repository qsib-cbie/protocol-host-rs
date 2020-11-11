use std::fmt;
use std::num::{ParseFloatError, ParseIntError};
use std::string::FromUtf8Error;

pub type Result<T> = std::result::Result<T, InternalError>;

#[derive(Debug)]
pub enum InternalError {
    Generic(String),
    IoError(std::io::Error),
    BoxError(Box<dyn std::error::Error>),
    ParseUtf8(FromUtf8Error),
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    SerdeError(serde_json::error::Error),
    #[cfg(feature = "usb")]
    UsbError(libusb::Error),
    ZmqError(zmq::Error),
    HexError(hex::FromHexError),
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InternalError::Generic(ref e) => {
                log::error!("Failed with error: {}", e);
                e.fmt(f)
            },
            InternalError::IoError(ref e) => {
                e.fmt(f)
            },
            InternalError::BoxError(ref e) => {
                e.fmt(f)
            },
            InternalError::ParseUtf8(ref e) => {
                log::error!("Failed to parse utf8 string");
                e.fmt(f)
            },
            InternalError::ParseFloat(ref e) => {
                log::error!("Failed to parse input as floating point value");
                e.fmt(f)
            },
            InternalError::ParseInt(ref e) => {
                log::error!("Failed to parse input as integer value");
                e.fmt(f)
            },
            InternalError::SerdeError(ref e) => {
                log::error!("Encountered serde error: {}", e);
                e.fmt(f)
            },
            #[cfg(feature = "usb")]
            InternalError::UsbError(ref e) => {
                log::error!("Encountered usb error: {}", e);
                e.fmt(f)
            },
            InternalError::ZmqError(ref e) => {
                log::error!("Encountered zmq error: {}", e);
                e.fmt(f)
            },
            InternalError::HexError(ref e) => {
                log::error!("Encountered hex error: {}", e);
                e.fmt(f)
            },
        }
    }
}

impl From<&str> for InternalError {
    fn from(err: &str) -> InternalError {
        InternalError::Generic(String::from(err))
    }
}

impl From<std::string::String> for InternalError {
    fn from(err: String) -> InternalError {
        InternalError::Generic(err)
    }
}

impl From<FromUtf8Error> for InternalError {
    fn from(err: FromUtf8Error) -> InternalError {
        InternalError::ParseUtf8(err)
    }
}

impl From<ParseFloatError> for InternalError {
    fn from(err: ParseFloatError) -> InternalError {
        InternalError::ParseFloat(err)
    }
}

impl From<ParseIntError> for InternalError {
    fn from(err: ParseIntError) -> InternalError {
        InternalError::ParseInt(err)
    }
}

impl From<std::io::Error> for InternalError {
    fn from(err: std::io::Error) -> InternalError {
        InternalError::IoError(err)
    }
}

impl From<Box<dyn std::error::Error>> for InternalError {
    fn from(err: Box<dyn std::error::Error>) -> InternalError {
        InternalError::BoxError(err)
    }
}

impl From<serde_json::error::Error> for InternalError {
    fn from(err: serde_json::error::Error) -> InternalError {
        InternalError::SerdeError(err)
    }
}

#[cfg(feature = "usb")]
impl From<libusb::Error> for InternalError {
    fn from(err: libusb::Error) -> InternalError {
        InternalError::UsbError(err)
    }
}

impl From<zmq::Error> for InternalError {
    fn from(err: zmq::Error) -> InternalError {
        InternalError::ZmqError(err)
    }
}

impl From<hex::FromHexError> for InternalError {
    fn from(err: hex::FromHexError) -> InternalError {
        InternalError::HexError(err)
    }
}