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


mod stream_blob_length;
pub use stream_blob_length::StreamBlobLength;
