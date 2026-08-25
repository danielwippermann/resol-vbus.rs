//! Error and `Result` types, re-exported from the [`anyhow`] crate.
//!
//! Instead of defining a custom `Error` type, this crate re-exports the
//! [`anyhow::Error`] type provided by the [`anyhow`] crate. It can wrap any
//! error that implements [`std::error::Error`] and is convenient for carrying
//! error context through the various reader/writer APIs of this crate.

pub use anyhow::{Error, Result};
