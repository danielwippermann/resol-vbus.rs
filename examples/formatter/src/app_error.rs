use std;
use chrono;
use resol_vbus;


#[derive(Debug)]
pub enum AppError {
    GenericString(String),
    IoError(std::io::Error),
    NumParseIntError(std::num::ParseIntError),
    ChronoParseError(chrono::ParseError),
    ResolVBusSpecFileError(resol_vbus::specification_file::Error),
}


impl From<String> for AppError {
    fn from(err: String) -> AppError {
        AppError::GenericString(err)
    }
}


impl<'a> From<&'a str> for AppError {
    fn from(err: &'a str) -> AppError {
        AppError::GenericString(err.to_owned())
    }
}


impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> AppError {
        AppError::IoError(err)
    }
}


impl From<std::num::ParseIntError> for AppError {
    fn from(err: std::num::ParseIntError) -> AppError {
        AppError::NumParseIntError(err)
    }
}


impl From<chrono::ParseError> for AppError {
    fn from(err: chrono::ParseError) -> AppError {
        AppError::ChronoParseError(err)
    }
}


impl From<resol_vbus::specification_file::Error> for AppError {
    fn from(err: resol_vbus::specification_file::Error) -> AppError {
        AppError::ResolVBusSpecFileError(err)
    }
}


pub type Result<T> = std::result::Result<T, AppError>;
