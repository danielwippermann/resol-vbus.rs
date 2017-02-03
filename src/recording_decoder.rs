//! Functions in this module allow to decode a byte stream conforming to the VBus Recording
//! File Format.

use byteorder::{ByteOrder, LittleEndian};
use chrono::{TimeZone, UTC};

use stream_blob_length::StreamBlobLength::{self, BlobLength, Partial, Malformed};
use header::Header;
use packet::Packet;
use data::Data;


/// Checks the provided slice of bytes whether it contains a valid VBus record.
pub fn length_from_bytes(buf: &[u8]) -> StreamBlobLength {
    let len = buf.len();
    if len < 1 {
        Partial
    } else if buf [0] != 0xA5 {
        Malformed
    } else if len < 14 {
        Partial
    } else if (buf [1] >> 4) != (buf [1] & 0x0F) {
        Malformed
    } else if buf [2] != buf [4] {
        Malformed
    } else if buf [3] != buf [5] {
        Malformed
    } else {
        let expected_len = LittleEndian::read_u16(&buf [2..4]) as usize;
        if expected_len < 14 {
            Malformed
        } else if len < expected_len {
            Partial
        } else {
            BlobLength(expected_len)
        }
    }
}


/// Convert slice of bytes to respective `Data` variant.
pub fn data_from_checked_bytes(channel: u8, buf: &[u8]) -> Data {
    let timestamp_ms = LittleEndian::read_i64(&buf [6..14]);
    let destination_address = LittleEndian::read_u16(&buf [14..16]);
    let source_address = LittleEndian::read_u16(&buf [16..18]);
    let protocol_version = buf [18];
    let major = protocol_version & 0xF0;

    if major == 0x10 {
        let command = LittleEndian::read_u16(&buf [20..22]);
        let frame_data_length = LittleEndian::read_u16(&buf [22..24]) as usize;
        let frame_count = (frame_data_length >> 2) as u8;

        let mut frame_data = [ 0u8; 508 ];
        frame_data [0..frame_data_length].copy_from_slice(&buf [26..26 + frame_data_length]);

        Data::Packet(Packet {
            header: Header {
                timestamp: UTC.timestamp(timestamp_ms / 1000, (timestamp_ms % 1000) as u32 * 1000000),
                channel: channel,
                destination_address: destination_address,
                source_address: source_address,
                protocol_version: protocol_version,
            },
            command: command,
            frame_count: frame_count,
            frame_data: frame_data,
        })
    } else {
        panic!("Unhandled protocol version {}", protocol_version);
    }
}


/// Convert slice of bytes to respective `Data` variant.
pub fn data_from_bytes(channel: u8, buf: &[u8]) -> Option<Data> {
    match length_from_bytes(buf) {
        BlobLength(length) => {
            if length < 20 {
                None
            } else if buf [1] != 0x66 {
                None
            } else {
                let protocol_version = buf [18];
                let major = protocol_version & 0xF0;

                if major == 0x10 {
                    if length < 26 {
                        None
                    } else {
                        let frame_data_length = LittleEndian::read_u16(&buf [22..24]) as usize;
                        if length < 26 + frame_data_length {
                            None
                        } else {
                            Some(data_from_checked_bytes(channel, buf))
                        }
                    }
                } else {
                    None
                }
            }
        }
        _ => None,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use test_data::{RECORDING_1};

    #[test]
    fn test_length_from_bytes() {
        assert_eq!(Partial, length_from_bytes(&[]));
        assert_eq!(Malformed, length_from_bytes(&[ 0x00 ]));
        assert_eq!(Partial, length_from_bytes(&[ 0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x00, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00 ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xA5, 0x43, 0x0E, 0x00, 0x0E, 0x00, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00, 0x00 ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xA5, 0x44, 0x0E, 0x00, 0x0F, 0x00, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00, 0x00 ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x01, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00, 0x00 ]));
        assert_eq!(BlobLength(14), length_from_bytes(&[ 0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x00, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00, 0x00 ]));

        assert_eq!(BlobLength(14), length_from_bytes(&RECORDING_1 [0..]));
        assert_eq!(BlobLength(70), length_from_bytes(&RECORDING_1 [14..]));
        assert_eq!(BlobLength(16), length_from_bytes(&RECORDING_1 [84..]));
        assert_eq!(BlobLength(134), length_from_bytes(&RECORDING_1 [100..]));
        assert_eq!(BlobLength(30), length_from_bytes(&RECORDING_1 [234..]));
        assert_eq!(BlobLength(66), length_from_bytes(&RECORDING_1 [264..]));
        assert_eq!(BlobLength(82), length_from_bytes(&RECORDING_1 [330..]));
        assert_eq!(BlobLength(82), length_from_bytes(&RECORDING_1 [412..]));
        assert_eq!(BlobLength(82), length_from_bytes(&RECORDING_1 [494..]));
        assert_eq!(BlobLength(82), length_from_bytes(&RECORDING_1 [576..]));
        assert_eq!(BlobLength(82), length_from_bytes(&RECORDING_1 [658..]));
        assert_eq!(740, RECORDING_1.len());
    }

    #[test]
    fn test_data_from_checked_bytes() {
        assert_eq!("00_0010_0053_10_0100", data_from_checked_bytes(0x00, &RECORDING_1 [14..]).to_id_string());
        assert_eq!("01_0010_7E11_10_0100", data_from_checked_bytes(0x01, &RECORDING_1 [100..]).to_id_string());
        assert_eq!("01_0010_7E21_10_0100", data_from_checked_bytes(0x01, &RECORDING_1 [234..]).to_id_string());
        assert_eq!("01_0015_7E11_10_0100", data_from_checked_bytes(0x01, &RECORDING_1 [264..]).to_id_string());
        assert_eq!("01_6651_7E11_10_0200", data_from_checked_bytes(0x01, &RECORDING_1 [330..]).to_id_string());
        assert_eq!("01_6652_7E11_10_0200", data_from_checked_bytes(0x01, &RECORDING_1 [412..]).to_id_string());
        assert_eq!("01_6653_7E11_10_0200", data_from_checked_bytes(0x01, &RECORDING_1 [494..]).to_id_string());
        assert_eq!("01_6654_7E11_10_0200", data_from_checked_bytes(0x01, &RECORDING_1 [576..]).to_id_string());
        assert_eq!("01_6655_7E11_10_0200", data_from_checked_bytes(0x01, &RECORDING_1 [658..]).to_id_string());
    }

    #[test]
    fn test_data_from_bytes() {
        assert_eq!(None, data_from_bytes(0x00, &RECORDING_1 [0..]));
        assert_eq!("00_0010_0053_10_0100", data_from_bytes(0x00, &RECORDING_1 [14..]).unwrap().to_id_string());
        assert_eq!(None, data_from_bytes(0x00, &RECORDING_1 [84..]));
        assert_eq!("01_0010_7E11_10_0100", data_from_bytes(0x01, &RECORDING_1 [100..]).unwrap().to_id_string());
        assert_eq!("01_0010_7E21_10_0100", data_from_bytes(0x01, &RECORDING_1 [234..]).unwrap().to_id_string());
        assert_eq!("01_0015_7E11_10_0100", data_from_bytes(0x01, &RECORDING_1 [264..]).unwrap().to_id_string());
        assert_eq!("01_6651_7E11_10_0200", data_from_bytes(0x01, &RECORDING_1 [330..]).unwrap().to_id_string());
        assert_eq!("01_6652_7E11_10_0200", data_from_bytes(0x01, &RECORDING_1 [412..]).unwrap().to_id_string());
        assert_eq!("01_6653_7E11_10_0200", data_from_bytes(0x01, &RECORDING_1 [494..]).unwrap().to_id_string());
        assert_eq!("01_6654_7E11_10_0200", data_from_bytes(0x01, &RECORDING_1 [576..]).unwrap().to_id_string());
        assert_eq!("01_6655_7E11_10_0200", data_from_bytes(0x01, &RECORDING_1 [658..]).unwrap().to_id_string());
    }
}
