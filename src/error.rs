use libc::c_int;
use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    String(String),
    NulError(std::ffi::NulError),
    Errno(c_int),
    IntoStringError(std::ffi::IntoStringError),
    ParseIntError(std::num::ParseIntError),
    GimliError(gimli::Error),
    IoError(std::io::Error),
    MpscRecvError(std::sync::mpsc::RecvError),
    RustylineError(rustyline::error::ReadlineError),
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::String(_) => None,
            Error::NulError(ref e) => Some(e),
            Error::Errno(_) => None,
            Error::IntoStringError(ref e) => Some(e),
            Error::ParseIntError(ref e) => Some(e),
            Error::GimliError(ref e) => Some(e),
            Error::IoError(ref e) => Some(e),
            Error::MpscRecvError(ref e) => Some(e),
            Error::RustylineError(ref e) => Some(e),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::String(ref s) => f.write_str(s),
            Error::NulError(ref e) => e.fmt(f),
            Error::Errno(errno) => write!(f, "errno {}", errno),
            Error::IntoStringError(ref e) => e.fmt(f),
            Error::ParseIntError(ref e) => e.fmt(f),
            Error::GimliError(ref e) => e.fmt(f),
            Error::IoError(ref e) => e.fmt(f),
            Error::MpscRecvError(ref e) => e.fmt(f),
            Error::RustylineError(ref e) => e.fmt(f),
        }
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Error {
        Error::String(s.to_owned())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Error {
        Error::String(s)
    }
}

impl From<gimli::Error> for Error {
    fn from(e: gimli::Error) -> Error {
        Error::GimliError(e)
    }
}

impl From<rustyline::error::ReadlineError> for Error {
    fn from(e: rustyline::error::ReadlineError) -> Error {
        Error::RustylineError(e)
    }
}

impl From<std::sync::mpsc::RecvError> for Error {
    fn from(e: std::sync::mpsc::RecvError) -> Error {
        Error::MpscRecvError(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::IoError(e)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Error {
        Error::ParseIntError(e)
    }
}

impl From<std::ffi::NulError> for Error {
    fn from(e: std::ffi::NulError) -> Error {
        Error::NulError(e)
    }
}

impl From<std::ffi::IntoStringError> for Error {
    fn from(e: std::ffi::IntoStringError) -> Error {
        Error::IntoStringError(e)
    }
}
