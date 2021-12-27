use std::fmt;

#[derive(Debug)]
pub struct Error(String);

impl Error {
    pub fn new(description: String) -> Error {
        Error(description)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub trait ErrorCause: fmt::Display {}

impl<T: ErrorCause> From<T> for Error {
    fn from(cause: T) -> Error {
        Error::new(format!("{}", cause))
    }
}

impl ErrorCause for &'static str {}
impl ErrorCause for String {}
impl ErrorCause for std::io::Error {}
impl ErrorCause for std::num::ParseIntError {}
impl ErrorCause for resol_vbus::chrono::format::ParseError {}
impl ErrorCause for resol_vbus::Error {}

pub type Result<T> = std::result::Result<T, Error>;
