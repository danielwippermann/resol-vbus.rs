# resol-vbus.rs

A Rust library for processing RESOL VBus data.

[![Rust](https://github.com/danielwippermann/resol-vbus.rs/actions/workflows/test.yml/badge.svg)](https://github.com/danielwippermann/resol-vbus.rs/actions/workflows/test.yml)
[![codecov](https://codecov.io/github/danielwippermann/resol-vbus.rs/branch/master/graph/badge.svg?token=kBErkGDKrY)](https://codecov.io/github/danielwippermann/resol-vbus.rs)
[![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Fdanielwippermann%2Fresol-vbus.rs.svg?type=shield)](https://app.fossa.io/projects/git%2Bgithub.com%2Fdanielwippermann%2Fresol-vbus.rs?ref=badge_shield)

[Documentation](https://docs.rs/resol-vbus/)


## Usage

First, add this to your `Cargo.toml`:

```toml
[dependencies]
resol-vbus = "0.3"
```


## Changelog

### Work in progress for Version 0.3.0

- **[BREAKING CHANGE]**: Update chrono dependency and MSRV.
    In an effort to prepare and cleanup their API for the next semver release, the new version of chrono raised their MSRV and deprecated a couple of functions that were previously used by our examples and tests.


### Version 0.2.1

- Implement `std::io::{Read,Write}` for `BlobBuffer`.
- Add `get_{ref,mut}` functions to several `...Reader` structs.
- Add `utils::current_timestamp` function and use it.
- Add `SpecificationFile#convert_value` function.
- Add `SpecificationFile#unit_by_unit_code` function.
- Add some convenience functions to `Specification`.
- Fix timezone bug in `Specification::fmt_timestamp`.
- Update VBus specification file.
- Add channel support to `LiveDataRecordingReader`.
- Several bugfixes and other improvements to code and documentation.


### Version 0.2.0

- **[BREAKING CHANGE]**: Rename `to_id_string` to `id_string`.
- **[BREAKING CHANGE]**: Replace `Arc` with `Rc` in `Specification` type.
- **[BREAKING CHANGE]**: Reexport less from `specification` module, instead export itself.
- **[BREAKING CHANGE]**: Rename `get_power_of_10` to `power_of_ten_f64`.
- **[BREAKING CHANGE]**: Change `Specification`'s `raw_value` handling from `f64` to `i64`.
- **[BREAKING CHANGE]**: Rename `get_raw_value_{i64,f64}` methods to `raw_value_{i64,f64}`.
- Add `Specification::fields_in_data_set` function.
- Add `AsRef<Header>` and `AsRef<[Data]>` implementations.
- Add `DataSet::clear_packets_older_than` function.
- Add `BlobReader::to_inner` function.
- Add `specification::power_of_ten_i64` function.
- Add and use `RawValueFormatter` type with improved l10n support.
- Add `DataSet::clear_all_packets` function.
- Add `Data::{is,into}_{packet,datagram,telegram}` functions.
- Add `SpecificationFile::new_default` function.
- Add `Datagram` support to `recording_decoder` modules.
- Add `recording_encoder` module.
- Add `DataSet::{iter,iter_mut}` functions.
- Add `DataSet::sort_by` function.
- Add `RecordingWriter` type.
- Add `LiveDataRecordingWriter` type.
- Add `Specification::fmt_timestamp` function.
- Add `Data::as_{packet,datagram,telegram}` functions.
- Add `IdHash` trait and `id_hash` function.
- Add `IdHash` impls for `Packet`, `Datagram` and `Telegram` types.
- Add `IdHash` impls for `Data` and `DataSet` types.
- Add `DataSet#len` function.
- Add `Packet#valid_frame_data{,_mut,_len}` functions.
- Add `PacketId` and `PacketFieldId` types.
- Add `ToPacketId` impl for `Packet` type.
- Add `Specification#get_packet_spec_by_id` function.
- Add `PacketSpec#get_field_spec{,_by}_position` functions.
- Export `{,To}Packet{,Field}Id` types and traits as well as `chrono` mod.
- Add `Telegram#valid_frame_data{,_mut,_len}` functions.
- Add timestamp based filtering to `LiveDataRecordingReader`.
- Add `DataSet::remove_all_data` function.
- Add `DataSet::sort_by_id_slice` function.
- Add `DataSetPacketField::{packet_id,packet_field_id}` functions.
- Add `new` and `field_id` functions to `DataSetPacketField`.
- Add optional support for min and max timestamps for `RecordingReader`.
- Add `get_ref` and `get_mut` methods to several `...Writer` structs.
- Add `offset` method to `BlobReader`.
- Add `offset` methods to `LiveDataRecordingReader` and `RecordingReader`.
- Add `LiveDataRecordingReader#read_to_stats`.
- Add `BlobBuffer`.
- Add `LiveDataBuffer`.
- Add `AsRef` and `AsMut` impls to `LiveDataWriter`.
- Add `Error` and `Result` types and use them throughout the library.
- Add `Default` impl for `Header` struct.
- Update integrated VBus specification file to datecode 20181220.
- Update chrono dependency to 0.4.
- Update to Rust edition 2018, apply cargo-fmt and cargo-clippy.
- Several bugfixes and other improvements to code and documentation.


### Version 0.1.1

- **[BREAKING CHANGE]**: Publicly re-exported symbols from the `specification_file` module
    Previously we re-exported every symbol from the `specification_file` module on the crate level, unnecessarily cluttering the library's namespace. This has been corrected in this version, only re-exporting `SpecificationFile`, `Language` and exporting the `specification_file` module itself. Under normal circumstances this change would result in a semver version bump, but since 0.1.0 was only released today, I hope nobody else is impacted.
- Add `LiveDataRecordingReader`.


### Version 0.1.0

First public release.


## Contributors

- [Daniel Wippermann](https://github.com/danielwippermann)


## Legal Notices

RESOL, VBus, VBus.net and others are trademarks or registered trademarks of RESOL - Elektronische Regelungen GmbH.

All other trademarks are the property of their respective owners.


## License

`resol-vbus.rs` is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See LICENSE.txt for details.


[![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Fdanielwippermann%2Fresol-vbus.rs.svg?type=large)](https://app.fossa.io/projects/git%2Bgithub.com%2Fdanielwippermann%2Fresol-vbus.rs?ref=badge_large)