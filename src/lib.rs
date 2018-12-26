// This is part of resol-vbus.rs.
// Copyright (c) 2017, Daniel Wippermann.
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
//! - Discovers LAN-enabled RESOL devices on the local network
//! - Allows to send parameterization commands to a controller
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
//! ## Examples
//!
//! ### Recorder of live VBus data into a persistent file format
//!
//! ```rust,no_run
//! //! A recorder of live VBus data into the binary recorded VBus file format.
//! extern crate resol_vbus;
//!
//!
//! use std::fs::File;
//! use std::net::TcpStream;
//!
//! use resol_vbus::*;
//!
//!
//!
//! fn main() {
//!     // Create a TCP connection to the DL2
//!     let stream = TcpStream::connect("192.168.178.101:7053").expect("Unable to connect to DL2");
//!
//!     // Use a `TcpConnector` to perform the login handshake into the DL2
//!     let mut connector = TcpConnector::new(stream);
//!     connector.password = "vbus".to_owned();
//!     connector.connect().expect("Unable to connect to DL2");
//!
//!     // Get back the original TCP connection and hand it to a `LiveDataReader`
//!     let stream = connector.into_inner();
//!     let mut ldr = LiveDataReader::new(0, stream);
//!
//!     // Create an recording file and hand it to a `RecordingWriter`
//!     let file = File::create("test.vbus").expect("Unable to create output file");
//!     let mut rw = RecordingWriter::new(file);
//!
//!     // Read VBus `Data` values from the `LiveDataReader`
//!     while let Some(data) = ldr.read_data().expect("Unable to read data") {
//!         println!("{}", data.id_string());
//!
//!         // Add `Data` value into `DataSet` to be stored
//!         let mut data_set = DataSet::new();
//!         data_set.timestamp = data.as_ref().timestamp;
//!         data_set.add_data(data);
//!
//!         // Write the `DataSet` into the `RecordingWriter` for permanent storage
//!         rw.write_data_set(&data_set).expect("Unable to write data set");
//!     }
//! }
//! ```
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
// #![deny(warnings)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::needless_bool)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::write_with_newline)]

extern crate byteorder;
pub extern crate chrono;

#[cfg(test)]
mod test_data;

#[cfg(test)]
mod test_utils;

mod error;
pub use error::{Error, Result};

pub mod utils;

mod stream_blob_length;
pub use stream_blob_length::StreamBlobLength;

mod blob_buffer;
pub use blob_buffer::BlobBuffer;

mod blob_reader;
pub use blob_reader::BlobReader;

mod id_hash;
pub use id_hash::{id_hash, IdHash};

mod header;
pub use header::Header;

mod packet;
pub use packet::{Packet, PacketFieldId, PacketId, ToPacketFieldId, ToPacketId};

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

mod live_data_buffer;
pub use live_data_buffer::LiveDataBuffer;

mod read_with_timeout;
pub use read_with_timeout::ReadWithTimeout;

mod live_data_reader;
pub use live_data_reader::LiveDataReader;

mod live_data_writer;
pub use live_data_writer::LiveDataWriter;

mod live_data_stream;
pub use live_data_stream::LiveDataStream;

pub mod recording_decoder;

pub mod recording_encoder;

mod recording_reader;
pub use recording_reader::RecordingReader;

mod recording_writer;
pub use recording_writer::RecordingWriter;

mod live_data_recording_reader;
pub use live_data_recording_reader::LiveDataRecordingReader;

mod live_data_recording_writer;
pub use live_data_recording_writer::LiveDataRecordingWriter;

pub mod specification_file;
pub use specification_file::{Language, SpecificationFile};

pub mod specification;
pub use specification::Specification;

mod file_list_reader;
pub use file_list_reader::FileListReader;

mod tcp_connector;
pub use tcp_connector::TcpConnector;
