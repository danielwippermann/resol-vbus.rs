use std::{collections::HashSet, io::Read};

use chrono::{DateTime, Utc};

use crate::{
    blob_reader::BlobReader,
    data_set::DataSet,
    error::Result,
    recording_comment::RecordingComment,
    recording_decoder::{data_from_bytes, length_from_bytes, timestamp_from_checked_bytes},
    stream_blob_length::StreamBlobLength::{BlobLength, Malformed, Partial},
};

/// Allows reading `Data` variants from a `Read` trait object.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::{FileListReader, RecordingReader};
///
/// let files: Vec<_> = std::env::args().skip(1).collect();
///
/// let flr = FileListReader::new(files);
///
/// let mut rr = RecordingReader::new(flr);
///
/// while let Some(data_set) = rr.read_data_set().unwrap() {
///     // process the data set
///     println!("{}:", data_set.timestamp);
///
///     for data in data_set.iter() {
///         println!("  - {}", data.id_string());
///     }
/// }
/// ```
#[derive(Debug)]
pub struct RecordingReader<R: Read> {
    reader: BlobReader<R>,
    previous_length: usize,
    min_timestamp: Option<DateTime<Utc>>,
    max_timestamp: Option<DateTime<Utc>>,
}

impl<R: Read> RecordingReader<R> {
    /// Constructs a `RecordingReader`.
    pub fn new(reader: R) -> RecordingReader<R> {
        RecordingReader {
            reader: BlobReader::new(reader),
            previous_length: 0,
            min_timestamp: None,
            max_timestamp: None,
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

    /// Read from the stream until a valid blob of data is found.
    pub fn read_record(&mut self) -> Result<&[u8]> {
        if self.previous_length > 0 {
            self.reader.consume(self.previous_length);
            self.previous_length = 0;
        }

        loop {
            match length_from_bytes(&self.reader) {
                BlobLength(size) => {
                    self.previous_length = size;
                    break;
                }
                Partial => {
                    if self.reader.read()? == 0 {
                        break;
                    }
                }
                Malformed => {
                    self.reader.consume(1);
                }
            }
        }

        let bytes = &*self.reader;
        Ok(&bytes[0..self.previous_length])
    }

    fn read_to_next_data_set_record(&mut self) -> Result<Option<DateTime<Utc>>> {
        let min_timestamp = self.min_timestamp;
        let max_timestamp = self.max_timestamp;
        let has_timestamps = min_timestamp.is_some() || max_timestamp.is_some();

        loop {
            let bytes = self.read_record()?;
            let length = bytes.len();

            if length == 0 {
                return Ok(None);
            } else if bytes[1] == 0x44 {
                let data_set_timestamp = timestamp_from_checked_bytes(&bytes[6..14]);

                if has_timestamps {
                    if let Some(timestamp) = min_timestamp {
                        if data_set_timestamp < timestamp {
                            continue;
                        }
                    }

                    if let Some(timestamp) = max_timestamp {
                        if data_set_timestamp >= timestamp {
                            continue;
                        }
                    }
                }

                return Ok(Some(data_set_timestamp));
            }
        }
    }

    /// Read a `DataSet` and corresponding comments from the stream until the `DataSet` is complete.
    pub fn read_data_set_and_comments(
        &mut self,
    ) -> Result<Option<(DataSet, Vec<RecordingComment>)>> {
        if let Some(data_set_timestamp) = self.read_to_next_data_set_record()? {
            let mut data_set = DataSet::new();
            data_set.timestamp = data_set_timestamp;

            let mut comments = Vec::new();

            let mut current_channel = 0u8;

            loop {
                let bytes = self.read_record()?;
                let length = bytes.len();

                if length == 0 || bytes[1] == 0x44 {
                    break;
                } else if bytes[1] == 0x66 {
                    if let Some(data) = data_from_bytes(current_channel, bytes) {
                        data_set.add_data(data);
                    }
                } else if bytes[1] == 0x77 {
                    if length >= 16 {
                        current_channel = bytes[14];
                    }
                } else if bytes[1] == 0x99 {
                    let timestamp = timestamp_from_checked_bytes(&bytes[6..14]);
                    let comment = RecordingComment::new(timestamp, Vec::from(&bytes[14..]));
                    comments.push(comment);
                } else {
                    return Err(format!("Unsupported record type 0x{:02X}", bytes[1]).into());
                }
            }

            self.previous_length = 0;
            data_set.timestamp = data_set_timestamp;
            Ok(Some((data_set, comments)))
        } else {
            Ok(None)
        }
    }

    /// Read from the stream until a valid `DataSet` variant can be decoded.
    pub fn read_data_set(&mut self) -> Result<Option<DataSet>> {
        if let Some(data_set_timestamp) = self.read_to_next_data_set_record()? {
            let mut data_set = DataSet::new();
            data_set.timestamp = data_set_timestamp;

            let mut current_channel = 0u8;

            loop {
                let bytes = self.read_record()?;
                let length = bytes.len();

                if length == 0 || bytes[1] == 0x44 {
                    break;
                } else if bytes[1] == 0x66 {
                    if let Some(data) = data_from_bytes(current_channel, bytes) {
                        data_set.add_data(data);
                    }
                } else if bytes[1] == 0x77 {
                    if length >= 16 {
                        current_channel = bytes[14];
                    }
                } else if bytes[1] == 0x99 {
                    // ignore comments
                } else {
                    return Err(format!("Unsupported record type 0x{:02X}", bytes[1]).into());
                }
            }

            self.previous_length = 0;
            data_set.timestamp = data_set_timestamp;
            Ok(Some(data_set))
        } else {
            Ok(None)
        }
    }

    /// Quickly read to EOF of the source and return the DataSet for all uniquely found `Data` variants.
    pub fn read_topology_data_set(&mut self) -> Result<DataSet> {
        let mut set = HashSet::new();

        let mut current_channel = 0u8;

        let min_timestamp = self.min_timestamp;
        let max_timestamp = self.max_timestamp;
        let has_timestamps = min_timestamp.is_some() || max_timestamp.is_some();
        let mut is_valid_timestamp = false;

        loop {
            let record = self.read_record()?;
            let length = record.len();
            if length == 0 {
                break;
            }

            if record[1] == 0x44 {
                current_channel = 0;
                is_valid_timestamp = true;

                if has_timestamps {
                    let record_timestamp = timestamp_from_checked_bytes(&record[6..14]);

                    if let Some(timestamp) = min_timestamp {
                        if record_timestamp < timestamp {
                            is_valid_timestamp = false;
                        }
                    }

                    if let Some(timestamp) = max_timestamp {
                        if record_timestamp >= timestamp {
                            is_valid_timestamp = false;
                        }
                    }
                }
            } else if !is_valid_timestamp {
                // nop
            } else if record[1] == 0x66 {
                if length >= 26 {
                    let mut fingerprint = [0u8; 10];
                    fingerprint[0] = current_channel;
                    fingerprint[1] = record[14];
                    fingerprint[2] = record[15];
                    fingerprint[3] = record[16];
                    fingerprint[4] = record[17];
                    fingerprint[5] = record[18];
                    fingerprint[6] = record[20];
                    fingerprint[7] = record[21];
                    fingerprint[8] = record[24];
                    fingerprint[9] = record[25];
                    set.insert(fingerprint);
                }
            } else if record[1] == 0x77 {
                if length >= 16 {
                    current_channel = record[14];
                }
            } else if record[1] == 0x99 {
                // ignore comments
            } else {
                return Err(format!("Unsupported record type 0x{:02X}", record[1]).into());
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

            if let Some(data) = data_from_bytes(fingerprint[0], &fake_record) {
                data_set.add_data(data);
            }
        }

        data_set.sort();
        Ok(data_set)
    }

    /// Get amount of already consumed bytes.
    pub fn offset(&self) -> usize {
        self.reader.offset()
    }
}

impl<R: Read> AsRef<R> for RecordingReader<R> {
    fn as_ref(&self) -> &R {
        self.reader.as_ref()
    }
}

impl<R: Read> AsMut<R> for RecordingReader<R> {
    fn as_mut(&mut self) -> &mut R {
        self.reader.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        test_data::{RECORDING_1, RECORDING_4},
        utils::utc_timestamp,
    };

    #[test]
    fn test_set_min_max_timestamps() {
        let mut ldrr = RecordingReader::new(RECORDING_1);

        assert_eq!(None, ldrr.min_timestamp);
        assert_eq!(None, ldrr.max_timestamp);

        let min_timestamp = utc_timestamp(1485688933);
        let max_timestamp = utc_timestamp(1672045902);

        ldrr.set_min_max_timestamps(Some(min_timestamp), Some(max_timestamp));

        assert_eq!(Some(utc_timestamp(1485688933)), ldrr.min_timestamp);
        assert_eq!(Some(utc_timestamp(1672045902)), ldrr.max_timestamp);
    }

    #[test]
    fn test_read_record() {
        let mut rr = RecordingReader::new(RECORDING_1);
        for expected_len in [14, 70, 16, 134, 30, 66, 82, 82, 82, 82, 82, 0, 0, 0].iter() {
            let result = rr.read_record().unwrap();
            assert_eq!(*expected_len, result.len());
        }

        let mut rr = RecordingReader::new(&RECORDING_1[1..]);
        for expected_len in [70, 16, 134, 30, 66, 82, 82, 82, 82, 82, 0, 0, 0].iter() {
            let result = rr.read_record().unwrap();
            assert_eq!(*expected_len, result.len());
        }
    }

    #[test]
    fn test_read_to_next_data_set_record() {
        // without timestamp filtering
        let mut rr = RecordingReader::new(RECORDING_1);
        let timestamp = rr.read_to_next_data_set_record().unwrap().unwrap();
        assert_eq!("2017-01-09T09:57:29.009+00:00", timestamp.to_rfc3339());

        assert_eq!(true, rr.read_to_next_data_set_record().unwrap().is_none());

        // with min timestamp
        let mut rr = RecordingReader::new(RECORDING_1);
        rr.set_min_max_timestamps(Some(utc_timestamp(1483955849)), None);

        let timestamp = rr.read_to_next_data_set_record().unwrap().unwrap();
        assert_eq!("2017-01-09T09:57:29.009+00:00", timestamp.to_rfc3339());

        assert_eq!(true, rr.read_to_next_data_set_record().unwrap().is_none());

        let mut rr = RecordingReader::new(RECORDING_1);
        rr.set_min_max_timestamps(Some(utc_timestamp(1483955850)), None);

        assert_eq!(true, rr.read_to_next_data_set_record().unwrap().is_none());

        // with max timestamp
        let mut rr = RecordingReader::new(RECORDING_1);
        rr.set_min_max_timestamps(None, Some(utc_timestamp(1483955850)));

        let timestamp = rr.read_to_next_data_set_record().unwrap().unwrap();
        assert_eq!("2017-01-09T09:57:29.009+00:00", timestamp.to_rfc3339());

        assert_eq!(true, rr.read_to_next_data_set_record().unwrap().is_none());

        let mut rr = RecordingReader::new(RECORDING_1);
        rr.set_min_max_timestamps(None, Some(utc_timestamp(1483955849)));

        assert_eq!(true, rr.read_to_next_data_set_record().unwrap().is_none());

        // with min and max timestamps
        let mut rr = RecordingReader::new(RECORDING_1);
        rr.set_min_max_timestamps(
            Some(utc_timestamp(1483955849)),
            Some(utc_timestamp(1483955850)),
        );

        let timestamp = rr.read_to_next_data_set_record().unwrap().unwrap();
        assert_eq!("2017-01-09T09:57:29.009+00:00", timestamp.to_rfc3339());

        assert_eq!(true, rr.read_to_next_data_set_record().unwrap().is_none());
    }

    #[test]
    fn test_read_data_set_and_comments() {
        let mut rr = RecordingReader::new(RECORDING_1);
        let (data_set, comments) = rr.read_data_set_and_comments().unwrap().unwrap();

        assert_eq!(
            "2017-01-09T09:57:29.009+00:00",
            data_set.timestamp.to_rfc3339()
        );
        assert_eq!(9, data_set.as_data_slice().len());
        assert_eq!(
            "00_0010_0053_10_0100",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "01_0010_7E11_10_0100",
            data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "01_0010_7E21_10_0100",
            data_set.as_data_slice()[2].id_string()
        );
        assert_eq!(
            "01_0015_7E11_10_0100",
            data_set.as_data_slice()[3].id_string()
        );
        assert_eq!(
            "01_6651_7E11_10_0200",
            data_set.as_data_slice()[4].id_string()
        );
        assert_eq!(
            "01_6652_7E11_10_0200",
            data_set.as_data_slice()[5].id_string()
        );
        assert_eq!(
            "01_6653_7E11_10_0200",
            data_set.as_data_slice()[6].id_string()
        );
        assert_eq!(
            "01_6654_7E11_10_0200",
            data_set.as_data_slice()[7].id_string()
        );
        assert_eq!(
            "01_6655_7E11_10_0200",
            data_set.as_data_slice()[8].id_string()
        );
        assert_eq!(0, comments.len());

        assert_eq!(true, rr.read_data_set_and_comments().unwrap().is_none());

        let mut rr = RecordingReader::new(RECORDING_4);
        let (data_set, comments) = rr.read_data_set_and_comments().unwrap().unwrap();

        assert_eq!(
            "2017-01-09T09:57:29.009+00:00",
            data_set.timestamp.to_rfc3339()
        );
        assert_eq!(9, data_set.as_data_slice().len());
        assert_eq!(
            "00_0010_0053_10_0100",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "01_0010_7E11_10_0100",
            data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "01_0010_7E21_10_0100",
            data_set.as_data_slice()[2].id_string()
        );
        assert_eq!(
            "01_0015_7E11_10_0100",
            data_set.as_data_slice()[3].id_string()
        );
        assert_eq!(
            "01_6651_7E11_10_0200",
            data_set.as_data_slice()[4].id_string()
        );
        assert_eq!(
            "01_6652_7E11_10_0200",
            data_set.as_data_slice()[5].id_string()
        );
        assert_eq!(
            "01_6653_7E11_10_0200",
            data_set.as_data_slice()[6].id_string()
        );
        assert_eq!(
            "01_6654_7E11_10_0200",
            data_set.as_data_slice()[7].id_string()
        );
        assert_eq!(
            "01_6655_7E11_10_0200",
            data_set.as_data_slice()[8].id_string()
        );
        assert_eq!(1, comments.len());
        assert_eq!(23, comments[0].comment().len());

        assert_eq!(true, rr.read_data_set_and_comments().unwrap().is_none());

        // Unsupported record type
        let bytes: &[u8] = &[
            0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xA5, 0x88, 0x0E, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut rr = RecordingReader::new(bytes);

        let error = rr.read_data_set_and_comments().err().unwrap();

        assert_eq!("Unsupported record type 0x88", error.to_string());
    }

    #[test]
    fn test_read_data_set() {
        let mut rr = RecordingReader::new(RECORDING_1);
        let data_set = rr.read_data_set().unwrap().unwrap();

        assert_eq!(
            "2017-01-09T09:57:29.009+00:00",
            data_set.timestamp.to_rfc3339()
        );
        assert_eq!(9, data_set.as_data_slice().len());
        assert_eq!(
            "00_0010_0053_10_0100",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "01_0010_7E11_10_0100",
            data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "01_0010_7E21_10_0100",
            data_set.as_data_slice()[2].id_string()
        );
        assert_eq!(
            "01_0015_7E11_10_0100",
            data_set.as_data_slice()[3].id_string()
        );
        assert_eq!(
            "01_6651_7E11_10_0200",
            data_set.as_data_slice()[4].id_string()
        );
        assert_eq!(
            "01_6652_7E11_10_0200",
            data_set.as_data_slice()[5].id_string()
        );
        assert_eq!(
            "01_6653_7E11_10_0200",
            data_set.as_data_slice()[6].id_string()
        );
        assert_eq!(
            "01_6654_7E11_10_0200",
            data_set.as_data_slice()[7].id_string()
        );
        assert_eq!(
            "01_6655_7E11_10_0200",
            data_set.as_data_slice()[8].id_string()
        );

        assert_eq!(true, rr.read_data_set().unwrap().is_none());

        let mut rr = RecordingReader::new(RECORDING_4);
        let data_set = rr.read_data_set().unwrap().unwrap();

        assert_eq!(
            "2017-01-09T09:57:29.009+00:00",
            data_set.timestamp.to_rfc3339()
        );
        assert_eq!(9, data_set.as_data_slice().len());
        assert_eq!(
            "00_0010_0053_10_0100",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "01_0010_7E11_10_0100",
            data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "01_0010_7E21_10_0100",
            data_set.as_data_slice()[2].id_string()
        );
        assert_eq!(
            "01_0015_7E11_10_0100",
            data_set.as_data_slice()[3].id_string()
        );
        assert_eq!(
            "01_6651_7E11_10_0200",
            data_set.as_data_slice()[4].id_string()
        );
        assert_eq!(
            "01_6652_7E11_10_0200",
            data_set.as_data_slice()[5].id_string()
        );
        assert_eq!(
            "01_6653_7E11_10_0200",
            data_set.as_data_slice()[6].id_string()
        );
        assert_eq!(
            "01_6654_7E11_10_0200",
            data_set.as_data_slice()[7].id_string()
        );
        assert_eq!(
            "01_6655_7E11_10_0200",
            data_set.as_data_slice()[8].id_string()
        );

        assert_eq!(true, rr.read_data_set().unwrap().is_none());

        // Unsupported record type
        let bytes: &[u8] = &[
            0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xA5, 0x88, 0x0E, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut rr = RecordingReader::new(bytes);

        let error = rr.read_data_set().err().unwrap();

        assert_eq!("Unsupported record type 0x88", error.to_string());
    }

    #[test]
    fn test_read_topology_data_set() -> Result<()> {
        let timestamp = utc_timestamp(1483955849);
        let timestamp_plus_one = utc_timestamp(1483955850);

        // without timestamp filtering
        let mut rr = RecordingReader::new(RECORDING_1);
        let data_set = rr.read_topology_data_set()?;

        assert_eq!(9, data_set.as_data_slice().len());
        assert_eq!(
            "00_0010_0053_10_0100",
            data_set.as_data_slice()[0].id_string()
        );
        assert_eq!(
            "01_0010_7E11_10_0100",
            data_set.as_data_slice()[1].id_string()
        );
        assert_eq!(
            "01_0010_7E21_10_0100",
            data_set.as_data_slice()[2].id_string()
        );
        assert_eq!(
            "01_0015_7E11_10_0100",
            data_set.as_data_slice()[3].id_string()
        );
        assert_eq!(
            "01_6651_7E11_10_0200",
            data_set.as_data_slice()[4].id_string()
        );
        assert_eq!(
            "01_6652_7E11_10_0200",
            data_set.as_data_slice()[5].id_string()
        );
        assert_eq!(
            "01_6653_7E11_10_0200",
            data_set.as_data_slice()[6].id_string()
        );
        assert_eq!(
            "01_6654_7E11_10_0200",
            data_set.as_data_slice()[7].id_string()
        );
        assert_eq!(
            "01_6655_7E11_10_0200",
            data_set.as_data_slice()[8].id_string()
        );

        // with min timestamp filtering
        let mut rr = RecordingReader::new(RECORDING_1);
        rr.set_min_max_timestamps(Some(timestamp), None);
        assert_eq!(9, rr.read_topology_data_set()?.len());

        let mut rr = RecordingReader::new(RECORDING_1);
        rr.set_min_max_timestamps(Some(timestamp_plus_one), None);
        assert_eq!(0, rr.read_topology_data_set()?.len());

        // with max timestamp filtering
        let mut rr = RecordingReader::new(RECORDING_1);
        rr.set_min_max_timestamps(None, Some(timestamp_plus_one));
        assert_eq!(9, rr.read_topology_data_set()?.len());

        let mut rr = RecordingReader::new(RECORDING_1);
        rr.set_min_max_timestamps(None, Some(timestamp));
        assert_eq!(0, rr.read_topology_data_set()?.len());

        // with min and max timestamp filtering
        let mut rr = RecordingReader::new(RECORDING_1);
        rr.set_min_max_timestamps(Some(timestamp), Some(timestamp_plus_one));
        assert_eq!(9, rr.read_topology_data_set()?.len());

        // with comments records in input
        let mut rr = RecordingReader::new(RECORDING_4);
        assert_eq!(9, rr.read_topology_data_set()?.len());

        // Unsupported record type
        let bytes: &[u8] = &[
            0xA5, 0x44, 0x0E, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xA5, 0x88, 0x0E, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut rr = RecordingReader::new(bytes);

        let error = rr.read_topology_data_set().err().unwrap();

        assert_eq!("Unsupported record type 0x88", error.to_string());

        Ok(())
    }

    #[test]
    fn test_as_ref() {
        let rr = RecordingReader::new(RECORDING_1);

        assert_eq!(&RECORDING_1, rr.as_ref());
    }

    #[test]
    fn test_as_mut() {
        let mut rr = RecordingReader::new(RECORDING_1);

        assert_eq!(&RECORDING_1, rr.as_mut());
    }
}
