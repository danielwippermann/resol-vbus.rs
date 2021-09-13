use crate::{
    blob_buffer::BlobBuffer,
    data::Data,
    live_data_decoder::{data_from_checked_bytes, length_from_bytes},
    stream_blob_length::StreamBlobLength::{BlobLength, Malformed, Partial},
    utils::current_timestamp,
};

/// A size-adapting buffer that supports decoding VBus live data. See
/// [`BlobBuffer`](struct.BlobBuffer.html) for details.
#[derive(Debug)]
pub struct LiveDataBuffer {
    channel: u8,
    buf: BlobBuffer,
    previous_length: usize,
}

impl LiveDataBuffer {
    /// Constructs a `LiveDataReader`.
    pub fn new(channel: u8) -> LiveDataBuffer {
        LiveDataBuffer {
            channel,
            buf: BlobBuffer::new(),
            previous_length: 0,
        }
    }

    /// Write bytes to the internal buffer.
    pub fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Try to peek length of valid blob in internal buffer.
    pub fn peek_length(&mut self) -> Option<usize> {
        if self.previous_length > 0 {
            self.buf.consume(self.previous_length);
            self.previous_length = 0;
        }

        loop {
            match length_from_bytes(&self.buf) {
                BlobLength(length) => {
                    break Some(length);
                }
                Partial => {
                    break None;
                }
                Malformed => {
                    self.buf.consume(1);
                }
            }
        }
    }

    /// Try to read a valid blob of bytes from internal buffer.
    pub fn read_bytes(&mut self) -> Option<&[u8]> {
        match self.peek_length() {
            Some(length) => {
                self.previous_length = length;
                Some(&self.buf[0..length])
            }
            None => None,
        }
    }

    /// Try to read a valid blob of bytes as `Data` from internal buffer.
    pub fn read_data(&mut self) -> Option<Data> {
        let channel = self.channel;
        self.read_bytes()
            .map(|bytes| data_from_checked_bytes(current_timestamp(), channel, bytes))
    }

    /// Get amount of already read bytes.
    pub fn offset(&self) -> usize {
        self.buf.offset()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_data::LIVE_DATA_1;

    #[test]
    fn test_peek_length() {
        let mut ldb = LiveDataBuffer::new(0);
        ldb.extend_from_slice(LIVE_DATA_1);

        assert_eq!(Some(172), ldb.peek_length());
        assert_eq!(Some(172), ldb.peek_length());
    }

    #[test]
    fn test_read_bytes() {
        let mut ldb = LiveDataBuffer::new(0);
        ldb.extend_from_slice(LIVE_DATA_1);

        for expected_len in [172, 70, 16, 94, 16].iter() {
            let actual_len = ldb.read_bytes().expect("Expected to return bytes").len();
            assert_eq!(*expected_len, actual_len);
        }

        assert_eq!(None, ldb.read_bytes());

        let mut ldb = LiveDataBuffer::new(0);
        ldb.extend_from_slice(&LIVE_DATA_1[1..]);

        for expected_len in [70, 16, 94, 16].iter() {
            let actual_len = ldb.read_bytes().expect("Expected to return bytes").len();
            assert_eq!(*expected_len, actual_len);
        }

        assert_eq!(None, ldb.read_bytes());
    }

    #[test]
    fn test_read_data() {
        let mut ldb = LiveDataBuffer::new(0x11);
        ldb.extend_from_slice(LIVE_DATA_1);

        let data = ldb.read_data().expect("Expected data");

        assert_eq!("11_0010_7E11_10_0100", data.id_string());

        let data = ldb.read_data().expect("Expected data");

        assert_eq!("11_0015_7E11_10_0100", data.id_string());

        let data = ldb.read_data().expect("Expected data");

        assert_eq!("11_0010_7E22_10_0100", data.id_string());

        let data = ldb.read_data().expect("Expected data");

        assert_eq!("11_6651_7E11_10_0200", data.id_string());

        let data = ldb.read_data().expect("Expected data");

        assert_eq!("11_0000_7E11_20_0500_0000", data.id_string());

        assert_eq!(None, ldb.read_data());
    }
}
