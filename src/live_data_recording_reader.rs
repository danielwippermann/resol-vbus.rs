use std::collections::HashSet;
use std::io::Read;

use chrono::{DateTime, TimeZone, Utc};

use data::Data;
use data_set::DataSet;
use error::Result;
use live_data_decoder;
use recording_decoder;
use recording_reader::RecordingReader;
use stream_blob_length::StreamBlobLength::*;

#[derive(Debug, Default, PartialEq)]
pub struct LiveDataRecordingStats {
    total_record_count: usize,
    live_data_record_count: usize,
    live_data_record_byte_count: usize,
    malformed_byte_count: usize,
    data_count: usize,
    data_byte_count: usize,
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
}

impl<T: Read> LiveDataRecordingReader<T> {
    /// Construct a new `LiveDataRecordingReader<T>` instance.
    pub fn new(reader: T) -> LiveDataRecordingReader<T> {
        LiveDataRecordingReader {
            reader: RecordingReader::new(reader),
            min_timestamp: None,
            max_timestamp: None,
            buf: Vec::new(),
            timestamp: Utc.timestamp(0, 0),
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

    /// Quickly read to EOF of the source and return the DataSet for all uniquely found `Data` variants.
    pub fn read_topology_data_set(&mut self) -> Result<DataSet> {
        let mut set = HashSet::new();

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
                    panic!("Record type 0x88 too small: {}", len);
                }
            } else {
                panic!("Unexpected record type 0x{:02X}", record[1]);
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

                        self.timestamp = record_timestamp;
                        self.buf.extend_from_slice(&record[22..]);
                        break;
                    } else {
                        panic!("Record type 0x88 too small: {}", len);
                    }
                } else {
                    panic!("Unexpected record type 0x{:02X}", record[1]);
                }
            }
        }
    }

    /// Quickly read to EOF of the source and return the `LiveDataRecordingStats`.
    pub fn read_to_stats(&mut self) -> Result<LiveDataRecordingStats> {
        let has_timestamps = self.min_timestamp.is_some() || self.max_timestamp.is_some();

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
                    panic!("Record type 0x88 too small: {}", len);
                }
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

    use test_data::LIVE_DATA_RECORDING_1;

    #[test]
    fn test_read_topology_data_set() {
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        let data_set = ldrr.read_topology_data_set().unwrap();

        let data_slice = data_set.as_data_slice();
        assert_eq!(3, data_slice.len());
        assert_eq!("00_0010_7E11_10_0100", data_slice[0].id_string());
        assert_eq!("00_0015_7E11_10_0100", data_slice[1].id_string());
        assert_eq!("00_6655_7E11_10_0200", data_slice[2].id_string());
    }

    #[test]
    fn test_read_data() {
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        let data = ldrr.read_data().unwrap().unwrap();
        assert_eq!("00_0010_7E11_10_0100", data.id_string());

        let data = ldrr.read_data().unwrap().unwrap();
        assert_eq!("00_0015_7E11_10_0100", data.id_string());

        let data = ldrr.read_data().unwrap().unwrap();
        assert_eq!("00_6655_7E11_10_0200", data.id_string());

        let data = ldrr.read_data().unwrap().unwrap();
        assert_eq!("00_0000_7E11_20_0500_0000", data.id_string());

        let data = ldrr.read_data().unwrap().unwrap();
        assert_eq!("00_0010_7E11_10_0100", data.id_string());

        let data = ldrr.read_data().unwrap().unwrap();
        assert_eq!("00_0015_7E11_10_0100", data.id_string());

        let data = ldrr.read_data().unwrap().unwrap();
        assert_eq!("00_0000_7E11_20_0500_0000", data.id_string());

        let data = ldrr.read_data().unwrap();
        assert_eq!(None, data);
    }

    #[test]
    fn test_read_to_stats() {
        let mut ldrr = LiveDataRecordingReader::new(LIVE_DATA_RECORDING_1);

        let stats = ldrr.read_to_stats().expect("No error");

        assert_eq!(LiveDataRecordingStats {
            total_record_count: 18,
            live_data_record_count: 18,
            live_data_record_byte_count: 610,
            malformed_byte_count: 0,
            data_count: 7,
            data_byte_count: 610,
        }, stats);
    }
}
