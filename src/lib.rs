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

pub mod utils;

mod stream_blob_length;
pub use stream_blob_length::StreamBlobLength;

mod blob_reader;
pub use blob_reader::BlobReader;

mod header;
pub use header::Header;

mod packet;
pub use packet::Packet;

mod datagram;
pub use datagram::Datagram;

mod telegram;
pub use telegram::Telegram;

mod data;
pub use data::Data;

mod data_set;
pub use data_set::DataSet;

pub mod live_data_decoder;

pub mod live_data_encoder;

mod live_data_reader;
pub use live_data_reader::LiveDataReader;

mod live_data_writer;
pub use live_data_writer::LiveDataWriter;

pub mod recording_decoder;

mod recording_reader;
pub use recording_reader::RecordingReader;

mod specification_file;
pub use specification_file::*;
