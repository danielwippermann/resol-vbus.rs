// This is part of resol-vbus.rs.
// Copyright (c) 2017-2019, Daniel Wippermann.
// See README.md and LICENSE.txt for details.

//! # resol-vbus.rs
//!
//! A Rust library for processing RESOL VBus data.
//!
//!
//! ## Features
//!
//! - Provides types for different VBus data versions
//! - Processes live and recorded VBus data streams
//! - Converts binary VBus data into human or machine readable format
//!
//!
//! ## Planned, but not yet implemented features
//!
//! - Improve filtering and conversion of VBus data fields
//!
//!
//! ## Supported Devices & Services
//!
//! * [All current RESOL controllers with VBus](http://www.resol.de/index/produkte/sprache/en)
//! * [RESOL DL2 Datalogger](http://www.resol.de/index/produktdetail/kategorie/2/sprache/en/id/12)
//! * [RESOL DL3 Datalogger](http://www.resol.de/index/produktdetail/kategorie/2/sprache/en/id/86)
//! * [RESOL VBus/LAN interface adapter](http://www.resol.de/index/produktdetail/kategorie/2/id/76/sprache/en)
//! * [RESOL VBus/USB interface adapter](http://www.resol.de/index/produktdetail/kategorie/2/id/13/sprache/en)
//! * [RESOL VBus.net](http://www.vbus.net/)
//!
//!
//! ## Technical Information & Specifications
//!
//! * [RESOL VBus Google Group](https://groups.google.com/forum/#!forum/resol-vbus)
//! * [RESOL VBus Protocol Specification](http://danielwippermann.github.io/resol-vbus/vbus-specification.html)
//! * [RESOL VBus Packet List](http://danielwippermann.github.io/resol-vbus/vbus-packets.html)
//! * [RESOL VBus Recording File Format](http://danielwippermann.github.io/resol-vbus/vbus-recording-file-format.html)
//! * [RESOL VBus Specification File Format v1](http://danielwippermann.github.io/resol-vbus/vbus-specification-file-format-v1.html)
//! * [RESOL VBus over TCP Specification](http://danielwippermann.github.io/resol-vbus/vbus-over-tcp.html)
//! * [RESOL DL2 (v1) Data Download API](https://drive.google.com/file/d/0B4wMTuLGRPi2YmM5ZTJiNDQtNjkyMi00ZWYzLTgzYzgtYTdiMjBlZmI5ODgx/edit?usp=sharing)
//! * [RESOL DL2 (v2) & DL3 Data Download API](http://danielwippermann.github.io/resol-vbus/dlx-data-download-api.html)
//!
//!
//! ### Converter for recorded VBus data to CSV.
//!
//! ```rust
//! //! A converter from the binary recorded VBus file format to human-readable CSV.
//! extern crate resol_vbus;
//!
//! use std::fs::File;
//! use std::io::{Read, Write};
//!
//! use resol_vbus::*;
//! use resol_vbus::chrono::{Local};
//!
//!
//! /// Load the VSF file to allow decoding of the payload contained in `Packet` `frame_data` values.
//! fn load_vsf_file(vsf_filename: &str) -> Specification {
//!     let mut f = File::open(vsf_filename).unwrap();
//!
//!     let mut buf = Vec::new();
//!     let size = f.read_to_end(&mut buf).unwrap();
//!
//!     let spec_file = SpecificationFile::from_bytes(&buf [0..size]).unwrap();
//!
//!     let spec = Specification::from_file(spec_file, Language::En);
//!
//!     spec
//! }
//!
//!
//! /// Read the "*.vbus" files a second time to convert / process the `DataSet` values within.
//! fn print_data(file_list: Vec<String>, topology_data_set: DataSet, spec: &Specification) {
//!     let flr = FileListReader::new(file_list);
//!
//!     let mut rr = LiveDataRecordingReader::new(flr);
//!
//!     let mut cumultative_data_set = topology_data_set;
//!
//!     let mut output = std::io::stdout();
//!
//!     while let Some(data) = rr.read_data().unwrap() {
//!         let timestamp = data.as_ref().timestamp;
//!         let local_timestamp = timestamp.with_timezone(&Local);
//!
//!         cumultative_data_set.add_data(data);
//!
//!         write!(output, "{}", local_timestamp).unwrap();
//!
//!         for field in spec.fields_in_data_set(&cumultative_data_set.as_ref()) {
//!             write!(output, "\t{}", field.fmt_raw_value(false)).unwrap();
//!         }
//!
//!         write!(output, "\n").unwrap();
//!     }
//! }
//!
//!
//! fn main() {
//!     let vsf_filename = "res/vbus_specification.vsf";
//!
//!     let file_list = std::env::args().skip(1).map(|arg| arg.to_owned()).collect::<Vec<_>>();
//!
//!     // Load the VSF file to allow decoding of `Packet` values.
//!     let spec = load_vsf_file(vsf_filename);
//!
//!     // Read the "*.vbus" files once to find all unique `Packet` values. This allows to
//!     // only generate CSV column headers once and keep the column layout stable throughout
//!     // the conversion.
//!     let flr = FileListReader::new(file_list.clone());
//!
//!     let mut rr = LiveDataRecordingReader::new(flr);
//!
//!     let topology_data_set = rr.read_topology_data_set().unwrap();
//!     for data in topology_data_set.as_data_slice() {
//!         println!("{}: {:?}", data.id_string(), data);
//!     }
//!
//!     // Read the "*.vbus" files a second time to convert / process the `DataSet` values within.
//!     print_data(file_list, topology_data_set, &spec);
//! }
//! ```

#![warn(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(warnings)]
#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(rust_2021_compatibility)]

pub use chrono;

#[cfg(test)]
mod test_data;

#[cfg(test)]
mod test_utils;

mod blob_buffer;
mod blob_reader;
mod data;
mod data_set;
mod datagram;
mod error;
mod file_list_reader;
mod header;
mod id_hash;
mod live_data_buffer;
pub mod live_data_decoder;
pub mod live_data_encoder;
mod live_data_reader;
mod live_data_recording_reader;
mod live_data_recording_writer;
mod live_data_writer;
mod packet;
pub mod recording_decoder;
pub mod recording_encoder;
mod recording_reader;
mod recording_writer;
pub mod specification;
pub mod specification_file;
mod stream_blob_length;
mod telegram;
pub mod utils;

pub use crate::{
    blob_buffer::BlobBuffer,
    blob_reader::BlobReader,
    data::Data,
    data_set::DataSet,
    datagram::Datagram,
    error::{Error, Result},
    file_list_reader::FileListReader,
    header::Header,
    id_hash::{id_hash, IdHash},
    live_data_buffer::LiveDataBuffer,
    live_data_reader::LiveDataReader,
    live_data_recording_reader::LiveDataRecordingReader,
    live_data_recording_writer::LiveDataRecordingWriter,
    live_data_writer::LiveDataWriter,
    packet::{Packet, PacketFieldId, PacketId, ToPacketFieldId, ToPacketId},
    recording_reader::RecordingReader,
    recording_writer::RecordingWriter,
    specification::Specification,
    specification_file::{Language, SpecificationFile},
    stream_blob_length::StreamBlobLength,
    telegram::Telegram,
};
