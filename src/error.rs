use std::{error, fmt};
use libc::c_int;

#[derive(Debug)]
pub enum Error {
    String(String),
    NulError(std::ffi::NulError),
    Errno(c_int),
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::String(_) => None,
            Error::NulError(ref e) => Some(e),
            Error::Errno(_) => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::String(ref s) => f.write_str(s),
            Error::NulError(ref e) => e.fmt(f),
            Error::Errno(errno) => write!(f, "errno {}", errno),
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

impl From<std::ffi::NulError> for Error {
    fn from(e: std::ffi::NulError) -> Error {
        Error::NulError(e)
    }
}