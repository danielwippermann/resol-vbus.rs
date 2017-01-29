// This is part of rust-resol-vbus.
// Copyright (c) 2017, Daniel Wippermann.
// See README.md and LICENSE.txt for details.

//! # rust-resol-vbus
//!
//! A Rust library for processing RESOL VBus data.
//!
//! ## Features
//!
//! - TBD

#![warn(missing_docs)]
#![deny(missing_debug_implementations)]
// #![deny(warnings)]

extern crate byteorder;
extern crate chrono;


#[cfg(test)]
mod test_data;

#[cfg(test)]
mod test_utils;

mod stream_blob_length;
pub use stream_blob_length::StreamBlobLength;

mod blob_reader;
pub use blob_reader::BlobReader;

mod header;
pub use header::Header;
