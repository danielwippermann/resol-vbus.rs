use std::{
    cmp::Ordering::{self, Equal, Greater, Less},
    hash::Hasher,
};

use crate::{
    datagram::Datagram, header::Header, id_hash::IdHash, packet::Packet, telegram::Telegram,
};

/// `Data` is a type that contains one of the supported VBus protocol data variants.
///
/// # Examples
///
/// ```rust
/// use std::io::Read;
///
/// use resol_vbus::{LiveDataReader, Result};
///
/// # #[allow(dead_code)]
/// fn print_data_ids<R: Read>(r: R) -> Result<()> {
///     let mut ldr = LiveDataReader::new(0, r);
///
///     while let Some(data) = ldr.read_data()? {
///         if !data.is_packet() {
///             continue;
///         }
///
///         println!("{}: {}", data.as_header().timestamp, data.id_string());
///     }
///
///     Ok(())
/// }
/// ```
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum Data {
    /// Contains a `Packet` conforming to VBus protocol version 1.x.
    Packet(Packet),

    /// Contains a `Datagram` conforming to VBus protocol version 2.x.
    Datagram(Datagram),

    /// Contains a `Telegram` conforming to VBus protocol version 3.x.
    Telegram(Telegram),
}

impl Data {
    /// Returns `true` if the variant is a `Packet`.
    pub fn is_packet(&self) -> bool {
        matches!(*self, Data::Packet(_))
    }

    /// Returns `true` if the variant is a `Packet`.
    pub fn is_datagram(&self) -> bool {
        matches!(*self, Data::Datagram(_))
    }

    /// Returns `true` if the variant is a `Packet`.
    pub fn is_telegram(&self) -> bool {
        matches!(*self, Data::Telegram(_))
    }

    /// Returns the `Packet` value, consuming the `Data` value.
    ///
    /// # Panics
    ///
    /// The function panics if the `Data` value is no `Packet` variant.
    pub fn into_packet(self) -> Packet {
        match self {
            Data::Packet(packet) => packet,
            _ => panic!(
                "called `Data::into_packet` for a non-`Packet` value: {:?}",
                self
            ),
        }
    }

    /// Returns the `Datagram` value, consuming the `Data` value.
    ///
    /// # Panics
    ///
    /// The function panics if the `Data` value is no `Datagram` variant.
    pub fn into_datagram(self) -> Datagram {
        match self {
            Data::Datagram(datagram) => datagram,
            _ => panic!(
                "called `Data::into_datagram` for a non-`Datagram` value: {:?}",
                self
            ),
        }
    }

    /// Returns the `Telegram` value, consuming the `Data` value.
    ///
    /// # Panics
    ///
    /// The function panics if the `Data` value is no `Telegram` variant.
    pub fn into_telegram(self) -> Telegram {
        match self {
            Data::Telegram(telegram) => telegram,
            _ => panic!(
                "called `Data::into_telegram` for a non-`Telegram` value: {:?}",
                self
            ),
        }
    }

    /// Returns the `Header` part of the variant inside this `Data`.
    pub fn as_header(&self) -> &Header {
        self.as_ref()
    }

    /// Returns the `Packet` value.
    ///
    /// # Panics
    ///
    /// The function panics if the `Data` value is no `Packet` variant.
    pub fn as_packet(&self) -> &Packet {
        match *self {
            Data::Packet(ref packet) => packet,
            _ => panic!(
                "called `Data::as_packet` for a non-`Packet` value: {:?}",
                self
            ),
        }
    }

    /// Returns the `Datagram` value.
    ///
    /// # Panics
    ///
    /// The function panics if the `Data` value is no `Datagram` variant.
    pub fn as_datagram(&self) -> &Datagram {
        match *self {
            Data::Datagram(ref datagram) => datagram,
            _ => panic!(
                "called `Data::as_datagram` for a non-`Datagram` value: {:?}",
                self
            ),
        }
    }

    /// Returns the `Telegram` value.
    ///
    /// # Panics
    ///
    /// The function panics if the `Data` value is no `Telegram` variant.
    pub fn as_telegram(&self) -> &Telegram {
        match *self {
            Data::Telegram(ref telegram) => telegram,
            _ => panic!(
                "called `Data::as_telegram` for a non-`Telegram` value: {:?}",
                self
            ),
        }
    }

    /// Creates an identification string for the variant inside this `Data`.
    pub fn id_string(&self) -> String {
        match *self {
            Data::Packet(ref packet) => packet.id_string(),
            Data::Datagram(ref dgram) => dgram.id_string(),
            Data::Telegram(ref tgram) => tgram.id_string(),
        }
    }
}

impl IdHash for Data {
    fn id_hash<H: Hasher>(&self, h: &mut H) {
        match *self {
            Data::Packet(ref packet) => packet.id_hash(h),
            Data::Datagram(ref dgram) => dgram.id_hash(h),
            Data::Telegram(ref tgram) => tgram.id_hash(h),
        }
    }
}

impl PartialEq for Data {
    /// Returns `true` if two `Data` values are "identical".
    ///
    /// Each `Data` variant has a set of fields that make up its "identity". The `PartialEq` trait
    /// implementation checks those fields for equality and returns `true` if all of the fields
    /// match.
    ///
    /// See the descriptions for the `Header`, `Packet`, `Datagram` and `Telegram` types to find
    /// out which fields are considered in each case.
    fn eq(&self, right: &Data) -> bool {
        let left = self;

        let left_header = left.as_header();
        let right_header = right.as_header();

        if left_header.channel != right_header.channel
            || left_header.destination_address != right_header.destination_address
            || left_header.source_address != right_header.source_address
            || left_header.protocol_version != right_header.protocol_version {
            false
        } else {
            match *left {
                Data::Packet(ref left_packet) => {
                    if let Data::Packet(ref right_packet) = *right {
                        left_packet.command == right_packet.command
                    } else {
                        false
                    }
                }
                Data::Datagram(ref left_dgram) => {
                    if let Data::Datagram(ref right_dgram) = *right {
                        if left_dgram.command != right_dgram.command {
                            false
                        } else if left_dgram.command != 0x0900 {
                            true
                        } else {
                            left_dgram.param16 == right_dgram.param16
                        }
                    } else {
                        false
                    }
                }
                Data::Telegram(ref left_tgram) => {
                    if let Data::Telegram(ref right_tgram) = *right {
                        left_tgram.command == right_tgram.command
                    } else {
                        false
                    }
                }
            }
        }
    }
}

impl PartialOrd for Data {
    /// Compares two `Data` values are "identical".
    ///
    /// Each `Data` variant has a set of fields that make up its "identity". The `PartialOrd` trait
    /// implementation compares those fields.
    ///
    /// See the descriptions for the `Header`, `Packet`, `Datagram` and `Telegram` types to find
    /// out which fields are considered in each case.
    fn partial_cmp(&self, right: &Data) -> Option<Ordering> {
        let left = self;

        let left_header = left.as_header();
        let right_header = right.as_header();

        if left_header.channel < right_header.channel {
            Some(Less)
        } else if left_header.channel > right_header.channel {
            Some(Greater)
        } else if left_header.destination_address < right_header.destination_address {
            Some(Less)
        } else if left_header.destination_address > right_header.destination_address {
            Some(Greater)
        } else if left_header.source_address < right_header.source_address {
            Some(Less)
        } else if left_header.source_address > right_header.source_address {
            Some(Greater)
        } else if left_header.protocol_version < right_header.protocol_version {
            Some(Less)
        } else if left_header.protocol_version > right_header.protocol_version {
            Some(Greater)
        } else {
            match *left {
                Data::Packet(ref left_packet) => {
                    if let Data::Packet(ref right_packet) = *right {
                        Some(left_packet.command.cmp(&right_packet.command))
                    } else {
                        None
                    }
                }
                Data::Datagram(ref left_dgram) => {
                    if let Data::Datagram(ref right_dgram) = *right {
                        if left_dgram.command < right_dgram.command {
                            Some(Less)
                        } else if left_dgram.command > right_dgram.command {
                            Some(Greater)
                        } else if left_dgram.command != 0x0900 {
                            Some(Equal)
                        } else if left_dgram.param16 < right_dgram.param16 {
                            Some(Less)
                        } else if left_dgram.param16 > right_dgram.param16 {
                            Some(Greater)
                        } else {
                            Some(Equal)
                        }
                    } else {
                        None
                    }
                }
                Data::Telegram(ref left_tgram) => {
                    if let Data::Telegram(ref right_tgram) = *right {
                        Some(left_tgram.command.cmp(&right_tgram.command))
                    } else {
                        None
                    }
                }
            }
        }
    }
}

impl From<Packet> for Data {
    fn from(packet: Packet) -> Data {
        Data::Packet(packet)
    }
}

impl From<Datagram> for Data {
    fn from(dgram: Datagram) -> Data {
        Data::Datagram(dgram)
    }
}

impl From<Telegram> for Data {
    fn from(tgram: Telegram) -> Data {
        Data::Telegram(tgram)
    }
}

impl AsRef<Header> for Data {
    fn as_ref(&self) -> &Header {
        match *self {
            Data::Packet(ref packet) => packet.as_ref(),
            Data::Datagram(ref dgram) => dgram.as_ref(),
            Data::Telegram(ref tgram) => tgram.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{DateTime, TimeZone, Utc};

    use crate::{
        id_hash::id_hash,
        live_data_decoder::data_from_checked_bytes,
        test_data::{LIVE_DATA_1, LIVE_TELEGRAM_1},
    };

    fn packet_data(timestamp: DateTime<Utc>, channel: u8) -> Data {
        data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[0..])
    }

    fn datagram_data(timestamp: DateTime<Utc>, channel: u8) -> Data {
        data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[352..])
    }

    fn telegram_data(timestamp: DateTime<Utc>, channel: u8) -> Data {
        data_from_checked_bytes(timestamp, channel, &LIVE_TELEGRAM_1[0..])
    }

    #[test]
    fn test_is_packet() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = packet_data(timestamp, channel);
        assert_eq!(true, packet_data.is_packet());

        let dgram_data = datagram_data(timestamp, channel);
        assert_eq!(false, dgram_data.is_packet());

        let tgram_data = telegram_data(timestamp, channel);
        assert_eq!(false, tgram_data.is_packet());
    }

    #[test]
    fn test_is_datagram() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = packet_data(timestamp, channel);
        assert_eq!(false, packet_data.is_datagram());

        let dgram_data = datagram_data(timestamp, channel);
        assert_eq!(true, dgram_data.is_datagram());

        let tgram_data = telegram_data(timestamp, channel);
        assert_eq!(false, tgram_data.is_datagram());
    }

    #[test]
    fn test_is_telegram() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = packet_data(timestamp, channel);
        assert_eq!(false, packet_data.is_telegram());

        let dgram_data = datagram_data(timestamp, channel);
        assert_eq!(false, dgram_data.is_telegram());

        let tgram_data = telegram_data(timestamp, channel);
        assert_eq!(true, tgram_data.is_telegram());
    }

    #[test]
    fn test_into_packet() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = packet_data(timestamp, channel);
        let packet = packet_data.into_packet();

        assert_eq!(timestamp, packet.header.timestamp);
        assert_eq!(channel, packet.header.channel);
        assert_eq!(0x0010, packet.header.destination_address);
        assert_eq!(0x7E11, packet.header.source_address);
        assert_eq!(0x10, packet.header.protocol_version);
    }

    #[test]
    #[should_panic]
    fn test_into_packet_panic() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let dgram_data = datagram_data(timestamp, channel);
        dgram_data.into_packet();
    }

    #[test]
    fn test_into_datagram() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let dgram_data = datagram_data(timestamp, channel);
        let dgram = dgram_data.into_datagram();

        assert_eq!(timestamp, dgram.header.timestamp);
        assert_eq!(channel, dgram.header.channel);
        assert_eq!(0x0000, dgram.header.destination_address);
        assert_eq!(0x7E11, dgram.header.source_address);
        assert_eq!(0x20, dgram.header.protocol_version);
    }

    #[test]
    #[should_panic]
    fn test_into_datagram_panic() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let tgram_data = telegram_data(timestamp, channel);
        tgram_data.into_datagram();
    }

    #[test]
    fn test_into_telegram() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let tgram_data = telegram_data(timestamp, channel);
        let tgram = tgram_data.into_telegram();

        assert_eq!(timestamp, tgram.header.timestamp);
        assert_eq!(channel, tgram.header.channel);
        assert_eq!(0x7771, tgram.header.destination_address);
        assert_eq!(0x2011, tgram.header.source_address);
        assert_eq!(0x30, tgram.header.protocol_version);
    }

    #[test]
    #[should_panic]
    fn test_into_telegram_panic() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = packet_data(timestamp, channel);
        packet_data.into_telegram();
    }

    #[test]
    fn test_as_header() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = packet_data(timestamp, channel);

        let header = packet_data.as_header();
        assert_eq!(timestamp, header.timestamp);
        assert_eq!(channel, header.channel);
        assert_eq!(0x0010, header.destination_address);
        assert_eq!(0x7E11, header.source_address);
        assert_eq!(0x10, header.protocol_version);

        let dgram_data = datagram_data(timestamp, channel);

        let header = dgram_data.as_header();
        assert_eq!(timestamp, header.timestamp);
        assert_eq!(channel, header.channel);
        assert_eq!(0x0000, header.destination_address);
        assert_eq!(0x7E11, header.source_address);
        assert_eq!(0x20, header.protocol_version);

        let tgram_data = telegram_data(timestamp, channel);

        let header = tgram_data.as_header();
        assert_eq!(timestamp, header.timestamp);
        assert_eq!(channel, header.channel);
        assert_eq!(0x7771, header.destination_address);
        assert_eq!(0x2011, header.source_address);
        assert_eq!(0x30, header.protocol_version);
    }

    #[test]
    fn test_as_packet() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = packet_data(timestamp, channel);
        packet_data.as_packet();
    }

    #[test]
    #[should_panic]
    fn test_as_packet_panic() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let datagram_data = datagram_data(timestamp, channel);
        datagram_data.as_packet();
    }

    #[test]
    fn test_as_datagram() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let datagram_data = datagram_data(timestamp, channel);
        datagram_data.as_datagram();
    }

    #[test]
    #[should_panic]
    fn test_as_datagram_panic() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let telegram_data = telegram_data(timestamp, channel);
        telegram_data.as_datagram();
    }

    #[test]
    fn test_as_telegram() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let telegram_data = telegram_data(timestamp, channel);
        telegram_data.as_telegram();
    }

    #[test]
    #[should_panic]
    fn test_as_telegram_panic() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = packet_data(timestamp, channel);
        packet_data.as_telegram();
    }

    #[test]
    fn test_eq() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = packet_data(timestamp, channel);
        let packet = packet_data.clone().into_packet();

        let dgram_data = datagram_data(timestamp, channel);
        let dgram = dgram_data.clone().into_datagram();

        let tgram_data = telegram_data(timestamp, channel);
        let tgram = tgram_data.clone().into_telegram();

        let other_timestamp = Utc.timestamp(0, 0);

        // Between variants
        assert_eq!(false, packet_data.eq(&dgram_data));
        assert_eq!(false, packet_data.eq(&tgram_data));
        assert_eq!(false, dgram_data.eq(&tgram_data));

        // ---- Packet ----
        let other = packet.clone();
        assert_eq!(true, Data::Packet(other).eq(&packet_data));

        let mut other = packet.clone();
        other.header.timestamp = other_timestamp;
        assert_eq!(true, Data::Packet(other).eq(&packet_data));

        let mut other = packet.clone();
        other.header.channel ^= 1;
        assert_eq!(false, Data::Packet(other).eq(&packet_data));

        let mut other = packet.clone();
        other.header.destination_address ^= 1;
        assert_eq!(false, Data::Packet(other).eq(&packet_data));

        let mut other = packet.clone();
        other.header.source_address ^= 1;
        assert_eq!(false, Data::Packet(other).eq(&packet_data));

        let mut other = packet.clone();
        other.header.protocol_version ^= 1;
        assert_eq!(false, Data::Packet(other).eq(&packet_data));

        let mut other = packet.clone();
        other.command ^= 1;
        assert_eq!(false, Data::Packet(other).eq(&packet_data));

        let mut other = packet.clone();
        other.frame_count ^= 1;
        assert_eq!(true, Data::Packet(other).eq(&packet_data));

        let mut other = packet.clone();
        other.frame_data[0] ^= 1;
        assert_eq!(true, Data::Packet(other).eq(&packet_data));

        // ---- Datagram ----
        let other = dgram.clone();
        assert_eq!(true, Data::Datagram(other).eq(&dgram_data));

        let mut other = dgram.clone();
        other.header.timestamp = other_timestamp;
        assert_eq!(true, Data::Datagram(other).eq(&dgram_data));

        let mut other = dgram.clone();
        other.header.channel ^= 1;
        assert_eq!(false, Data::Datagram(other).eq(&dgram_data));

        let mut other = dgram.clone();
        other.header.destination_address ^= 1;
        assert_eq!(false, Data::Datagram(other).eq(&dgram_data));

        let mut other = dgram.clone();
        other.header.source_address ^= 1;
        assert_eq!(false, Data::Datagram(other).eq(&dgram_data));

        let mut other = dgram.clone();
        other.header.protocol_version ^= 1;
        assert_eq!(false, Data::Datagram(other).eq(&dgram_data));

        let mut other = dgram.clone();
        other.command ^= 1;
        assert_eq!(false, Data::Datagram(other).eq(&dgram_data));

        let mut other = dgram.clone();
        other.param16 ^= 1;
        assert_eq!(true, Data::Datagram(other).eq(&dgram_data));

        let mut other = dgram.clone();
        other.param32 ^= 1;
        assert_eq!(true, Data::Datagram(other).eq(&dgram_data));

        // ---- Telegram ----
        let other = tgram.clone();
        assert_eq!(true, Data::Telegram(other).eq(&tgram_data));

        let mut other = tgram.clone();
        other.header.timestamp = other_timestamp;
        assert_eq!(true, Data::Telegram(other).eq(&tgram_data));

        let mut other = tgram.clone();
        other.header.channel ^= 1;
        assert_eq!(false, Data::Telegram(other).eq(&tgram_data));

        let mut other = tgram.clone();
        other.header.destination_address ^= 1;
        assert_eq!(false, Data::Telegram(other).eq(&tgram_data));

        let mut other = tgram.clone();
        other.header.source_address ^= 1;
        assert_eq!(false, Data::Telegram(other).eq(&tgram_data));

        let mut other = tgram.clone();
        other.header.protocol_version ^= 1;
        assert_eq!(false, Data::Telegram(other).eq(&tgram_data));

        let mut other = tgram.clone();
        other.command ^= 1;
        assert_eq!(false, Data::Telegram(other).eq(&tgram_data));

        let mut other = tgram.clone();
        other.frame_data[0] ^= 1;
        assert_eq!(true, Data::Telegram(other).eq(&tgram_data));
    }

    #[test]
    fn test_partial_cmp() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = packet_data(timestamp, channel);
        let packet = match packet_data {
            Data::Packet(ref packet) => packet,
            _ => unreachable!(),
        };

        let dgram_data = datagram_data(timestamp, channel);
        let dgram = match dgram_data {
            Data::Datagram(ref dgram) => dgram,
            _ => unreachable!(),
        };

        let tgram_data = telegram_data(timestamp, channel);
        let tgram = match tgram_data {
            Data::Telegram(ref tgram) => tgram,
            _ => unreachable!(),
        };

        let other_timestamp = Utc.timestamp(0, 0);

        // Between variants
        assert_eq!(Some(Greater), packet_data.partial_cmp(&dgram_data));
        assert_eq!(Some(Less), packet_data.partial_cmp(&tgram_data));
        assert_eq!(Some(Less), dgram_data.partial_cmp(&tgram_data));

        // ---- Packet ----
        let other = packet.clone();
        assert_eq!(Some(Equal), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.header.timestamp = other_timestamp;
        assert_eq!(Some(Equal), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.header.channel -= 1;
        assert_eq!(Some(Less), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.header.channel += 1;
        assert_eq!(Some(Greater), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.header.destination_address -= 1;
        assert_eq!(Some(Less), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.header.destination_address += 1;
        assert_eq!(Some(Greater), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.header.source_address -= 1;
        assert_eq!(Some(Less), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.header.source_address += 1;
        assert_eq!(Some(Greater), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.header.protocol_version -= 1;
        assert_eq!(Some(Less), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.header.protocol_version += 1;
        assert_eq!(Some(Greater), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.command -= 1;
        assert_eq!(Some(Less), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.command += 1;
        assert_eq!(Some(Greater), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.frame_count ^= 1;
        assert_eq!(Some(Equal), Data::Packet(other).partial_cmp(&packet_data));

        let mut other = packet.clone();
        other.frame_data[0] ^= 1;
        assert_eq!(Some(Equal), Data::Packet(other).partial_cmp(&packet_data));

        // ---- Datagram ----
        let other = dgram.clone();
        assert_eq!(Some(Equal), Data::Datagram(other).partial_cmp(&dgram_data));

        let mut other = dgram.clone();
        other.header.timestamp = other_timestamp;
        assert_eq!(Some(Equal), Data::Datagram(other).partial_cmp(&dgram_data));

        let mut other = dgram.clone();
        other.header.channel -= 1;
        assert_eq!(Some(Less), Data::Datagram(other).partial_cmp(&dgram_data));

        let mut other = dgram.clone();
        other.header.channel += 1;
        assert_eq!(
            Some(Greater),
            Data::Datagram(other).partial_cmp(&dgram_data)
        );

        let mut other = dgram.clone();
        other.header.destination_address += 1; // NOTE(daniel): subtraction would underflow
        assert_eq!(Some(Less), dgram_data.partial_cmp(&Data::Datagram(other)));

        let mut other = dgram.clone();
        other.header.destination_address += 1;
        assert_eq!(
            Some(Greater),
            Data::Datagram(other).partial_cmp(&dgram_data)
        );

        let mut other = dgram.clone();
        other.header.source_address -= 1;
        assert_eq!(Some(Less), Data::Datagram(other).partial_cmp(&dgram_data));

        let mut other = dgram.clone();
        other.header.source_address += 1;
        assert_eq!(
            Some(Greater),
            Data::Datagram(other).partial_cmp(&dgram_data)
        );

        let mut other = dgram.clone();
        other.header.protocol_version -= 1;
        assert_eq!(Some(Less), Data::Datagram(other).partial_cmp(&dgram_data));

        let mut other = dgram.clone();
        other.header.protocol_version += 1;
        assert_eq!(
            Some(Greater),
            Data::Datagram(other).partial_cmp(&dgram_data)
        );

        let mut other = dgram.clone();
        other.command -= 1;
        assert_eq!(Some(Less), Data::Datagram(other).partial_cmp(&dgram_data));

        let mut other = dgram.clone();
        other.command += 1;
        assert_eq!(
            Some(Greater),
            Data::Datagram(other).partial_cmp(&dgram_data)
        );

        let mut other = dgram.clone();
        other.param16 ^= 1;
        assert_eq!(Some(Equal), Data::Datagram(other).partial_cmp(&dgram_data));

        let mut other = dgram.clone();
        other.param32 ^= 1;
        assert_eq!(Some(Equal), Data::Datagram(other).partial_cmp(&dgram_data));

        // ---- Telegram ----
        let other = tgram.clone();
        assert_eq!(Some(Equal), Data::Telegram(other).partial_cmp(&tgram_data));

        let mut other = tgram.clone();
        other.header.timestamp = other_timestamp;
        assert_eq!(Some(Equal), Data::Telegram(other).partial_cmp(&tgram_data));

        let mut other = tgram.clone();
        other.header.channel -= 1;
        assert_eq!(Some(Less), Data::Telegram(other).partial_cmp(&tgram_data));

        let mut other = tgram.clone();
        other.header.channel += 1;
        assert_eq!(
            Some(Greater),
            Data::Telegram(other).partial_cmp(&tgram_data)
        );

        let mut other = tgram.clone();
        other.header.destination_address -= 1;
        assert_eq!(Some(Less), Data::Telegram(other).partial_cmp(&tgram_data));

        let mut other = tgram.clone();
        other.header.destination_address += 1;
        assert_eq!(
            Some(Greater),
            Data::Telegram(other).partial_cmp(&tgram_data)
        );

        let mut other = tgram.clone();
        other.header.source_address -= 1;
        assert_eq!(Some(Less), Data::Telegram(other).partial_cmp(&tgram_data));

        let mut other = tgram.clone();
        other.header.source_address += 1;
        assert_eq!(
            Some(Greater),
            Data::Telegram(other).partial_cmp(&tgram_data)
        );

        let mut other = tgram.clone();
        other.header.protocol_version -= 1;
        assert_eq!(Some(Less), Data::Telegram(other).partial_cmp(&tgram_data));

        let mut other = tgram.clone();
        other.header.protocol_version += 1;
        assert_eq!(
            Some(Greater),
            Data::Telegram(other).partial_cmp(&tgram_data)
        );

        let mut other = tgram.clone();
        other.command -= 1;
        assert_eq!(Some(Less), Data::Telegram(other).partial_cmp(&tgram_data));

        let mut other = tgram.clone();
        other.command += 1;
        assert_eq!(
            Some(Greater),
            Data::Telegram(other).partial_cmp(&tgram_data)
        );

        let mut other = tgram.clone();
        other.frame_data[0] ^= 1;
        assert_eq!(Some(Equal), Data::Telegram(other).partial_cmp(&tgram_data));
    }

    #[test]
    fn test_id_hash() {
        let timestamp = Utc.timestamp(1485688933, 0);
        let channel = 0x11;

        let data = packet_data(timestamp, channel);

        let result = id_hash(&data);
        assert_eq!(541127499104566154, result);

        let data = datagram_data(timestamp, channel);

        let result = id_hash(&data);
        assert_eq!(6066488998843577430, result);

        let data = telegram_data(timestamp, channel);

        let result = id_hash(&data);
        assert_eq!(2688669052981416192, result);
    }
}
