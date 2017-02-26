//! Functions in the module can be used to convert a `Data` variant into the respective recorded
//! representation according to the VBus Recording File Format.

use byteorder::{ByteOrder, LittleEndian};
use chrono::{DateTime, TimeZone, UTC};

use data::Data;
use header::Header;


/// Returns the number of bytes that the recorded representation of the Data needs.
pub fn length_from_data(data: &Data) -> usize {
    match *data {
        Data::Packet(ref packet) => 26 + packet.frame_count as usize * 4,
        Data::Datagram(_) => 26 + 6,
        Data::Telegram(ref tgram) => 26 + tgram.frame_count() as usize * 7,
    }
}


/// Stores the timestamp in the provided byte slice.
pub fn bytes_from_timestamp(timestamp: DateTime<UTC>, buf: &mut [u8]) {
    let timestamp_s = timestamp.timestamp();
    let timestamp_ms = timestamp.timestamp_subsec_millis();
    let timestamp = timestamp_s * 1000 + timestamp_ms as i64;

    LittleEndian::write_i64(&mut buf [0..8], timestamp);
}


/// Stores the record header in the provided byte slice.
pub fn bytes_from_record(typ: u8, length: u16, timestamp: DateTime<UTC>, buf: &mut [u8]) {
    buf [0] = 0xA5;
    buf [1] = typ;
    LittleEndian::write_u16(&mut buf [2..4], length);
    LittleEndian::write_u16(&mut buf [4..6], length);
    bytes_from_timestamp(timestamp, &mut buf [6..14]);
}


/// Stores a "VBus channel marker" record in the provided byte slice.
pub fn bytes_from_channel(channel: u8, buf: &mut [u8]) {
    bytes_from_record(0x77, 16, UTC.timestamp(0, 0), buf);
    buf [14] = channel;
    buf [15] = 0;
}


/// Stores the recorded representation of the Data in the provided byte slice.
pub fn bytes_from_data(data: &Data, buf: &mut [u8]) {
    let length = length_from_data(data);

    {
        let header: &Header = data.as_ref();
        bytes_from_record(0x66, length as u16, header.timestamp, buf);
        LittleEndian::write_u16(&mut buf [14..16], header.destination_address);
        LittleEndian::write_u16(&mut buf [16..18], header.source_address);
        buf [18] = header.protocol_version;
        buf [19] = 0;
    }

    match *data {
        Data::Packet(ref packet) => {
            let frame_data_length = packet.frame_count as usize * 4;

            LittleEndian::write_u16(&mut buf [20..22], packet.command);
            LittleEndian::write_u16(&mut buf [22..24], frame_data_length as u16);
            buf [24] = 0;
            buf [25] = 0;
            buf [26..(26 + frame_data_length)].copy_from_slice(&packet.frame_data [0..frame_data_length]);
        }
        Data::Datagram(ref dgram) => {
            LittleEndian::write_u16(&mut buf [20..22], dgram.command);
            buf [22] = 6;
            buf [23] = 0;
            buf [24] = 0;
            buf [25] = 0;
            LittleEndian::write_i16(&mut buf [26..28], dgram.param16);
            LittleEndian::write_i32(&mut buf [28..32], dgram.param32);
        }
        Data::Telegram(ref tgram) => {
            let frame_data_length = tgram.frame_count() as usize * 7;

            buf [20] = tgram.command;
            buf [21] = 0;
            LittleEndian::write_u16(&mut buf [22..24], frame_data_length as u16);
            buf [24] = 0;
            buf [25] = 0;
            buf [26..(26 + frame_data_length)].copy_from_slice(&tgram.frame_data [0..frame_data_length]);
        }
    }
}


#[cfg(test)]
mod tests {
    use chrono::{TimeZone, UTC};

    use super::*;

    use recording_decoder::data_from_checked_bytes;

    use test_data::{RECORDING_1, RECORDING_3};
    use test_utils::to_hex_string;

    #[test]
    fn test_length_from_data() {
        let channel = 0x11;

        let data1 = data_from_checked_bytes(channel, &RECORDING_1 [100..]);

        assert_eq!(134, length_from_data(&data1));

        let data2 = data_from_checked_bytes(channel, &RECORDING_3 [0..]);

        assert_eq!(32, length_from_data(&data2));

        // NOTE(daniel): no official Telegram recording found...
    }

    #[test]
    fn test_bytes_from_timestamp() {
        let timestamp = UTC.timestamp(1485688933, 0);

        let mut buf = [0u8; 8];

        bytes_from_timestamp(timestamp, &mut buf);
        assert_eq!("880af6e959010000", to_hex_string(&buf));
    }

    #[test]
    fn test_bytes_from_record() {
        let timestamp = UTC.timestamp(1485688933, 0);

        let mut buf = [0u8; 14];

        bytes_from_record(0x66, 134, timestamp, &mut buf);
        assert_eq!("a56686008600880af6e959010000", to_hex_string(&buf));
    }

    #[test]
    fn test_bytes_from_channel() {
        let mut buf = [0u8; 16];

        bytes_from_channel(0x11, &mut buf);
        assert_eq!("a5771000100000000000000000001100", to_hex_string(&buf));
    }

    #[test]
    fn test_bytes_from_data() {
        let channel = 0x11;

        let mut buf = [0u8; 1024];

        let data1 = data_from_checked_bytes(channel, &RECORDING_1 [100..]);

        bytes_from_data(&data1, &mut buf);
        assert_eq!(&RECORDING_1 [100..234], &buf [0..134]);

        let data2 = data_from_checked_bytes(channel, &RECORDING_3 [0..]);

        bytes_from_data(&data2, &mut buf);
        assert_eq!(&RECORDING_3 [0..32], &buf [0..32]);

        // NOTE(daniel): no official Telegram recording found...
    }
}
