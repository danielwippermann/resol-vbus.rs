use std::io::{Read, Result};

use chrono::{UTC};

use blob_reader::BlobReader;
use data::Data;
use stream_blob_length::StreamBlobLength::{BlobLength, Partial, Malformed};
use live_data_decoder::{length_from_bytes, data_from_checked_bytes};


/// Allows reading `Data` variants from a `Read` trait object.
#[derive(Debug)]
pub struct LiveDataReader<R: Read> {
    channel: u8,
    reader: BlobReader<R>,
    previous_length: usize,
}


impl<R: Read> LiveDataReader<R> {

    /// Constructs a `LiveDataReader`.
    pub fn new(channel: u8, reader: R) -> LiveDataReader<R> {
        LiveDataReader {
            channel: channel,
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

    /// Read from the stream until a valid `Data` variant can be decoded.
    pub fn read_data(&mut self) -> Result<Option<Data>> {
        let channel = self.channel;
        let bytes = self.read_bytes()?;

        let data = if bytes.len() > 0 {
            Some(data_from_checked_bytes(UTC::now(), channel, bytes))
        } else {
            None
        };

        Ok(data)
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    use test_data::LIVE_DATA_1;

    #[test]
    fn test_read_bytes() {
        let mut ldr = LiveDataReader::new(0, LIVE_DATA_1);

        for expected_len in [ 172, 70, 16, 94, 16, 0, 0, 0 ].iter() {
            let result = ldr.read_bytes().unwrap();
            assert_eq!(*expected_len, result.len());
        }

        let mut ldr = LiveDataReader::new(0, &LIVE_DATA_1 [1..]);

        for expected_len in [ 70, 16, 94, 16, 0, 0, 0 ].iter() {
            let result = ldr.read_bytes().unwrap();
            assert_eq!(*expected_len, result.len());
        }
    }

    #[test]
    fn test_read_data() {
        let channel = 0x11;

        let mut ldr = LiveDataReader::new(channel, LIVE_DATA_1);

        let data = ldr.read_data().unwrap().unwrap();

        assert_eq!("11_0010_7E11_10_0100", data.id_string());

        let data = ldr.read_data().unwrap().unwrap();

        assert_eq!("11_0015_7E11_10_0100", data.id_string());

        let data = ldr.read_data().unwrap().unwrap();

        assert_eq!("11_0010_7E22_10_0100", data.id_string());

        let data = ldr.read_data().unwrap().unwrap();

        assert_eq!("11_6651_7E11_10_0200", data.id_string());

        let data = ldr.read_data().unwrap().unwrap();

        assert_eq!("11_0000_7E11_20_0500_0000", data.id_string());

        let data = ldr.read_data().unwrap();

        assert_eq!(true, data.is_none());
    }
}
