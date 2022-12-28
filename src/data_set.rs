use std::{
    cmp::{Ord, Ordering},
    hash::Hasher,
    slice::{Iter, IterMut},
};

use chrono::{DateTime, Utc};

use crate::{data::Data, id_hash::IdHash, packet::PacketId, utils::current_timestamp};

/// A `DataSet` contains a set of unique (non-identical) `Data` values.
///
/// # Examples
///
/// ```rust
/// use std::io::Read;
///
/// use resol_vbus::{DataSet, RecordingReader, Result};
///
/// # #[allow(dead_code)]
/// fn print_data_ids<R: Read>(r: R) -> Result<()> {
///     let mut rr = RecordingReader::new(r);
///
///     let mut cumultative_data_set = DataSet::new();
///
///     while let Some(data_set) = rr.read_data_set()? {
///         let timestamp = data_set.timestamp;
///
///         cumultative_data_set.add_data_set(data_set);
///
///         println!("{}:", timestamp);
///         for data in cumultative_data_set.iter() {
///             println!("    - {}", data.id_string());
///         }
///     }
///
///     Ok(())
/// }
/// ```
#[derive(Clone, Debug)]
pub struct DataSet {
    /// The timestamp that corresponds to the contained set of `Data` values.
    pub timestamp: DateTime<Utc>,
    set: Vec<Data>,
}

impl DataSet {
    /// Construct an empty `DataSet`.
    pub fn new() -> DataSet {
        DataSet {
            timestamp: current_timestamp(),
            set: Vec::new(),
        }
    }

    /// Construct a `DataSet` from a list of `Data` values.
    pub fn from_data(timestamp: DateTime<Utc>, set: Vec<Data>) -> DataSet {
        DataSet { timestamp, set }
    }

    /// Return the amount of `Data` values contained in this `DataSet`.
    pub fn len(&self) -> usize {
        self.set.len()
    }

    /// Return whether this `DataSet` is empty.
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    /// Return the `Data` values contained in this `DataSet`.
    pub fn as_data_slice(&self) -> &[Data] {
        &self.set[..]
    }

    /// Add a `Data` value, replacing any identical existing one.
    pub fn add_data(&mut self, data: Data) {
        let timestamp = data.as_header().timestamp;

        let position = self.set.iter().position(|d| d.eq(&data));

        match position {
            Some(index) => self.set[index] = data,
            None => self.set.push(data),
        };

        if self.timestamp < timestamp {
            self.timestamp = timestamp;
        }
    }

    /// Add all `Data` values from one `DataSet` into another.
    pub fn add_data_set(&mut self, data_set: DataSet) {
        let timestamp = data_set.timestamp;

        for data in data_set.set.into_iter() {
            self.add_data(data);
        }

        if self.timestamp < timestamp {
            self.timestamp = timestamp;
        }
    }

    /// Remove all `Data` values.
    pub fn remove_all_data(&mut self) {
        self.set.clear();
    }

    /// Remove `Data` values with timestamps older than `min_timestamp`.
    pub fn remove_data_older_than(&mut self, min_timestamp: DateTime<Utc>) {
        self.set
            .retain(|data| data.as_header().timestamp >= min_timestamp);
    }

    /// Find all `Packet` values and set their `frame_count` to zero effectively hiding
    /// their `frame_data` payload.
    pub fn clear_all_packets(&mut self) {
        for data in self.set.iter_mut() {
            if let Data::Packet(ref mut packet) = *data {
                if packet.frame_count > 0 {
                    packet.frame_count = 0;
                }
            }
        }
    }

    /// Find all `Packet` values with timestamps older than `min_timestamp` and set their
    /// `frame_count` to zero effectively hiding their `frame_data` payload.
    pub fn clear_packets_older_than(&mut self, min_timestamp: DateTime<Utc>) {
        for data in self.set.iter_mut() {
            if let Data::Packet(ref mut packet) = *data {
                if packet.header.timestamp < min_timestamp && packet.frame_count > 0 {
                    packet.frame_count = 0;
                }
            }
        }
    }

    /// Returns an iterator over the `Data` values.
    pub fn iter(&self) -> Iter<'_, Data> {
        self.set.iter()
    }

    /// Returns an iterator over the `Data` values.
    pub fn iter_mut(&mut self) -> IterMut<'_, Data> {
        self.set.iter_mut()
    }

    /// Sort the `Data` values contained in this `DataSet`.
    pub fn sort(&mut self) {
        self.set.sort_by(|l, r| l.partial_cmp(r).unwrap());
    }

    /// Sort the `Data` values contained in this `DataSet`.
    pub fn sort_by<F>(&mut self, f: F)
    where
        F: FnMut(&Data, &Data) -> Ordering,
    {
        self.set.sort_by(f);
    }

    /// Sort the `Data` values contained in this `DataSet` by a list of known `PacketId` values.
    pub fn sort_by_id_slice(&mut self, ids: &[PacketId]) {
        self.sort_by(|l, r| {
            if l.is_packet() && r.is_packet() {
                let l_id = l.as_packet().packet_id();
                let r_id = r.as_packet().packet_id();

                let l_pos = ids.iter().position(|id| *id == l_id);
                let r_pos = ids.iter().position(|id| *id == r_id);

                if l_pos.is_some() && r_pos.is_some() {
                    l_pos.cmp(&r_pos)
                } else if l_pos.is_some() {
                    Ordering::Less
                } else if r_pos.is_some() {
                    Ordering::Greater
                } else {
                    l_id.cmp(&r_id)
                }
            } else if l.is_packet() {
                Ordering::Less
            } else if r.is_packet() {
                Ordering::Greater
            } else {
                l.partial_cmp(r).expect("Comparison should always succeed")
            }
        })
    }
}

impl IdHash for DataSet {
    fn id_hash<H: Hasher>(&self, h: &mut H) {
        for data in self.set.iter() {
            data.id_hash(h);
        }
    }
}

impl Default for DataSet {
    fn default() -> DataSet {
        DataSet::new()
    }
}

impl AsRef<[Data]> for DataSet {
    fn as_ref(&self) -> &[Data] {
        &self.set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Duration;

    use crate::{
        id_hash::id_hash,
        live_data_decoder::data_from_checked_bytes,
        test_data::{LIVE_DATA_1, LIVE_TELEGRAM_1},
        test_utils::{test_clone_derive, test_debug_derive},
        utils::utc_timestamp,
    };

    #[test]
    fn test_len() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let packet_data = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[0..]);
        let dgram_data = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[352..]);

        let mut data_set = DataSet::with_timestamp(utc_timestamp(0));

        assert_eq!(0, data_set.len());

        data_set.add_data(packet_data);

        assert_eq!(1, data_set.len());

        data_set.add_data(dgram_data.clone());

        assert_eq!(2, data_set.len());

        data_set.add_data(dgram_data);

        assert_eq!(2, data_set.len());
    }

    #[test]
    fn test_is_empty() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let packet_data = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[0..]);

        let mut data_set = DataSet::with_timestamp(utc_timestamp(0));

        assert!(data_set.is_empty());

        data_set.add_data(packet_data);

        assert!(!data_set.is_empty());
    }

    #[test]
    fn test_add_data() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let packet_data = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[0..]);
        let dgram_data = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[352..]);
        let tgram_data = data_from_checked_bytes(timestamp, channel, &LIVE_TELEGRAM_1[0..]);

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        assert_eq!(0, data_set.as_data_slice().len());

        data_set.add_data(packet_data.clone());
        assert_eq!(timestamp, data_set.timestamp);
        assert_eq!(1, data_set.as_data_slice().len());
        assert_eq!(
            "11_0010_7E11_10_0100",
            data_set.as_data_slice()[0].id_string()
        );

        let other_timestamp = timestamp + Duration::seconds(1);

        let data = data_from_checked_bytes(other_timestamp, channel, &LIVE_DATA_1[0..]);
        data_set.add_data(data);
        assert_eq!(other_timestamp, data_set.timestamp);
        assert_eq!(1, data_set.as_data_slice().len());
        assert_eq!(
            "11_0010_7E11_10_0100",
            data_set.as_data_slice()[0].id_string()
        );

        let other_channel = channel + 1;

        let data = data_from_checked_bytes(timestamp, other_channel, &LIVE_DATA_1[0..]);
        data_set.add_data(data);
        assert_eq!(other_timestamp, data_set.timestamp);
        assert_eq!(2, data_set.as_data_slice().len());
        assert_eq!(
            "11_0010_7E11_10_0100",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "12_0010_7E11_10_0100",
            data_set.as_data_slice()[1].id_string()
        );

        data_set.add_data(dgram_data.clone());
        assert_eq!(other_timestamp, data_set.timestamp);
        assert_eq!(3, data_set.as_data_slice().len());
        assert_eq!(
            "11_0010_7E11_10_0100",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "12_0010_7E11_10_0100",
            data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "11_0000_7E11_20_0500_0000",
            data_set.as_data_slice()[2].id_string()
        );

        data_set.add_data(tgram_data.clone());
        assert_eq!(other_timestamp, data_set.timestamp);
        assert_eq!(4, data_set.as_data_slice().len());
        assert_eq!(
            "11_0010_7E11_10_0100",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "12_0010_7E11_10_0100",
            data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "11_0000_7E11_20_0500_0000",
            data_set.as_data_slice()[2].id_string()
        );
        assert_eq!(
            "11_7771_2011_30_25",
            data_set.as_data_slice()[3].id_string()
        );
    }

    #[test]
    fn test_add_data_set() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[352..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_TELEGRAM_1[0..],
        ));

        let mut other_data_set = DataSet::new();
        other_data_set.timestamp = utc_timestamp(0);
        other_data_set.add_data_set(data_set);

        assert_eq!(timestamp, other_data_set.timestamp);
        assert_eq!(3, other_data_set.as_data_slice().len());
        assert_eq!(
            "11_0010_7E11_10_0100",
            other_data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "11_0000_7E11_20_0500_0000",
            other_data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "11_7771_2011_30_25",
            other_data_set.as_data_slice()[2].id_string()
        );

        let timestamp = current_timestamp();

        let mut other_data_set = DataSet::with_timestamp(utc_timestamp(0));
        other_data_set.add_data_set(DataSet::with_timestamp(timestamp));

        assert_eq!(timestamp, other_data_set.timestamp);
    }

    #[test]
    fn test_remove_all_data() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[352..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_TELEGRAM_1[0..],
        ));

        data_set.remove_all_data();

        assert_eq!(0, data_set.len());
    }

    #[test]
    fn test_remove_data_older_than() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        data_set.add_data(data_from_checked_bytes(
            timestamp + Duration::seconds(10),
            channel,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp + Duration::seconds(20),
            channel,
            &LIVE_DATA_1[352..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp + Duration::seconds(30),
            channel,
            &LIVE_TELEGRAM_1[0..],
        ));
        data_set.remove_data_older_than(timestamp + Duration::seconds(20));

        assert_eq!(timestamp + Duration::seconds(30), data_set.timestamp);
        assert_eq!(2, data_set.as_data_slice().len());
        assert_eq!(
            "11_0000_7E11_20_0500_0000",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "11_7771_2011_30_25",
            data_set.as_data_slice()[1].id_string()
        );
    }

    #[test]
    fn test_clear_all_packets() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[352..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_TELEGRAM_1[0..],
        ));

        data_set.clear_all_packets();

        let data_slice = data_set.as_data_slice();
        assert_eq!(3, data_slice.len());
        assert_eq!("11_0010_7E11_10_0100", data_slice[0].id_string());
        if let Data::Packet(ref packet) = data_slice[0] {
            assert_eq!(0, packet.frame_count);
        } else {
            panic!("First element should have been a packet");
        }
        assert_eq!("11_0000_7E11_20_0500_0000", data_slice[1].id_string());
        assert_eq!("11_7771_2011_30_25", data_slice[2].id_string());
    }

    #[test]
    fn test_clear_packets_older_than() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        data_set.add_data(data_from_checked_bytes(
            timestamp + Duration::seconds(10),
            channel,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp + Duration::seconds(20),
            channel,
            &LIVE_DATA_1[352..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp + Duration::seconds(30),
            channel,
            &LIVE_TELEGRAM_1[0..],
        ));
        data_set.clear_packets_older_than(timestamp + Duration::seconds(20));

        assert_eq!(timestamp + Duration::seconds(30), data_set.timestamp);

        let data_slice = data_set.as_data_slice();
        assert_eq!(3, data_slice.len());
        assert_eq!("11_0010_7E11_10_0100", data_slice[0].id_string());
        if let Data::Packet(ref packet) = data_slice[0] {
            assert_eq!(0, packet.frame_count);
        } else {
            panic!("First element should have been a packet");
        }
        assert_eq!("11_0000_7E11_20_0500_0000", data_slice[1].id_string());
        assert_eq!("11_7771_2011_30_25", data_slice[2].id_string());
    }

    #[test]
    fn test_iter() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[352..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_TELEGRAM_1[0..],
        ));

        let mut iter = data_set.iter();

        let item = iter.next().expect("Should have been Data");
        assert_eq!("11_0010_7E11_10_0100", item.id_string());

        let item = iter.next().expect("Should have been Data");
        assert_eq!("11_0000_7E11_20_0500_0000", item.id_string());

        let item = iter.next().expect("Should have been Data");
        assert_eq!("11_7771_2011_30_25", item.id_string());

        let item = iter.next();
        assert_eq!(None, item);
    }

    #[test]
    fn test_iter_mut() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[352..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_TELEGRAM_1[0..],
        ));

        let mut iter = data_set.iter_mut();

        let item = iter.next().expect("Should have been Data");
        assert_eq!("11_0010_7E11_10_0100", item.id_string());

        let item = iter.next().expect("Should have been Data");
        assert_eq!("11_0000_7E11_20_0500_0000", item.id_string());

        let item = iter.next().expect("Should have been Data");
        assert_eq!("11_7771_2011_30_25", item.id_string());

        let item = iter.next();
        assert_eq!(None, item);
    }

    #[test]
    fn test_sort() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel + 1,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[352..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_TELEGRAM_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[258..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[242..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[172..],
        ));

        assert_eq!(7, data_set.as_data_slice().len());
        assert_eq!(
            "12_0010_7E11_10_0100",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "11_0000_7E11_20_0500_0000",
            data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "11_0010_7E11_10_0100",
            data_set.as_data_slice()[2].id_string()
        );
        assert_eq!(
            "11_7771_2011_30_25",
            data_set.as_data_slice()[3].id_string()
        );
        assert_eq!(
            "11_6651_7E11_10_0200",
            data_set.as_data_slice()[4].id_string()
        );
        assert_eq!(
            "11_0010_7E22_10_0100",
            data_set.as_data_slice()[5].id_string()
        );
        assert_eq!(
            "11_0015_7E11_10_0100",
            data_set.as_data_slice()[6].id_string()
        );

        data_set.sort();

        assert_eq!(7, data_set.as_data_slice().len());
        assert_eq!(
            "11_0000_7E11_20_0500_0000",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "11_0010_7E11_10_0100",
            data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "11_0010_7E22_10_0100",
            data_set.as_data_slice()[2].id_string()
        );
        assert_eq!(
            "11_0015_7E11_10_0100",
            data_set.as_data_slice()[3].id_string()
        );
        assert_eq!(
            "11_6651_7E11_10_0200",
            data_set.as_data_slice()[4].id_string()
        );
        assert_eq!(
            "11_7771_2011_30_25",
            data_set.as_data_slice()[5].id_string()
        );
        assert_eq!(
            "12_0010_7E11_10_0100",
            data_set.as_data_slice()[6].id_string()
        );
    }

    #[test]
    fn test_sort_by() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel + 1,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[352..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_TELEGRAM_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[258..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[242..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[172..],
        ));

        assert_eq!(7, data_set.as_data_slice().len());
        assert_eq!(
            "12_0010_7E11_10_0100",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "11_0000_7E11_20_0500_0000",
            data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "11_0010_7E11_10_0100",
            data_set.as_data_slice()[2].id_string()
        );
        assert_eq!(
            "11_7771_2011_30_25",
            data_set.as_data_slice()[3].id_string()
        );
        assert_eq!(
            "11_6651_7E11_10_0200",
            data_set.as_data_slice()[4].id_string()
        );
        assert_eq!(
            "11_0010_7E22_10_0100",
            data_set.as_data_slice()[5].id_string()
        );
        assert_eq!(
            "11_0015_7E11_10_0100",
            data_set.as_data_slice()[6].id_string()
        );

        data_set.sort_by(|l, r| {
            let l_id = &l.id_string()[8..];
            let r_id = &r.id_string()[8..];
            l_id.cmp(r_id)
        });

        assert_eq!(7, data_set.as_data_slice().len());
        assert_eq!(
            "11_7771_2011_30_25",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "12_0010_7E11_10_0100",
            data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "11_0010_7E11_10_0100",
            data_set.as_data_slice()[2].id_string()
        );
        assert_eq!(
            "11_0015_7E11_10_0100",
            data_set.as_data_slice()[3].id_string()
        );
        assert_eq!(
            "11_6651_7E11_10_0200",
            data_set.as_data_slice()[4].id_string()
        );
        assert_eq!(
            "11_0000_7E11_20_0500_0000",
            data_set.as_data_slice()[5].id_string()
        );
        assert_eq!(
            "11_0010_7E22_10_0100",
            data_set.as_data_slice()[6].id_string()
        );
    }

    #[test]
    fn test_sort_by_id_slice() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[172..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[242..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[258..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[352..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_TELEGRAM_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel + 1,
            &LIVE_DATA_1[0..],
        ));

        data_set.sort_by_id_slice(&[
            PacketId(0x12, 0x0010, 0x7E11, 0x0100),
            PacketId(0x11, 0x0015, 0x7E11, 0x0100),
        ]);

        let data_slice = data_set.as_data_slice();

        assert_eq!(7, data_slice.len());
        assert_eq!("12_0010_7E11_10_0100", data_slice[0].id_string());
        assert_eq!("11_0015_7E11_10_0100", data_slice[1].id_string());
        assert_eq!("11_0010_7E11_10_0100", data_slice[2].id_string());
        assert_eq!("11_0010_7E22_10_0100", data_slice[3].id_string());
        assert_eq!("11_6651_7E11_10_0200", data_slice[4].id_string());
        assert_eq!("11_0000_7E11_20_0500_0000", data_slice[5].id_string());
        assert_eq!("11_7771_2011_30_25", data_slice[6].id_string());
    }

    #[test]
    fn test_id_hash() {
        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = utc_timestamp(0);
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[0..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_DATA_1[352..],
        ));
        data_set.add_data(data_from_checked_bytes(
            timestamp,
            channel,
            &LIVE_TELEGRAM_1[0..],
        ));

        let result = id_hash(&data_set);

        assert_eq!(13725728793204414233, result);
    }

    #[test]
    fn test_derived_trait_impls() {
        let data_set = DataSet::new();

        test_debug_derive(&data_set);
        test_clone_derive(&data_set);
    }

    #[test]
    fn test_default_trait_impl() {
        let timestamp_before = current_timestamp();

        let data_set = DataSet::default();

        let timestamp_after = current_timestamp();

        assert!(data_set.timestamp >= timestamp_before);
        assert!(data_set.timestamp <= timestamp_after);
        assert!(data_set.is_empty());
    }
}
