use libc::c_int;
use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    String(String),
    NulError(std::ffi::NulError),
    Errno(c_int),
    IntoStringError(std::ffi::IntoStringError),
    ReadlineError(rustyline::error::ReadlineError),
    ParseIntError(std::num::ParseIntError),
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::String(_) => None,
            Error::NulError(ref e) => Some(e),
            Error::Errno(_) => None,
            Error::IntoStringError(ref e) => Some(e),
            Error::ReadlineError(ref e) => Some(e),
            Error::ParseIntError(ref e) => Some(e),
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
            Error::ReadlineError(ref e) => e.fmt(f),
            Error::ParseIntError(ref e) => e.fmt(f),
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

impl From<rustyline::error::ReadlineError> for Error {
    fn from(e: rustyline::error::ReadlineError) -> Error {
        Error::ReadlineError(e)
    }
}

impl From<std::ffi::IntoStringError> for Error {
    fn from(e: std::ffi::IntoStringError) -> Error {
        Error::IntoStringError(e)
    }
}
