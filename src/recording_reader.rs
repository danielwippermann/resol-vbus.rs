use std::io::{Read, Result};

use chrono::{DateTime, UTC};

use blob_reader::BlobReader;
use stream_blob_length::StreamBlobLength::{BlobLength, Partial, Malformed};
use data_set::DataSet;
use recording_decoder::{length_from_bytes, timestamp_from_checked_bytes, data_from_bytes};


/// Allows reading `Data` variants from a `Read` trait object.
#[derive(Debug)]
pub struct RecordingReader<R: Read> {
    current_channel: u8,
    reader: BlobReader<R>,
    previous_length: usize,
}


impl<R: Read> RecordingReader<R> {

    /// Constructs a `RecordingReader`.
    pub fn new(reader: R) -> RecordingReader<R> {
        RecordingReader {
            current_channel: 0,
            reader: BlobReader::new(reader),
            previous_length: 0,
        }
    }

    /// Read from the stream until a valid blob of data is found.
    pub fn read_bytes(&mut self) -> Result<&[u8]> {
        if self.previous_length > 0 {
            self.reader.consume(self.previous_length);
            self.previous_length = 0;
        }

        loop {
            match length_from_bytes(self.reader.as_bytes()) {
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

        let bytes = self.reader.as_bytes();
        Ok(&bytes [0..self.previous_length])
    }

    fn read_to_next_data_set_record(&mut self) -> Result<Option<DateTime<UTC>>> {
        loop {
            let bytes = self.read_bytes()?;
            let length = bytes.len();

            if length == 0 {
                return Ok(None)
            } else if bytes [1] == 0x44 {
                let data_set_timestamp = timestamp_from_checked_bytes(&bytes [6..14]);
                return Ok(Some(data_set_timestamp));
            }
        }
    }

    /// Read from the stream until a valid `DataSet` variant can be decoded.
    pub fn read_data_set(&mut self) -> Result<Option<DataSet>> {
        if let Some(data_set_timestamp) = self.read_to_next_data_set_record()? {
            let mut data_set = DataSet::new();
            data_set.timestamp = data_set_timestamp;

            let mut current_channel = 0u8;

            loop {
                let bytes = self.read_bytes()?;
                let length = bytes.len();

                if length == 0 {
                    break;
                } else if bytes [1] == 0x44 {
                    break;
                } else if bytes [1] == 0x66 {
                    if let Some(data) = data_from_bytes(current_channel, bytes) {
                        data_set.add_data(data);
                    }
                } else if bytes [1] == 0x77 {
                    if length >= 16 {
                        current_channel = bytes [14];
                    }
                } else {
                    panic!("Unsupported record type 0x{:02X}", bytes [1]);
                }
            }

            self.previous_length = 0;
            data_set.timestamp = data_set_timestamp;
            return Ok(Some(data_set))
        } else {
            Ok(None)
        }
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    use test_data::{RECORDING_1};

    #[test]
    fn test_read_bytes() {
        let mut rr = RecordingReader::new(RECORDING_1);
        for expected_len in [ 14, 70, 16, 134, 30, 66, 82, 82, 82, 82, 82, 0, 0, 0 ].iter() {
            let result = rr.read_bytes().unwrap();
            assert_eq!(*expected_len, result.len());
        }

        let mut rr = RecordingReader::new(&RECORDING_1 [1..]);
        for expected_len in [ 70, 16, 134, 30, 66, 82, 82, 82, 82, 82, 0, 0, 0 ].iter() {
            let result = rr.read_bytes().unwrap();
            assert_eq!(*expected_len, result.len());
        }
    }

    #[test]
    fn test_read_to_next_data_set_record() {
        let mut rr = RecordingReader::new(RECORDING_1);
        let timestamp = rr.read_to_next_data_set_record().unwrap().unwrap();
        assert_eq!("2017-01-09T09:57:29.009+00:00", timestamp.to_rfc3339());

        assert_eq!(true, rr.read_to_next_data_set_record().unwrap().is_none());
    }

    #[test]
    fn test_read_data_set() {
        let mut rr = RecordingReader::new(RECORDING_1);
        let data_set = rr.read_data_set().unwrap().unwrap();

        assert_eq!("2017-01-09T09:57:29.009+00:00", data_set.timestamp.to_rfc3339());
        assert_eq!(9, data_set.as_data_slice().len());
        assert_eq!("00_0010_0053_10_0100", data_set.as_data_slice() [0].to_id_string());
        assert_eq!("01_0010_7E11_10_0100", data_set.as_data_slice() [1].to_id_string());
        assert_eq!("01_0010_7E21_10_0100", data_set.as_data_slice() [2].to_id_string());
        assert_eq!("01_0015_7E11_10_0100", data_set.as_data_slice() [3].to_id_string());
        assert_eq!("01_6651_7E11_10_0200", data_set.as_data_slice() [4].to_id_string());
        assert_eq!("01_6652_7E11_10_0200", data_set.as_data_slice() [5].to_id_string());
        assert_eq!("01_6653_7E11_10_0200", data_set.as_data_slice() [6].to_id_string());
        assert_eq!("01_6654_7E11_10_0200", data_set.as_data_slice() [7].to_id_string());
        assert_eq!("01_6655_7E11_10_0200", data_set.as_data_slice() [8].to_id_string());

        assert_eq!(true, rr.read_data_set().unwrap().is_none());
    }
}
