//! Functions in this module can be used to decode byte slices of data conforming to the
//! VBus protocol specification into the respective `Data` variants.

use byteorder::{ByteOrder, LittleEndian};
use chrono::{DateTime, UTC};

use data::Data;
use datagram::Datagram;
use header::Header;
use packet::Packet;
use stream_blob_length::StreamBlobLength::{self, BlobLength, Partial, Malformed};
use telegram::Telegram;
use utils::{calc_and_compare_checksum_v0, copy_bytes_injecting_septett, has_msb_set};


/// Checks the provided slice of bytes whether it contains valid VBus live data.
pub fn length_from_bytes(buf: &[u8]) -> StreamBlobLength {
    let len = buf.len();
    if len < 1 {
        Partial
    } else if buf [0] != 0xAA {
        Malformed
    } else if len < 6 {
        Partial
    } else if has_msb_set(&buf [1..6]) {
        Malformed
    } else {
        let protocol_version = buf [5];
        let major = protocol_version & 0xF0;
        // let minor = protocol_version & 0x0F;

        if major == 0x10 {
            if len < 10 {
                Partial
            } else if has_msb_set(&buf [6..10]) {
                Malformed
            } else if !calc_and_compare_checksum_v0(&buf [1..10]) {
                Malformed
            } else {
                let frame_count = buf [8] as usize;
                let expected_len = 10 + frame_count * 6;
                if len < expected_len {
                    Partial
                } else if has_msb_set(&buf [10..expected_len]) {
                    Malformed
                } else {
                    let valid = (0..frame_count).all(|frame_idx| {
                        let frame_start = 10 + frame_idx * 6;
                        calc_and_compare_checksum_v0(&buf [frame_start..frame_start + 6])
                    });
                    if !valid {
                        Malformed
                    } else {
                        BlobLength(expected_len)
                    }
                }
            }
        } else if major == 0x20 {
            if len < 16 {
                Partial
            } else if has_msb_set(&buf [6..16]) {
                Malformed
            } else if !calc_and_compare_checksum_v0(&buf [1..16]) {
                Malformed
            } else {
                BlobLength(16)
            }
        } else if major == 0x30 {
            if len < 8 {
                Partial
            } else if has_msb_set(&buf [6..8]) {
                Malformed
            } else if !calc_and_compare_checksum_v0(&buf [1..8]) {
                Malformed
            } else {
                let frame_count = Telegram::frame_count_from_command(buf [6]) as usize;
                let expected_len = 8 + frame_count * 9;
                if len < expected_len {
                    Partial
                } else if has_msb_set(&buf [8..expected_len]) {
                    Malformed
                } else {
                    let valid = (0..frame_count).all(|frame_idx| {
                        let frame_start = 8 + frame_idx * 9;
                        calc_and_compare_checksum_v0(&buf [frame_start..frame_start + 9])
                    });
                    if !valid {
                        Malformed
                    } else {
                        BlobLength(expected_len)
                    }
                }
            }
        } else {
            Malformed
        }
    }
}


/// Convert slice of bytes to respective `Data` variant.
pub fn data_from_checked_bytes(timestamp: DateTime<UTC>, channel: u8, buf: &[u8]) -> Data {
    let protocol_version = buf [5];
    let major = protocol_version & 0xF0;

    let header = Header {
        timestamp: timestamp,
        channel: channel,
        destination_address: LittleEndian::read_u16(&buf [1..]),
        source_address: LittleEndian::read_u16(&buf [3..]),
        protocol_version: buf [5],
    };

    if major == 0x10 {
        let frame_count = buf [8] as usize;

        let mut frame_data = [0u8; 508];
        for frame_idx in 0..frame_count {
            let src_start = 10 + frame_idx * 6;
            let dst_start = frame_idx * 4;
            copy_bytes_injecting_septett(&mut frame_data [dst_start..dst_start + 4], &buf [src_start..src_start + 5]);
        }

        Data::Packet(Packet {
            header: header,
            command: LittleEndian::read_u16(&buf [6..]),
            frame_count: buf [8],
            frame_data: frame_data,
        })
    } else if major == 0x20 {
        let mut payload = [0u8; 6];
        copy_bytes_injecting_septett(&mut payload, &buf [8..15]);

        Data::Datagram(Datagram {
            header: header,
            command: LittleEndian::read_u16(&buf [6..]),
            param16: LittleEndian::read_i16(&payload [0..]),
            param32: LittleEndian::read_i32(&payload [2..]),
        })
    } else if major == 0x30 {
        let command = buf [6];
        let frame_count = Telegram::frame_count_from_command(command) as usize;

        let mut frame_data = [0u8; 21];
        for frame_idx in 0..frame_count {
            let src_start = 8 + frame_idx * 9;
            let dst_start = frame_idx * 7;
            copy_bytes_injecting_septett(&mut frame_data [dst_start..dst_start + 7], &buf [src_start..src_start + 8]);
        }

        Data::Telegram(Telegram {
            header: header,
            command: command,
            frame_data: frame_data,
        })
    } else {
        unreachable!();
    }
}


/// Convert slice of bytes to respective `Data` variant.
pub fn data_from_bytes(timestamp: DateTime<UTC>, channel: u8, buf: &[u8]) -> Option<Data> {
    match length_from_bytes(buf) {
        BlobLength(_) => Some(data_from_checked_bytes(timestamp, channel, buf)),
        Partial | Malformed => None,
    }
}


#[cfg(test)]
mod tests {
    use chrono::{TimeZone, UTC};

    use stream_blob_length::StreamBlobLength::{BlobLength, Partial, Malformed};

    use data::Data;

    use super::*;

    use test_data::{LIVE_DATA_1, LIVE_TELEGRAM_1};
    use test_utils::to_hex_string;

    #[test]
    fn test_length_from_bytes() {
        // version independent
        assert_eq!(Partial, length_from_bytes(&[]));
        assert_eq!(Malformed, length_from_bytes(&[ 0x00 ]));
        assert_eq!(Partial, length_from_bytes(&[ 0xAA, 0x10, 0x00, 0x11, 0x7E ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xAA, 0x10, 0x00, 0x11, 0x7E, 0xFF ]));

        // version 1.0
        assert_eq!(Partial, length_from_bytes(&[ 0xAA, 0x10, 0x00, 0x22, 0x7E, 0x10, 0x00, 0x01, 0x01 ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xAA, 0x10, 0x00, 0x22, 0x7E, 0x10, 0x00, 0x01, 0x81, 0x3D ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xAA, 0x10, 0x00, 0x22, 0x7E, 0x10, 0x00, 0x01, 0x01, 0x00 ]));
        assert_eq!(BlobLength(10), length_from_bytes(&[ 0xAA, 0x10, 0x00, 0x22, 0x7E, 0x10, 0x00, 0x01, 0x00, 0x3E ]));
        assert_eq!(Partial, length_from_bytes(&[ 0xAA, 0x10, 0x00, 0x22, 0x7E, 0x10, 0x00, 0x01, 0x01, 0x3D, 0x4B, 0x01, 0x0E, 0x00, 0x00 ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xAA, 0x10, 0x00, 0x22, 0x7E, 0x10, 0x00, 0x01, 0x01, 0x3D, 0x4B, 0x01, 0x0E, 0x00, 0x80, 0x25 ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xAA, 0x10, 0x00, 0x22, 0x7E, 0x10, 0x00, 0x01, 0x01, 0x3D, 0x4B, 0x01, 0x0E, 0x00, 0x00, 0x00 ]));
        assert_eq!(BlobLength(16), length_from_bytes(&[ 0xAA, 0x10, 0x00, 0x22, 0x7E, 0x10, 0x00, 0x01, 0x01, 0x3D, 0x4B, 0x01, 0x0E, 0x00, 0x00, 0x25 ]));

        // version 2.0
        assert_eq!(Partial, length_from_bytes(&[ 0xAA, 0x00, 0x00, 0x11, 0x7E, 0x20, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xAA, 0x00, 0x00, 0x11, 0x7E, 0x20, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x4B ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xAA, 0x00, 0x00, 0x11, 0x7E, 0x20, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 ]));
        assert_eq!(BlobLength(16), length_from_bytes(&[ 0xAA, 0x00, 0x00, 0x11, 0x7E, 0x20, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4B ]));

        // version 3.0
        assert_eq!(Partial, length_from_bytes(&[ 0xAA, 0x71, 0x77, 0x11, 0x20, 0x30, 0x25 ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xAA, 0x71, 0x77, 0x11, 0x20, 0x30, 0xA5, 0x11 ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xAA, 0x71, 0x77, 0x11, 0x20, 0x30, 0x25, 0x00 ]));
        assert_eq!(BlobLength(8), length_from_bytes(&[ 0xAA, 0x71, 0x77, 0x11, 0x20, 0x30, 0x05, 0x31 ]));
        assert_eq!(Partial, length_from_bytes(&[ 0xAA, 0x71, 0x77, 0x11, 0x20, 0x30, 0x25, 0x11, 0x60, 0x18, 0x2B, 0x04, 0x00, 0x00, 0x00, 0x04 ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xAA, 0x71, 0x77, 0x11, 0x20, 0x30, 0x25, 0x11, 0x60, 0x18, 0x2B, 0x04, 0x00, 0x00, 0x00, 0x84, 0x54 ]));
        assert_eq!(Malformed, length_from_bytes(&[ 0xAA, 0x71, 0x77, 0x11, 0x20, 0x30, 0x25, 0x11, 0x60, 0x18, 0x2B, 0x04, 0x00, 0x00, 0x00, 0x04, 0x00 ]));
        assert_eq!(BlobLength(17), length_from_bytes(&[ 0xAA, 0x71, 0x77, 0x11, 0x20, 0x30, 0x25, 0x11, 0x60, 0x18, 0x2B, 0x04, 0x00, 0x00, 0x00, 0x04, 0x54 ]));

        // test data
        assert_eq!(BlobLength(172), length_from_bytes(&LIVE_DATA_1 [0..]));
        assert_eq!(BlobLength(70), length_from_bytes(&LIVE_DATA_1 [172..]));
        assert_eq!(BlobLength(16), length_from_bytes(&LIVE_DATA_1 [242..]));
        assert_eq!(BlobLength(94), length_from_bytes(&LIVE_DATA_1 [258..]));
        assert_eq!(BlobLength(16), length_from_bytes(&LIVE_DATA_1 [352..]));
        assert_eq!(BlobLength(17), length_from_bytes(&LIVE_TELEGRAM_1 [0..]));
    }

    #[test]
    fn test_data_from_checked_bytes() {
        let timestamp = UTC.timestamp(1485688933, 0);
        let channel = 0x11;

        let data = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [0..]);

        if let Data::Packet(packet) = data {
            assert_eq!(timestamp, packet.header.timestamp);
            assert_eq!(channel, packet.header.channel);
            assert_eq!(0x0010, packet.header.destination_address);
            assert_eq!(0x7E11, packet.header.source_address);
            assert_eq!(0x10, packet.header.protocol_version);
            assert_eq!(0x0100, packet.command);
            assert_eq!(0x1B, packet.frame_count);
            assert_eq!(to_hex_string(&[
                0x37, 0x00, 0x1d, 0x01,  // 0x00, 0x2a,
                0x3d, 0x01, 0x24, 0x01,  // 0x00, 0x1c,
                0x07, 0x01, 0x09, 0x01,  // 0x00, 0x6d,
                0x02, 0x00, 0x37, 0x01,  // 0x00, 0x45,
                0x13, 0x02, 0xb8, 0x22,  // 0x04, 0x0c,
                0xb8, 0x22, 0xb8, 0x22,  // 0x05, 0x46,
                0x0f, 0x27, 0x0f, 0x27,  // 0x00, 0x13,
                0x0f, 0x27, 0x46, 0x05,  // 0x00, 0x7e,
                0x0f, 0x27, 0x0f, 0x27,  // 0x00, 0x13,
                0x0f, 0x27, 0x0f, 0x27,  // 0x00, 0x13,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
                0x0f, 0x27, 0x0f, 0x27,  // 0x00, 0x13,
                0x0f, 0x27, 0x0f, 0x27,  // 0x00, 0x13,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
                0x00, 0x64, 0x00, 0x00,  // 0x00, 0x1b,
                0x00, 0x00, 0x64, 0x00,  // 0x00, 0x1b,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
                0x4c, 0xf2, 0x1f, 0x1e,  // 0x02, 0x02,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
                0x00, 0x00, 0x00, 0x00,  // 0x00, 0x7f,
            ]), to_hex_string(&packet.frame_data [0..108]));
        } else {
            panic!("Expected {:?} to be a Packet", data);
        }
    }
}
