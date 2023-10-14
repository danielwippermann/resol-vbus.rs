//! Functions in the module can be used to convert a `Data` variant into the respective live
//! representation according to the VBus protocol specification.

use crate::{
    data::Data,
    little_endian::{i16_to_le_bytes, i32_to_le_bytes, u16_to_le_bytes},
    telegram::Telegram,
    utils::{calc_and_set_checksum_v0, copy_bytes_extracting_septett},
};

/// Returns the number of bytes that the live representation of the Data needs.
pub fn length_from_data(data: &Data) -> usize {
    match *data {
        Data::Packet(ref packet) => 10 + packet.frame_count as usize * 6,
        Data::Datagram(_) => 16,
        Data::Telegram(ref tgram) => {
            8 + Telegram::frame_count_from_command(tgram.command) as usize * 9
        }
    }
}

/// Stores the live representation of the Data in the provided byte slice.
pub fn bytes_from_data(data: &Data, buf: &mut [u8]) {
    match *data {
        Data::Packet(ref packet) => {
            buf[0] = 0xAA;
            u16_to_le_bytes(&mut buf[1..3], packet.header.destination_address);
            u16_to_le_bytes(&mut buf[3..5], packet.header.source_address);
            buf[5] = 0x10;
            u16_to_le_bytes(&mut buf[6..8], packet.command);
            buf[8] = packet.frame_count;
            calc_and_set_checksum_v0(&mut buf[1..10]);

            for frame_idx in 0..(packet.frame_count as usize) {
                let src_start = frame_idx * 4;
                let dst_start = 10 + frame_idx * 6;
                copy_bytes_extracting_septett(
                    &mut buf[dst_start..dst_start + 5],
                    &packet.frame_data[src_start..src_start + 4],
                );
                calc_and_set_checksum_v0(&mut buf[dst_start..dst_start + 6]);
            }
        }
        Data::Datagram(ref dgram) => {
            buf[0] = 0xAA;
            u16_to_le_bytes(&mut buf[1..3], dgram.header.destination_address);
            u16_to_le_bytes(&mut buf[3..5], dgram.header.source_address);
            buf[5] = 0x20;
            u16_to_le_bytes(&mut buf[6..8], dgram.command);
            let mut payload = [0u8; 6];
            i16_to_le_bytes(&mut payload[0..], dgram.param16);
            i32_to_le_bytes(&mut payload[2..], dgram.param32);
            copy_bytes_extracting_septett(&mut buf[8..15], &payload);
            calc_and_set_checksum_v0(&mut buf[1..16]);
        }
        Data::Telegram(ref tgram) => {
            buf[0] = 0xAA;
            u16_to_le_bytes(&mut buf[1..3], tgram.header.destination_address);
            u16_to_le_bytes(&mut buf[3..5], tgram.header.source_address);
            buf[5] = 0x30;
            buf[6] = tgram.command;
            calc_and_set_checksum_v0(&mut buf[1..8]);

            for frame_idx in 0..(tgram.frame_count() as usize) {
                let src_start = frame_idx * 7;
                let dst_start = 8 + frame_idx * 7;
                copy_bytes_extracting_septett(
                    &mut buf[dst_start..dst_start + 8],
                    &tgram.frame_data[src_start..src_start + 7],
                );
                calc_and_set_checksum_v0(&mut buf[dst_start..dst_start + 9]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        live_data_decoder::data_from_checked_bytes,
        test_data::{LIVE_DATA_1, LIVE_TELEGRAM_1},
        utils::utc_timestamp,
    };

    #[test]
    fn test_length_from_data() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let data1 = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[0..]);

        assert_eq!(172, length_from_data(&data1));

        let data2 = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[352..]);

        assert_eq!(16, length_from_data(&data2));

        let data3 = data_from_checked_bytes(timestamp, channel, &LIVE_TELEGRAM_1[0..]);

        assert_eq!(17, length_from_data(&data3));
    }

    #[test]
    fn test_bytes_from_data() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let data1 = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[0..]);
        let mut buf = [0u8; 1024];

        bytes_from_data(&data1, &mut buf);
        assert_eq!(&LIVE_DATA_1[0..172], &buf[0..172]);

        let data2 = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[352..]);

        bytes_from_data(&data2, &mut buf);
        assert_eq!(&LIVE_DATA_1[352..368], &buf[0..16]);

        let data3 = data_from_checked_bytes(timestamp, channel, &LIVE_TELEGRAM_1[0..]);

        bytes_from_data(&data3, &mut buf);
        assert_eq!(&LIVE_TELEGRAM_1[0..17], &buf[0..17]);
    }
}
