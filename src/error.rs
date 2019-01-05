use std::{error::Error as StdError, fmt};

/// A common error type.
#[derive(Debug, PartialEq)]
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
            description: format!("{}", cause),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl StdError for Error {}

impl From<&str> for Error {
    fn from(cause: &str) -> Error {
        Error::new(cause)
    }
}

impl From<String> for Error {
    fn from(cause: String) -> Error {
        Error::new(cause)
    }
}

macro_rules! from_other_error {
    ($type:path) => {
        impl From<$type> for Error {
            fn from(cause: $type) -> Error {
                Error::from_cause(cause)
            }
        }
    };
}

from_other_error!(::std::io::Error);

/// A common result type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let error = Error::new("Test");

        assert_eq!("Test", &error.description);
    }

    #[test]
    fn test_from_cause() {
        let cause = Error::new("Cause");

        let error = Error::from_cause(cause);

        assert_eq!("Cause", &error.description);
    }

    #[test]
    fn test_display_fmt() {
        let error = Error::new("Test");

        let result = format!("{}", error);

        assert_eq!("Test", result);
    }

    #[test]
    fn test_from_other_error() {
        use std::io;

        let cause = io::Error::new(io::ErrorKind::Other, "Other error");

        let error = Error::from(cause);

        assert_eq!("Other error", &error.description);
    }
}
