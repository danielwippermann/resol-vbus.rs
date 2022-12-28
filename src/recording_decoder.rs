//! Functions in this module allow to decode a byte stream conforming to the VBus Recording
//! File Format.

use byteorder::{ByteOrder, LittleEndian};
use chrono::{DateTime, Utc};

use crate::{
    data::Data,
    datagram::Datagram,
    header::Header,
    packet::Packet,
    stream_blob_length::StreamBlobLength::{self, BlobLength, Malformed, Partial},
    utils::utc_timestamp_with_nsecs,
    Telegram,
};

/// Checks the provided slice of bytes whether it contains a valid VBus record.
pub fn length_from_bytes(buf: &[u8]) -> StreamBlobLength {
    let len = buf.len();
    if len < 1 {
        Partial
    } else if buf[0] != 0xA5 {
        Malformed
    } else if len < 14 {
        Partial
    } else if (buf[1] >> 4) != (buf[1] & 0x0F) || buf[2] != buf[4] || buf[3] != buf[5] {
        Malformed
    } else {
        let expected_len = LittleEndian::read_u16(&buf[2..4]) as usize;
        if expected_len < 14 {
            Malformed
        } else if len < expected_len {
            Partial
        } else {
            BlobLength(expected_len)
        }
    }
}

/// Convert slice of bytes to `DateTime<Utc>` object.
pub fn timestamp_from_checked_bytes(buf: &[u8]) -> DateTime<Utc> {
    let timestamp_ms = LittleEndian::read_i64(&buf[0..8]);
    let timestamp_s = timestamp_ms / 1000;
    let timestamp_ns = (timestamp_ms % 1000) as u32 * 1_000_000;
    utc_timestamp_with_nsecs(timestamp_s, timestamp_ns)
}

/// Convert slice of bytes to respective `Data` variant.
pub fn data_from_checked_bytes(channel: u8, buf: &[u8]) -> Data {
    let timestamp = timestamp_from_checked_bytes(&buf[6..14]);
    let destination_address = LittleEndian::read_u16(&buf[14..16]);
    let source_address = LittleEndian::read_u16(&buf[16..18]);
    let protocol_version = buf[18];
    let major = protocol_version & 0xF0;

    if major == 0x10 {
        let command = LittleEndian::read_u16(&buf[20..22]);
        let frame_data_length = LittleEndian::read_u16(&buf[22..24]) as usize;
        let frame_count = (frame_data_length >> 2) as u8;

        let mut frame_data = [0u8; 508];
        frame_data[0..frame_data_length].copy_from_slice(&buf[26..26 + frame_data_length]);

        Data::Packet(Packet {
            header: Header {
                timestamp,
                channel,
                destination_address,
                source_address,
                protocol_version,
            },
            command,
            frame_count,
            frame_data,
        })
    } else if major == 0x20 {
        let command = LittleEndian::read_u16(&buf[20..22]);
        let param16 = LittleEndian::read_i16(&buf[26..28]);
        let param32 = LittleEndian::read_i32(&buf[28..32]);

        Data::Datagram(Datagram {
            header: Header {
                timestamp,
                channel,
                destination_address,
                source_address,
                protocol_version,
            },
            command,
            param16,
            param32,
        })
    } else if major == 0x30 {
        let command = buf[20];
        let frame_data_length = LittleEndian::read_u16(&buf[22..24]) as usize;

        let mut frame_data = [0u8; 21];
        frame_data[0..frame_data_length].copy_from_slice(&buf[26..26 + frame_data_length]);

        Data::Telegram(Telegram {
            header: Header {
                timestamp,
                channel,
                destination_address,
                source_address,
                protocol_version,
            },
            command,
            frame_data,
        })
    } else {
        panic!("Unhandled protocol version 0x{protocol_version:02}");
    }
}

/// Convert slice of bytes to respective `Data` variant.
pub fn data_from_bytes(channel: u8, buf: &[u8]) -> Option<Data> {
    match length_from_bytes(buf) {
        BlobLength(length) => {
            if length < 20 || buf[1] != 0x66 {
                None
            } else {
                let protocol_version = buf[18];
                let major = protocol_version & 0xF0;

                if major == 0x10 {
                    if length < 26 {
                        None
                    } else {
                        let frame_data_length = LittleEndian::read_u16(&buf[22..24]) as usize;
                        if length < 26 + frame_data_length {
                            None
                        } else {
                            Some(data_from_checked_bytes(channel, buf))
                        }
                    }
                } else if major == 0x20 {
                    if length < 32 {
                        None
                    } else {
                        Some(data_from_checked_bytes(channel, buf))
                    }
                } else if major == 0x30 {
                    if length < 26 {
                        None
                    } else {
                        let frame_data_length = LittleEndian::read_u16(&buf[22..24]) as usize;
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

    use crate::test_data::{RECORDING_1, RECORDING_3, TELEGRAM_RECORDING_1};

    #[test]
    fn test_length_from_bytes() {
        assert_eq!(Partial, length_from_bytes(&[]));
        assert_eq!(Malformed, length_from_bytes(&[0x00]));
        assert_eq!(
            Partial,
            length_from_bytes(&[
                0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x00, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00
            ])
        );
        assert_eq!(
            Malformed,
            length_from_bytes(&[
                0xA5, 0x43, 0x0E, 0x00, 0x0E, 0x00, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00, 0x00
            ])
        );
        assert_eq!(
            Malformed,
            length_from_bytes(&[
                0xA5, 0x44, 0x0E, 0x00, 0x0F, 0x00, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00, 0x00
            ])
        );
        assert_eq!(
            Malformed,
            length_from_bytes(&[
                0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x01, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00, 0x00
            ])
        );
        assert_eq!(
            Malformed,
            length_from_bytes(&[
                0xA5, 0x44, 0x0D, 0x00, 0x0D, 0x00, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00, 0x00
            ])
        );
        assert_eq!(
            Partial,
            length_from_bytes(&[
                0xA5, 0x44, 0x10, 0x00, 0x10, 0x00, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00, 0x00
            ])
        );
        assert_eq!(
            BlobLength(14),
            length_from_bytes(&[
                0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x00, 0x31, 0x47, 0xA9, 0x82, 0x59, 0x01, 0x00, 0x00
            ])
        );

        assert_eq!(BlobLength(14), length_from_bytes(&RECORDING_1[0..]));
        assert_eq!(BlobLength(70), length_from_bytes(&RECORDING_1[14..]));
        assert_eq!(BlobLength(16), length_from_bytes(&RECORDING_1[84..]));
        assert_eq!(BlobLength(134), length_from_bytes(&RECORDING_1[100..]));
        assert_eq!(BlobLength(30), length_from_bytes(&RECORDING_1[234..]));
        assert_eq!(BlobLength(66), length_from_bytes(&RECORDING_1[264..]));
        assert_eq!(BlobLength(82), length_from_bytes(&RECORDING_1[330..]));
        assert_eq!(BlobLength(82), length_from_bytes(&RECORDING_1[412..]));
        assert_eq!(BlobLength(82), length_from_bytes(&RECORDING_1[494..]));
        assert_eq!(BlobLength(82), length_from_bytes(&RECORDING_1[576..]));
        assert_eq!(BlobLength(82), length_from_bytes(&RECORDING_1[658..]));
        assert_eq!(740, RECORDING_1.len());

        assert_eq!(BlobLength(32), length_from_bytes(&RECORDING_3[0..]));
        assert_eq!(BlobLength(32), length_from_bytes(&RECORDING_3[32..]));
        assert_eq!(BlobLength(32), length_from_bytes(&RECORDING_3[64..]));
        assert_eq!(BlobLength(32), length_from_bytes(&RECORDING_3[96..]));
        assert_eq!(BlobLength(32), length_from_bytes(&RECORDING_3[128..]));
        assert_eq!(BlobLength(32), length_from_bytes(&RECORDING_3[160..]));
        assert_eq!(BlobLength(32), length_from_bytes(&RECORDING_3[192..]));
        assert_eq!(224, RECORDING_3.len());
    }

    #[test]
    fn test_timestamp_from_checked_bytes() {
        let timestamp = timestamp_from_checked_bytes(&RECORDING_1[20..]);
        assert_eq!("2017-01-09T09:57:28.975+00:00", timestamp.to_rfc3339());

        let timestamp = timestamp_from_checked_bytes(&RECORDING_1[106..]);
        assert_eq!("2017-01-09T09:57:27.880+00:00", timestamp.to_rfc3339());

        let timestamp = timestamp_from_checked_bytes(&RECORDING_1[240..]);
        assert_eq!("2017-01-09T09:57:28.765+00:00", timestamp.to_rfc3339());

        let timestamp = timestamp_from_checked_bytes(&RECORDING_1[270..]);
        assert_eq!("2017-01-09T09:57:28.764+00:00", timestamp.to_rfc3339());

        let timestamp = timestamp_from_checked_bytes(&RECORDING_1[336..]);
        assert_eq!("2017-01-09T09:57:08.893+00:00", timestamp.to_rfc3339());

        let timestamp = timestamp_from_checked_bytes(&RECORDING_1[418..]);
        assert_eq!("2017-01-09T09:57:13.901+00:00", timestamp.to_rfc3339());

        let timestamp = timestamp_from_checked_bytes(&RECORDING_1[500..]);
        assert_eq!("2017-01-09T09:57:17.894+00:00", timestamp.to_rfc3339());

        let timestamp = timestamp_from_checked_bytes(&RECORDING_1[582..]);
        assert_eq!("2017-01-09T09:57:21.797+00:00", timestamp.to_rfc3339());

        let timestamp = timestamp_from_checked_bytes(&RECORDING_1[664..]);
        assert_eq!("2017-01-09T09:57:26.080+00:00", timestamp.to_rfc3339());
    }

    #[test]
    fn test_data_from_checked_bytes() {
        let data = data_from_checked_bytes(0x00, &RECORDING_1[14..]);
        assert_eq!(
            "2017-01-09T09:57:28.975+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("00_0010_0053_10_0100", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_1[100..]);
        assert_eq!(
            "2017-01-09T09:57:27.880+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_0010_7E11_10_0100", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_1[234..]);
        assert_eq!(
            "2017-01-09T09:57:28.765+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_0010_7E21_10_0100", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_1[264..]);
        assert_eq!(
            "2017-01-09T09:57:28.764+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_0015_7E11_10_0100", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_1[330..]);
        assert_eq!(
            "2017-01-09T09:57:08.893+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_6651_7E11_10_0200", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_1[412..]);
        assert_eq!(
            "2017-01-09T09:57:13.901+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_6652_7E11_10_0200", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_1[494..]);
        assert_eq!(
            "2017-01-09T09:57:17.894+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_6653_7E11_10_0200", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_1[576..]);
        assert_eq!(
            "2017-01-09T09:57:21.797+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_6654_7E11_10_0200", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_1[658..]);
        assert_eq!(
            "2017-01-09T09:57:26.080+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_6655_7E11_10_0200", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_3[0..32]);
        assert_eq!(
            "2017-02-20T09:52:11.644+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_0000_7E11_20_0900_18F8", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_3[32..64]);
        assert_eq!(
            "2017-02-20T10:38:11.793+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_0000_7E11_20_0900_18F8", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_3[64..96]);
        assert_eq!(
            "2017-02-20T11:39:42.753+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_0000_7E11_20_0900_0052", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_3[96..128]);
        assert_eq!(
            "2017-02-20T11:40:27.573+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_0000_7E11_20_0900_0036", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_3[128..160]);
        assert_eq!(
            "2017-02-20T11:40:34.934+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_0000_7E11_20_0900_0042", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_3[160..192]);
        assert_eq!(
            "2017-02-20T11:50:06.273+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_0000_7E11_20_0900_18F8", data.id_string());

        let data = data_from_checked_bytes(0x01, &RECORDING_3[192..224]);
        assert_eq!(
            "2017-02-20T12:56:01.229+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_0000_7E11_20_0900_18F8", data.id_string());

        let data = data_from_checked_bytes(0x01, &TELEGRAM_RECORDING_1[0..]);
        assert_eq!(
            "2017-01-29T11:22:13+00:00",
            data.as_header().timestamp.to_rfc3339()
        );
        assert_eq!("01_7771_2011_30_25", data.id_string());
    }

    #[test]
    #[should_panic(expected = "Unhandled protocol version 0x00")]
    fn test_data_from_checked_bytes_panic() {
        let bytes: &[u8] = &[
            0xA5, 0x66, 0x21, 0x00, 0x21, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        data_from_checked_bytes(0x00, bytes);
    }

    #[test]
    fn test_data_from_bytes() {
        // Packet record too short for header
        let bytes: &[u8] = &[
            /* 0 - 5 */ 0xA5, 0x66, 0x14, 0x00, 0x14, 0x00, /* 6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 19 */ 0x00, 0x00, 0x00, 0x00, 0x10,
            0x00,
        ];

        assert_eq!(None, data_from_bytes(0x00, bytes));

        // Packet record too short for payload
        let bytes: &[u8] = &[
            /* 0 - 5 */ 0xA5, 0x66, 0x1A, 0x00, 0x1A, 0x00, /* 6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 25 */ 0x00, 0x00, 0x00, 0x00, 0x10,
            0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
        ];

        assert_eq!(None, data_from_bytes(0x00, bytes));

        // Datagram record too short
        let bytes: &[u8] = &[
            /* 0 - 5 */ 0xA5, 0x66, 0x14, 0x00, 0x14, 0x00, /* 6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 19 */ 0x00, 0x00, 0x00, 0x00, 0x20,
            0x00,
        ];

        assert_eq!(None, data_from_bytes(0x00, bytes));

        // Telegram record too short for header
        let bytes: &[u8] = &[
            /* 0 - 5 */ 0xA5, 0x66, 0x14, 0x00, 0x14, 0x00, /* 6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 19 */ 0x00, 0x00, 0x00, 0x00, 0x30,
            0x00,
        ];

        assert_eq!(None, data_from_bytes(0x00, bytes));

        // Telegram record too short for payload
        let bytes: &[u8] = &[
            /* 0 - 5 */ 0xA5, 0x66, 0x1A, 0x00, 0x1A, 0x00, /* 6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 25 */ 0x00, 0x00, 0x00, 0x00, 0x30,
            0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
        ];

        assert_eq!(None, data_from_bytes(0x00, bytes));

        // Unknown protocol version record
        let bytes: &[u8] = &[
            /* 0 - 5 */ 0xA5, 0x66, 0x1A, 0x00, 0x1A, 0x00, /* 6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 25 */ 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
        ];

        assert_eq!(None, data_from_bytes(0x00, bytes));

        assert_eq!(None, data_from_bytes(0x00, &RECORDING_1[0..]));
        assert_eq!(None, data_from_bytes(0x00, &RECORDING_1[14..34]));
        assert_eq!(
            "00_0010_0053_10_0100",
            data_from_bytes(0x00, &RECORDING_1[14..])
                .unwrap()
                .id_string()
        );
        assert_eq!(None, data_from_bytes(0x00, &RECORDING_1[84..]));
        assert_eq!(
            "01_0010_7E11_10_0100",
            data_from_bytes(0x01, &RECORDING_1[100..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_0010_7E21_10_0100",
            data_from_bytes(0x01, &RECORDING_1[234..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_0015_7E11_10_0100",
            data_from_bytes(0x01, &RECORDING_1[264..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_6651_7E11_10_0200",
            data_from_bytes(0x01, &RECORDING_1[330..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_6652_7E11_10_0200",
            data_from_bytes(0x01, &RECORDING_1[412..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_6653_7E11_10_0200",
            data_from_bytes(0x01, &RECORDING_1[494..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_6654_7E11_10_0200",
            data_from_bytes(0x01, &RECORDING_1[576..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_6655_7E11_10_0200",
            data_from_bytes(0x01, &RECORDING_1[658..])
                .unwrap()
                .id_string()
        );

        assert_eq!(
            "01_0000_7E11_20_0900_18F8",
            data_from_bytes(0x01, &RECORDING_3[0..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_0000_7E11_20_0900_18F8",
            data_from_bytes(0x01, &RECORDING_3[32..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_0000_7E11_20_0900_0052",
            data_from_bytes(0x01, &RECORDING_3[64..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_0000_7E11_20_0900_0036",
            data_from_bytes(0x01, &RECORDING_3[96..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_0000_7E11_20_0900_0042",
            data_from_bytes(0x01, &RECORDING_3[128..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_0000_7E11_20_0900_18F8",
            data_from_bytes(0x01, &RECORDING_3[160..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_0000_7E11_20_0900_18F8",
            data_from_bytes(0x01, &RECORDING_3[192..])
                .unwrap()
                .id_string()
        );
        assert_eq!(
            "01_7771_2011_30_25",
            data_from_bytes(0x01, &TELEGRAM_RECORDING_1[0..])
                .unwrap()
                .id_string()
        );
    }
}
