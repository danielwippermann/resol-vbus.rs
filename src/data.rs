use header::Header;
use packet::Packet;
use datagram::Datagram;
use telegram::Telegram;


/// `Data` is a type that contains one of the supported VBus protocol data variants.
#[derive(Debug)]
pub enum Data {
    /// Contains a `Packet` conforming to VBus protocol version 1.x.
    Packet(Packet),

    /// Contains a `Datagram` conforming to VBus protocol version 2.x.
    Datagram(Datagram),

    /// Contains a `Telegram` conforming to VBus protocol version 3.x.
    Telegram(Telegram),
}


impl Data {

    /// Returns the `Header` part of the variant inside this `Data`.
    pub fn as_header(&self) -> &Header {
        match *self {
            Data::Packet(ref packet) => &packet.header,
            Data::Datagram(ref dgram) => &dgram.header,
            Data::Telegram(ref tgram) => &tgram.header,
        }
    }

    /// Creates an ID string for the variant inside this `Data`.
    pub fn to_id_string(&self) -> String {
        match *self {
            Data::Packet(ref packet) => packet.to_id_string(),
            Data::Datagram(ref dgram) => dgram.to_id_string(),
            Data::Telegram(ref tgram) => tgram.to_id_string(),
        }
    }

}


#[cfg(test)]
mod tests {
    use chrono::{TimeZone, UTC};

    use live_data_decoder::data_from_checked_bytes;

    // use super::*;

    use test_data::{LIVE_DATA_1, LIVE_TELEGRAM_1};

    #[test]
    fn test_as_header() {
        let timestamp = UTC.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [0..]);

        let header = packet_data.as_header();
        assert_eq!(timestamp, header.timestamp);
        assert_eq!(channel, header.channel);
        assert_eq!(0x0010, header.destination_address);
        assert_eq!(0x7E11, header.source_address);
        assert_eq!(0x10, header.protocol_version);

        let dgram_data = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [352..]);

        let header = dgram_data.as_header();
        assert_eq!(timestamp, header.timestamp);
        assert_eq!(channel, header.channel);
        assert_eq!(0x0000, header.destination_address);
        assert_eq!(0x7E11, header.source_address);
        assert_eq!(0x20, header.protocol_version);

        let tgram_data = data_from_checked_bytes(timestamp, channel, &LIVE_TELEGRAM_1 [0..]);

        let header = tgram_data.as_header();
        assert_eq!(timestamp, header.timestamp);
        assert_eq!(channel, header.channel);
        assert_eq!(0x7771, header.destination_address);
        assert_eq!(0x2011, header.source_address);
        assert_eq!(0x30, header.protocol_version);
    }
}
