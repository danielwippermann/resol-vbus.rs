use std::{collections::HashSet, io::Read};

use chrono::{DateTime, Utc};

use crate::{
    data::Data, data_set::DataSet, error::Result, live_data_decoder, recording_decoder,
    recording_reader::RecordingReader, stream_blob_length::StreamBlobLength::*,
    utils::utc_timestamp,
};

#[derive(Debug, Default, PartialEq)]
pub struct LiveDataRecordingStats {
    total_record_count: usize,
    live_data_record_count: usize,
    live_data_record_byte_count: usize,
    malformed_byte_count: usize,
    data_count: usize,
    data_byte_count: usize,
    max_channel: u8,
}

/// A `RecordingReader` for type 0x88 live data recordings.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::{FileListReader, LiveDataRecordingReader};
///
/// let files: Vec<_> = std::env::args().skip(1).collect();
///
/// let flr = FileListReader::new(files);
///
/// let mut ldrr = LiveDataRecordingReader::new(flr);
///
/// while let Some(data) = ldrr.read_data().unwrap() {
///     // process the data
///     println!("{}: {}", data.as_header().timestamp, data.id_string());
/// }
/// ```
#[derive(Debug)]
pub struct LiveDataRecordingReader<T: Read> {
    reader: RecordingReader<T>,
    min_timestamp: Option<DateTime<Utc>>,
    max_timestamp: Option<DateTime<Utc>>,
    buf: Vec<u8>,
    timestamp: DateTime<Utc>,
    channel: u8,
    current_channel: u8,
}

impl<T: Read> LiveDataRecordingReader<T> {
    /// Construct a new `LiveDataRecordingReader<T>` instance.
    pub fn new(reader: T) -> LiveDataRecordingReader<T> {
        LiveDataRecordingReader {
            reader: RecordingReader::new(reader),
            min_timestamp: None,
            max_timestamp: None,
            buf: Vec::new(),
            timestamp: utc_timestamp(0),
            channel: 0,
            current_channel: 0,
        }
    }

    /// Set optional minimum and maximum timestamps for prefiltering data.
    pub fn set_min_max_timestamps(
        &mut self,
        min_timestamp: Option<DateTime<Utc>>,
        max_timestamp: Option<DateTime<Utc>>,
    ) {
        self.min_timestamp = min_timestamp;
        self.max_timestamp = max_timestamp;
    }

    /// Set channel that `read_*` functions will filter data from.
    pub fn set_channel(&mut self, channel: u8) {
        self.channel = channel;
    }

    /// Quickly read to EOF of the source and return the DataSet for all uniquely found `Data` variants.
    pub fn read_topology_data_set(&mut self) -> Result<DataSet> {
        let mut set = HashSet::new();

        let mut current_channel = 0u8;

        let has_timestamps = self.min_timestamp.is_some() || self.max_timestamp.is_some();

        loop {
            let record = self.reader.read_record()?;
            let len = record.len();
            if len == 0 {
                break;
            }

            if record[1] == 0x88 {
                if len >= 22 {
                    if has_timestamps {
                        let record_timestamp =
                            recording_decoder::timestamp_from_checked_bytes(&record[14..22]);

                        if let Some(timestamp) = self.min_timestamp {
                            if record_timestamp < timestamp {
                                continue;
                            }
                        }

                        if let Some(timestamp) = self.max_timestamp {
                            if record_timestamp >= timestamp {
                                continue;
                            }
                        }
                    }

                    if current_channel != self.channel {
                        continue;
                    }

                    self.buf.extend_from_slice(&record[22..]);

                    let mut start = 0;
                    let mut consumed = 0;
                    while start < self.buf.len() {
                        match live_data_decoder::length_from_bytes(&self.buf[start..]) {
                            BlobLength(length) => {
                                if self.buf[start + 5] == 0x10 {
                                    let mut fingerprint = [0u8; 10];
                                    fingerprint[0] = 0;
                                    fingerprint[1] = self.buf[start + 1];
                                    fingerprint[2] = self.buf[start + 2];
                                    fingerprint[3] = self.buf[start + 3];
                                    fingerprint[4] = self.buf[start + 4];
                                    fingerprint[5] = self.buf[start + 5];
                                    fingerprint[6] = self.buf[start + 6];
                                    fingerprint[7] = self.buf[start + 7];
                                    fingerprint[8] = 0;
                                    fingerprint[9] = 0;
                                    set.insert(fingerprint);
                                }
                                start += length;
                            }
                            Partial => break,
                            Malformed => start += 1,
                        }

                        consumed = start;
                    }

                    if consumed > 0 {
                        drop(self.buf.drain(0..consumed));
                    }
                } else {
                    return Err(format!("Record type 0x88 too small: {len}").into());
                }
            } else if record[1] == 0x77 {
                if len >= 16 {
                    current_channel = record[14];
                } else {
                    return Err(format!("Record type 0x77 too small: {len}").into());
                }
            } else {
                return Err(format!("Unexpected record type 0x{:02X}", record[1]).into());
            }
        }

        let mut data_set = DataSet::new();

        for fingerprint in set {
            let mut fake_record = [0u8; 26];
            fake_record[0] = 0xA5;
            fake_record[1] = 0x66;
            fake_record[2] = 0x1A;
            fake_record[3] = 0x00;
            fake_record[4] = 0x1A;
            fake_record[5] = 0x00;
            fake_record[6] = 0x00;
            fake_record[7] = 0x00;
            fake_record[8] = 0x00;
            fake_record[9] = 0x00;
            fake_record[10] = 0x00;
            fake_record[11] = 0x00;
            fake_record[12] = 0x00;
            fake_record[13] = 0x00;
            fake_record[14] = fingerprint[1];
            fake_record[15] = fingerprint[2];
            fake_record[16] = fingerprint[3];
            fake_record[17] = fingerprint[4];
            fake_record[18] = fingerprint[5];
            fake_record[19] = 0;
            fake_record[20] = fingerprint[6];
            fake_record[21] = fingerprint[7];
            fake_record[22] = 0;
            fake_record[23] = 0;
            fake_record[24] = fingerprint[8];
            fake_record[25] = fingerprint[9];

            if let Some(data) = recording_decoder::data_from_bytes(fingerprint[0], &fake_record) {
                data_set.add_data(data);
            }
        }

        data_set.sort();

        Ok(data_set)
    }

    /// Read from the stream until a valid `Data` variant can be decoded.
    pub fn read_data(&mut self) -> Result<Option<Data>> {
        let has_timestamps = self.min_timestamp.is_some() || self.max_timestamp.is_some();

        loop {
            let mut start = 0;

            while start < self.buf.len() {
                match live_data_decoder::length_from_bytes(&self.buf[start..]) {
                    BlobLength(length) => {
                        let data = live_data_decoder::data_from_checked_bytes(
                            self.timestamp,
                            0,
                            &self.buf[start..start + length],
                        );

                        drop(self.buf.drain(0..start + length));

                        return Ok(Some(data));
                    }
                    Partial => break,
                    Malformed => start += 1,
                }
            }

            loop {
                let record = self.reader.read_record()?;
                let len = record.len();
                if len == 0 {
                    return Ok(None);
                }

                if record[1] == 0x88 {
                    if len >= 22 {
                        let record_timestamp =
                            recording_decoder::timestamp_from_checked_bytes(&record[14..22]);

                        if has_timestamps {
                            if let Some(timestamp) = self.min_timestamp {
                                if record_timestamp < timestamp {
                                    continue;
                                }
                            }

                            if let Some(timestamp) = self.max_timestamp {
                                if record_timestamp >= timestamp {
                                    continue;
                                }
                            }
                        }

                        if self.current_channel != self.channel {
                            continue;
                        }

                        self.timestamp = record_timestamp;
                        self.buf.extend_from_slice(&record[22..]);
                        break;
                    } else {
                        return Err(format!("Record type 0x88 too small: {len}").into());
                    }
                } else if record[1] == 0x77 {
                    if len >= 16 {
                        self.current_channel = record[14];
                    } else {
                        return Err(format!("Record type 0x77 too small: {len}").into());
                    }
                } else {
                    return Err(format!("Unexpected record type 0x{:02X}", record[1]).into());
                }
            }
        }
    }

    /// Quickly read to EOF of the source and return the `LiveDataRecordingStats`.
    pub fn read_to_stats(&mut self) -> Result<LiveDataRecordingStats> {
        let has_timestamps = self.min_timestamp.is_some() || self.max_timestamp.is_some();

        let mut current_channel = 0u8;

        let mut stats = LiveDataRecordingStats::default();

        loop {
            let record = self.reader.read_record()?;
            let len = record.len();
            if len == 0 {
                break;
            }

            stats.total_record_count += 1;

            if record[1] == 0x88 {
                if len >= 22 {
                    if has_timestamps {
                        let record_timestamp =
                            recording_decoder::timestamp_from_checked_bytes(&record[14..22]);

                        if let Some(timestamp) = self.min_timestamp {
                            if record_timestamp < timestamp {
                                continue;
                            }
                        }

                        if let Some(timestamp) = self.max_timestamp {
                            if record_timestamp >= timestamp {
                                continue;
                            }
                        }
                    }

                    if current_channel != self.channel {
                        continue;
                    }

                    stats.live_data_record_count += 1;
                    stats.live_data_record_byte_count += len - 22;

                    self.buf.extend_from_slice(&record[22..]);

                    let mut start = 0;
                    let mut consumed = 0;
                    while start < self.buf.len() {
                        match live_data_decoder::length_from_bytes(&self.buf[start..]) {
                            BlobLength(length) => {
                                stats.data_byte_count += length;
                                stats.data_count += 1;
                                start += length;
                            }
                            Partial => break,
                            Malformed => {
                                stats.malformed_byte_count += 1;
                                start += 1;
                            }
                        }

                        consumed = start;
                    }

                    if consumed > 0 {
                        drop(self.buf.drain(0..consumed));
                    }
                } else {
                    return Err(format!("Record type 0x88 too small: {len}").into());
                }
            } else if record[1] == 0x77 {
                if len >= 16 {
                    current_channel = record[14];
                    if stats.max_channel < current_channel {
                        stats.max_channel = current_channel;
                    }
                } else {
                    return Err(format!("Record type 0x77 too small: {len}").into());
                }
            } else {
                return Err(format!("Unexpected record type 0x{:02X}", record[1]).into());
            }
        }

        Ok(stats)
    }

    /// Get amount of already consumed bytes.
    pub fn offset(&self) -> usize {
        self.reader.offset()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        test_data::LIVE_DATA_RECORDING_1,
        test_utils::{test_debug_derive, test_partial_eq_derive},
    };

    #[test]
    fn test_live_data_recording_stats_derived_impls() {
        let stats = LiveDataRecordingStats::default();

        test_debug_derive(&stats);
        test_partial_eq_derive(&stats);
    }

    #[test]
    fn test_derived_impls() {
        let ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        test_debug_derive(&ldrr);
    }

    #[test]
    fn test_set_min_max_timestamps() {
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        assert_eq!(None, ldrr.min_timestamp);
        assert_eq!(None, ldrr.max_timestamp);

        let min_timestamp = utc_timestamp(1485688933);
        let max_timestamp = utc_timestamp(1672045902);

        ldrr.set_min_max_timestamps(Some(min_timestamp), Some(max_timestamp));

        assert_eq!(Some(utc_timestamp(1485688933)), ldrr.min_timestamp);
        assert_eq!(Some(utc_timestamp(1672045902)), ldrr.max_timestamp);
    }

    #[test]
    fn test_set_channel() {
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        assert_eq!(0, ldrr.channel);

        ldrr.set_channel(1);

        assert_eq!(1, ldrr.channel);
    }

    #[test]
    fn test_read_topology_data_set() -> Result<()> {
        // No timestamps and channel filtering
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        let data_set = ldrr.read_topology_data_set()?;

        let data_slice = data_set.as_data_slice();
        assert_eq!(3, data_slice.len());
        assert_eq!("00_0010_7E11_10_0100", data_slice[0].id_string());
        assert_eq!("00_0015_7E11_10_0100", data_slice[1].id_string());
        assert_eq!("00_6655_7E11_10_0200", data_slice[2].id_string());

        // Filter by min timestamp
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_min_max_timestamps(Some(utc_timestamp(1486857606)), None);

        let data_set = ldrr.read_topology_data_set()?;

        let data_slice = data_set.as_data_slice();
        assert_eq!(1, data_slice.len());
        assert_eq!("00_0015_7E11_10_0100", data_slice[0].id_string());

        // Filter by max timestamp
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_min_max_timestamps(None, Some(utc_timestamp(1486857603)));

        let data_set = ldrr.read_topology_data_set()?;

        let data_slice = data_set.as_data_slice();
        assert_eq!(1, data_slice.len());
        assert_eq!("00_0010_7E11_10_0100", data_slice[0].id_string());

        // Filter by min and max timestamp
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_min_max_timestamps(
            Some(utc_timestamp(1486857603)),
            Some(utc_timestamp(1486857604)),
        );

        let data_set = ldrr.read_topology_data_set()?;

        let data_slice = data_set.as_data_slice();
        assert_eq!(2, data_slice.len());
        assert_eq!("00_0015_7E11_10_0100", data_slice[0].id_string());
        assert_eq!("00_6655_7E11_10_0200", data_slice[1].id_string());

        // Filter by channel
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_channel(1);

        let data_set = ldrr.read_topology_data_set()?;

        let data_slice = data_set.as_data_slice();
        assert_eq!(0, data_slice.len());

        // Malformed live data
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x88, 0x1C, 0x00, 0x1C, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 21 */ 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, /* 22 - 27 */ 0xAA, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let data_set = ldrr.read_topology_data_set()?;

        assert_eq!(0, data_set.len());

        // Malformed record 0x88 (too small)
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x88, 0x0E, 0x00, 0x0E, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let error = ldrr.read_topology_data_set().err().unwrap();

        assert_eq!("Record type 0x88 too small: 14", error.to_string());

        // Malformed record 0x77 (too small)
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x77, 0x0E, 0x00, 0x0E, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let error = ldrr.read_topology_data_set().err().unwrap();

        assert_eq!("Record type 0x77 too small: 14", error.to_string());

        // Unexpected record type
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let error = ldrr.read_topology_data_set().err().unwrap();

        assert_eq!("Unexpected record type 0x44", error.to_string());

        // Channel switch commands
        let bytes: &[u8] = &[
            0xA5, 0x77, 0x10, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let data_set = ldrr.read_topology_data_set()?;

        assert_eq!(0, data_set.len());

        // NOTE(daniel): current_channel is not modified for topology scans
        assert_eq!(0, ldrr.current_channel);

        Ok(())
    }

    #[test]
    fn test_read_data() -> Result<()> {
        // No timestamps and channel filtering
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0010_7E11_10_0100", data.id_string());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0015_7E11_10_0100", data.id_string());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_6655_7E11_10_0200", data.id_string());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0000_7E11_20_0500_0000", data.id_string());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0010_7E11_10_0100", data.id_string());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0015_7E11_10_0100", data.id_string());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0000_7E11_20_0500_0000", data.id_string());

        let data = ldrr.read_data()?;
        assert_eq!(None, data);

        // Filter by min timestamp
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_min_max_timestamps(Some(utc_timestamp(1486857606)), None);

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0015_7E11_10_0100", data.id_string());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0000_7E11_20_0500_0000", data.id_string());

        let data = ldrr.read_data()?;
        assert_eq!(None, data);

        // Filter by max timestamp
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_min_max_timestamps(None, Some(utc_timestamp(1486857603)));

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0010_7E11_10_0100", data.id_string());

        let data = ldrr.read_data()?;
        assert_eq!(None, data);

        // Filter by min and max timestamp
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_min_max_timestamps(
            Some(utc_timestamp(1486857603)),
            Some(utc_timestamp(1486857604)),
        );

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0015_7E11_10_0100", data.id_string());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_6655_7E11_10_0200", data.id_string());

        let data = ldrr.read_data()?;
        assert_eq!(None, data);

        // Filter by channel
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_channel(1);

        let data = ldrr.read_data()?;
        assert_eq!(None, data);

        // Malformed live data
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x88, 0x1C, 0x00, 0x1C, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 21 */ 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, /* 22 - 27 */ 0xAA, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let data = ldrr.read_data()?;

        assert_eq!(None, data);

        // Malformed record 0x88 (too small)
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x88, 0x0E, 0x00, 0x0E, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let error = ldrr.read_data().err().unwrap();

        assert_eq!("Record type 0x88 too small: 14", error.to_string());

        // Malformed record 0x77 (too small)
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x77, 0x0E, 0x00, 0x0E, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let error = ldrr.read_data().err().unwrap();

        assert_eq!("Record type 0x77 too small: 14", error.to_string());

        // Unexpected record type
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let error = ldrr.read_data().err().unwrap();

        assert_eq!("Unexpected record type 0x44", error.to_string());

        // Channel switch
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x77, 0x10, 0x00, 0x10, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 15 */ 0x01, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let data = ldrr.read_data()?;

        assert_eq!(None, data);
        assert_eq!(1, ldrr.current_channel);

        // Malformed data after valid data
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x88, 0x27, 0x00, 0x27, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 21 */ 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, /* 22 - 37 */ 0xaa, 0x00, 0x00, 0x11, 0x7e, 0x20, 0x00, 0x05,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4b, /* 38 */ 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let data = ldrr.read_data()?.expect("Should return Data");

        assert_eq!("00_0000_7E11_20_0500_0000", data.id_string());
        assert_eq!(1, ldrr.buf.len());

        let data = ldrr.read_data()?;

        assert_eq!(None, data);
        assert_eq!(1, ldrr.buf.len()); // TODO(daniel): hmmm, those malformed should be consumed...

        Ok(())
    }

    #[test]
    fn test_read_to_stats() -> Result<()> {
        // No timestamps and channel filtering
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        let stats = ldrr.read_to_stats()?;

        assert_eq!(
            LiveDataRecordingStats {
                total_record_count: 18,
                live_data_record_count: 18,
                live_data_record_byte_count: 610,
                malformed_byte_count: 0,
                data_count: 7,
                data_byte_count: 610,
                max_channel: 0,
            },
            stats
        );

        // Filter by min timestamp
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_min_max_timestamps(Some(utc_timestamp(1486857606)), None);

        let stats = ldrr.read_to_stats()?;

        assert_eq!(
            LiveDataRecordingStats {
                total_record_count: 18,
                live_data_record_count: 3,
                live_data_record_byte_count: 86,
                malformed_byte_count: 0,
                data_count: 2,
                data_byte_count: 86,
                max_channel: 0,
            },
            stats
        );

        // Filter by max timestamp
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_min_max_timestamps(None, Some(utc_timestamp(1486857603)));

        let stats = ldrr.read_to_stats()?;

        assert_eq!(
            LiveDataRecordingStats {
                total_record_count: 18,
                live_data_record_count: 5,
                live_data_record_byte_count: 172,
                malformed_byte_count: 0,
                data_count: 1,
                data_byte_count: 172,
                max_channel: 0,
            },
            stats
        );

        // Filter by min and max timestamp
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_min_max_timestamps(
            Some(utc_timestamp(1486857603)),
            Some(utc_timestamp(1486857604)),
        );

        let stats = ldrr.read_to_stats()?;

        assert_eq!(
            LiveDataRecordingStats {
                total_record_count: 18,
                live_data_record_count: 4,
                live_data_record_byte_count: 164,
                malformed_byte_count: 0,
                data_count: 2,
                data_byte_count: 164,
                max_channel: 0,
            },
            stats
        );

        // Filter by channel
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        ldrr.set_channel(1);

        let stats = ldrr.read_to_stats()?;

        assert_eq!(
            LiveDataRecordingStats {
                total_record_count: 18,
                live_data_record_count: 0,
                live_data_record_byte_count: 0,
                malformed_byte_count: 0,
                data_count: 0,
                data_byte_count: 0,
                max_channel: 0,
            },
            stats
        );

        // Malformed live data
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x88, 0x1C, 0x00, 0x1C, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 21 */ 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, /* 22 - 27 */ 0xAA, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let stats = ldrr.read_to_stats()?;

        assert_eq!(
            LiveDataRecordingStats {
                total_record_count: 1,
                live_data_record_count: 1,
                live_data_record_byte_count: 6,
                malformed_byte_count: 6,
                data_count: 0,
                data_byte_count: 0,
                max_channel: 0,
            },
            stats
        );

        // Malformed record 0x88 (too small)
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x88, 0x0E, 0x00, 0x0E, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let error = ldrr.read_to_stats().err().unwrap();

        assert_eq!("Record type 0x88 too small: 14", error.to_string());

        // Unexpected record type
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let error = ldrr.read_to_stats().err().unwrap();

        assert_eq!("Unexpected record type 0x44", error.to_string());

        // Channel switch
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x77, 0x10, 0x00, 0x10, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* 14 - 15 */ 0x01, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let stats = ldrr.read_to_stats()?;

        assert_eq!(
            LiveDataRecordingStats {
                total_record_count: 1,
                live_data_record_count: 0,
                live_data_record_byte_count: 0,
                malformed_byte_count: 0,
                data_count: 0,
                data_byte_count: 0,
                max_channel: 1,
            },
            stats
        );

        assert_eq!(0, ldrr.current_channel);

        // Malformed record 0x77 (too small)
        let bytes: &[u8] = &[
            /*  0 -  5 */ 0xA5, 0x77, 0x0E, 0x00, 0x0E, 0x00, /*  6 - 13 */ 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut ldrr = LiveDataRecordingReader::new(bytes);

        let error = ldrr.read_to_stats().err().unwrap();

        assert_eq!("Record type 0x77 too small: 14", error.to_string());

        Ok(())
    }

    #[test]
    fn test_offset() -> Result<()> {
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        assert_eq!(0, ldrr.offset());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0010_7E11_10_0100", data.id_string());

        assert_eq!(254, ldrr.offset());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0015_7E11_10_0100", data.id_string());

        assert_eq!(344, ldrr.offset());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_6655_7E11_10_0200", data.id_string());

        assert_eq!(464, ldrr.offset());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0000_7E11_20_0500_0000", data.id_string());

        assert_eq!(534, ldrr.offset());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0010_7E11_10_0100", data.id_string());

        assert_eq!(826, ldrr.offset());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0015_7E11_10_0100", data.id_string());

        assert_eq!(916, ldrr.offset());

        let data = ldrr.read_data()?.unwrap();
        assert_eq!("00_0000_7E11_20_0500_0000", data.id_string());

        assert_eq!(968, ldrr.offset());

        let data = ldrr.read_data()?;
        assert_eq!(None, data);

        assert_eq!(1006, ldrr.offset());

        Ok(())
    }
}
