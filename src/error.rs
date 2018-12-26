use std::{
    error::Error as StdError,
    fmt,
};

/// A common error type.
#[derive(Debug)]
pub struct Error {
    description: String,
}

impl Error {
    /// Construct a new `Error` using the provided description.
    pub fn new<T: Into<String>>(description: T) -> Error {
        Error {
            description: description.into(),
        }
    }

    /// Construct a new `Error` using the provided cause's description.
    pub fn from_cause<T: StdError>(cause: T) -> Error {
        Error {
            description: cause.description().to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl StdError for Error {}

macro_rules! from_other_error {
    ($type:path) => {
        impl From<$type> for Error {
            fn from(cause: $type) -> Error {
                Error::new(cause.description().to_string())
            }
        }
    };
}

from_other_error!(::std::io::Error);

/// A common result type.
pub type Result<T> = std::result::Result<T, Error>;
