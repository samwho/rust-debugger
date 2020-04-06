use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    String(String),
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::String(_) => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::String(ref s) => f.write_str(s),
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