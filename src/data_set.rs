use std::cmp::Ordering;
use std::slice::{Iter, IterMut};

use chrono::{DateTime, UTC};

use data::Data;


/// A `DataSet` contains a set of unique `Data`.
#[derive(Clone, Debug)]
pub struct DataSet {
    /// The timestamp that corresponds to the contained set of `Data` objects.
    pub timestamp: DateTime<UTC>,
    set: Vec<Data>,
}


impl DataSet {

    /// Construct an empty `DataSet`.
    pub fn new() -> DataSet {
        DataSet {
            timestamp: UTC::now(),
            set: Vec::new(),
        }
    }

    /// Construct a `DataSet` from a list of `Data` objects.
    pub fn from_data(timestamp: DateTime<UTC>, set: Vec<Data>) -> DataSet {
        DataSet {
            timestamp: timestamp,
            set: set,
        }
    }

    /// Return the `Data` objects contained in this `DataSet`.
    pub fn as_data_slice(&self) -> &[Data] {
        &self.set [..]
    }

    /// Add a `Data` object, replacing any equivalent existing one.
    pub fn add_data(&mut self, data: Data) {
        let timestamp = data.as_header().timestamp;

        let position = self.set.iter().position(|d| {
            d.eq(&data)
        });

        match position {
            Some(index) => self.set [index] = data,
            None => self.set.push(data),
        };

        if self.timestamp < timestamp {
            self.timestamp = timestamp;
        }
    }

    /// Add all `Data` objects from one `DataSet` into another.
    pub fn add_data_set(&mut self, data_set: DataSet) {
        let timestamp = data_set.timestamp;

        for data in data_set.set.into_iter() {
            self.add_data(data);
        }

        if self.timestamp < timestamp {
            self.timestamp = timestamp;
        }
    }

    /// Remove `Data` with timestamps older than `min_timestamp`.
    pub fn remove_data_older_than(&mut self, min_timestamp: DateTime<UTC>) {
        self.set.retain(|data| data.as_header().timestamp >= min_timestamp);
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
    pub fn clear_packets_older_than(&mut self, min_timestamp: DateTime<UTC>) {
        for data in self.set.iter_mut() {
            if let Data::Packet(ref mut packet) = *data {
                if packet.header.timestamp < min_timestamp && packet.frame_count > 0 {
                    packet.frame_count = 0;
                }
            }
        }
    }

    /// Returns an iterator over the `Data` values.
    pub fn iter(&self) -> Iter<Data> {
        self.set.iter()
    }

    /// Returns an iterator over the `Data` values.
    pub fn iter_mut(&mut self) -> IterMut<Data> {
        self.set.iter_mut()
    }

    /// Sort the `Data` objects contained in this `DataSet`.
    pub fn sort(&mut self) {
        self.set.sort_by(|l, r| { l.partial_cmp(r).unwrap() });
    }

    /// Sort the `Data` objects contained in this `DataSet`.
    pub fn sort_by<F>(&mut self, f: F) where F: FnMut(&Data, &Data) -> Ordering {
        self.set.sort_by(f);
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
    use chrono::{Duration, TimeZone, UTC};

    use live_data_decoder::data_from_checked_bytes;

    use super::*;

    use test_data::{LIVE_DATA_1, LIVE_TELEGRAM_1};

    #[test]
    fn test_add_data() {
        let timestamp = UTC.timestamp(1485688933, 0);
        let channel = 0x11;

        let packet_data = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [0..]);
        let dgram_data = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [352..]);
        let tgram_data = data_from_checked_bytes(timestamp, channel, &LIVE_TELEGRAM_1 [0..]);

        let mut data_set = DataSet::new();
        data_set.timestamp = UTC.timestamp(0, 0);
        assert_eq!(0, data_set.as_data_slice().len());

        data_set.add_data(packet_data.clone());
        assert_eq!(timestamp, data_set.timestamp);
        assert_eq!(1, data_set.as_data_slice().len());
        assert_eq!("11_0010_7E11_10_0100", data_set.as_data_slice() [0].id_string());

        let other_timestamp = timestamp + Duration::seconds(1);

        let data = data_from_checked_bytes(other_timestamp, channel, &LIVE_DATA_1 [0..]);
        data_set.add_data(data);
        assert_eq!(other_timestamp, data_set.timestamp);
        assert_eq!(1, data_set.as_data_slice().len());
        assert_eq!("11_0010_7E11_10_0100", data_set.as_data_slice() [0].id_string());

        let other_channel = channel + 1;

        let data = data_from_checked_bytes(timestamp, other_channel, &LIVE_DATA_1 [0..]);
        data_set.add_data(data);
        assert_eq!(other_timestamp, data_set.timestamp);
        assert_eq!(2, data_set.as_data_slice().len());
        assert_eq!("11_0010_7E11_10_0100", data_set.as_data_slice() [0].id_string());
        assert_eq!("12_0010_7E11_10_0100", data_set.as_data_slice() [1].id_string());

        data_set.add_data(dgram_data.clone());
        assert_eq!(other_timestamp, data_set.timestamp);
        assert_eq!(3, data_set.as_data_slice().len());
        assert_eq!("11_0010_7E11_10_0100", data_set.as_data_slice() [0].id_string());
        assert_eq!("12_0010_7E11_10_0100", data_set.as_data_slice() [1].id_string());
        assert_eq!("11_0000_7E11_20_0500_0000", data_set.as_data_slice() [2].id_string());

        data_set.add_data(tgram_data.clone());
        assert_eq!(other_timestamp, data_set.timestamp);
        assert_eq!(4, data_set.as_data_slice().len());
        assert_eq!("11_0010_7E11_10_0100", data_set.as_data_slice() [0].id_string());
        assert_eq!("12_0010_7E11_10_0100", data_set.as_data_slice() [1].id_string());
        assert_eq!("11_0000_7E11_20_0500_0000", data_set.as_data_slice() [2].id_string());
        assert_eq!("11_7771_2011_30_25", data_set.as_data_slice() [3].id_string());
    }

    #[test]
    fn test_add_data_set() {
        let timestamp = UTC.timestamp(1485688933, 0);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = UTC.timestamp(0, 0);
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [0..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [352..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_TELEGRAM_1 [0..]));

        let mut other_data_set = DataSet::new();
        other_data_set.timestamp = UTC.timestamp(0, 0);
        other_data_set.add_data_set(data_set);

        assert_eq!(timestamp, other_data_set.timestamp);
        assert_eq!(3, other_data_set.as_data_slice().len());
        assert_eq!("11_0010_7E11_10_0100", other_data_set.as_data_slice() [0].id_string());
        assert_eq!("11_0000_7E11_20_0500_0000", other_data_set.as_data_slice() [1].id_string());
        assert_eq!("11_7771_2011_30_25", other_data_set.as_data_slice() [2].id_string());
    }

    #[test]
    fn test_remove_data_older_than() {
        let timestamp = UTC.timestamp(1485688933, 0);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = UTC.timestamp(0, 0);
        data_set.add_data(data_from_checked_bytes(timestamp + Duration::seconds(10), channel, &LIVE_DATA_1 [0..]));
        data_set.add_data(data_from_checked_bytes(timestamp + Duration::seconds(20), channel, &LIVE_DATA_1 [352..]));
        data_set.add_data(data_from_checked_bytes(timestamp + Duration::seconds(30), channel, &LIVE_TELEGRAM_1 [0..]));
        data_set.remove_data_older_than(timestamp + Duration::seconds(20));

        assert_eq!(timestamp + Duration::seconds(30), data_set.timestamp);
        assert_eq!(2, data_set.as_data_slice().len());
        assert_eq!("11_0000_7E11_20_0500_0000", data_set.as_data_slice() [0].id_string());
        assert_eq!("11_7771_2011_30_25", data_set.as_data_slice() [1].id_string());
    }

    #[test]
    fn test_clear_packets_older_than() {
        let timestamp = UTC.timestamp(1485688933, 0);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = UTC.timestamp(0, 0);
        data_set.add_data(data_from_checked_bytes(timestamp + Duration::seconds(10), channel, &LIVE_DATA_1 [0..]));
        data_set.add_data(data_from_checked_bytes(timestamp + Duration::seconds(20), channel, &LIVE_DATA_1 [352..]));
        data_set.add_data(data_from_checked_bytes(timestamp + Duration::seconds(30), channel, &LIVE_TELEGRAM_1 [0..]));
        data_set.clear_packets_older_than(timestamp + Duration::seconds(20));

        assert_eq!(timestamp + Duration::seconds(30), data_set.timestamp);

        let data_slice = data_set.as_data_slice();
        assert_eq!(3, data_slice.len());
        assert_eq!("11_0010_7E11_10_0100", data_slice [0].id_string());
        if let Data::Packet(ref packet) = data_slice [0] {
            assert_eq!(0, packet.frame_count);
        } else {
            panic!("First element should have been a packet");
        }
        assert_eq!("11_0000_7E11_20_0500_0000", data_slice [1].id_string());
        assert_eq!("11_7771_2011_30_25", data_slice [2].id_string());
    }

    #[test]
    fn test_sort() {
        let timestamp = UTC.timestamp(1485688933, 0);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = UTC.timestamp(0, 0);
        data_set.add_data(data_from_checked_bytes(timestamp, channel + 1, &LIVE_DATA_1 [0..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [352..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [0..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_TELEGRAM_1 [0..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [258..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [242..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [172..]));

        assert_eq!(7, data_set.as_data_slice().len());
        assert_eq!("12_0010_7E11_10_0100", data_set.as_data_slice() [0].id_string());
        assert_eq!("11_0000_7E11_20_0500_0000", data_set.as_data_slice() [1].id_string());
        assert_eq!("11_0010_7E11_10_0100", data_set.as_data_slice() [2].id_string());
        assert_eq!("11_7771_2011_30_25", data_set.as_data_slice() [3].id_string());
        assert_eq!("11_6651_7E11_10_0200", data_set.as_data_slice() [4].id_string());
        assert_eq!("11_0010_7E22_10_0100", data_set.as_data_slice() [5].id_string());
        assert_eq!("11_0015_7E11_10_0100", data_set.as_data_slice() [6].id_string());

        data_set.sort();

        assert_eq!(7, data_set.as_data_slice().len());
        assert_eq!("11_0000_7E11_20_0500_0000", data_set.as_data_slice() [0].id_string());
        assert_eq!("11_0010_7E11_10_0100", data_set.as_data_slice() [1].id_string());
        assert_eq!("11_0010_7E22_10_0100", data_set.as_data_slice() [2].id_string());
        assert_eq!("11_0015_7E11_10_0100", data_set.as_data_slice() [3].id_string());
        assert_eq!("11_6651_7E11_10_0200", data_set.as_data_slice() [4].id_string());
        assert_eq!("11_7771_2011_30_25", data_set.as_data_slice() [5].id_string());
        assert_eq!("12_0010_7E11_10_0100", data_set.as_data_slice() [6].id_string());
    }

    #[test]
    fn test_sort_by() {
        let timestamp = UTC.timestamp(1485688933, 0);
        let channel = 0x11;

        let mut data_set = DataSet::new();
        data_set.timestamp = UTC.timestamp(0, 0);
        data_set.add_data(data_from_checked_bytes(timestamp, channel + 1, &LIVE_DATA_1 [0..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [352..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [0..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_TELEGRAM_1 [0..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [258..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [242..]));
        data_set.add_data(data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [172..]));

        assert_eq!(7, data_set.as_data_slice().len());
        assert_eq!("12_0010_7E11_10_0100", data_set.as_data_slice() [0].id_string());
        assert_eq!("11_0000_7E11_20_0500_0000", data_set.as_data_slice() [1].id_string());
        assert_eq!("11_0010_7E11_10_0100", data_set.as_data_slice() [2].id_string());
        assert_eq!("11_7771_2011_30_25", data_set.as_data_slice() [3].id_string());
        assert_eq!("11_6651_7E11_10_0200", data_set.as_data_slice() [4].id_string());
        assert_eq!("11_0010_7E22_10_0100", data_set.as_data_slice() [5].id_string());
        assert_eq!("11_0015_7E11_10_0100", data_set.as_data_slice() [6].id_string());

        data_set.sort_by(|l, r| {
            let l_id = &l.id_string() [8..];
            let r_id = &r.id_string() [8..];
            l_id.cmp(r_id)
        });

        assert_eq!(7, data_set.as_data_slice().len());
        assert_eq!("11_7771_2011_30_25", data_set.as_data_slice() [0].id_string());
        assert_eq!("12_0010_7E11_10_0100", data_set.as_data_slice() [1].id_string());
        assert_eq!("11_0010_7E11_10_0100", data_set.as_data_slice() [2].id_string());
        assert_eq!("11_0015_7E11_10_0100", data_set.as_data_slice() [3].id_string());
        assert_eq!("11_6651_7E11_10_0200", data_set.as_data_slice() [4].id_string());
        assert_eq!("11_0000_7E11_20_0500_0000", data_set.as_data_slice() [5].id_string());
        assert_eq!("11_0010_7E22_10_0100", data_set.as_data_slice() [6].id_string());
    }
}
