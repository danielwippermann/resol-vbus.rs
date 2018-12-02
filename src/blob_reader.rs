use std::io::{Read, Result};
use std::time::Duration;


use read_with_timeout::ReadWithTimeout;


/// A buffering reader that allows to borrow the internal buffer.
///
/// The `BlobReader` behaves like a `std::io::BufReader` with the addition that the internal buffer
/// grows if necessary.
///
/// # Examples
///
/// ```rust,no_run
/// use std::fs::File;
///
/// use resol_vbus::{BlobReader, StreamBlobLength};
/// use resol_vbus::recording_decoder::{length_from_bytes};
///
/// let file = File::open("20161202_packets.vbus").unwrap();
/// let mut br = BlobReader::new(file);
///
/// loop {
///     match length_from_bytes(br.as_bytes()) {
///         StreamBlobLength::BlobLength(size) => {
///             // do something with the data
///
///             // afterwards consume it
///             br.consume(size);
///         }
///         StreamBlobLength::Malformed => {
///             // just consume the current starting byte, perhaps a valid blob is hidden behind it
///             br.consume(1);
///         }
///         StreamBlobLength::Partial => {
///             // internal buffer is either empty or contains the valid start of a blob, read more
///             // data
///             if br.read().unwrap() == 0 {
///                 break;
///             }
///         }
///     }
/// }
/// ```
#[derive(Debug)]
pub struct BlobReader<R: Read> {
    reader: R,
    buf: Vec<u8>,
    start: usize,
    offset: usize,
}


impl<R: Read> BlobReader<R> {

    /// Constructs a new `BlobReader<T>`.
    pub fn new(reader: R) -> BlobReader<R> {
        BlobReader {
            reader: reader,
            buf: Vec::new(),
            start: 0,
            offset: 0,
        }
    }

    /// Consumes this `BlobReader`, returning its inner `Read` value.
    pub fn into_inner(self) -> R {
        self.reader
    }

    /// Reads additional data to the internal buffer.
    pub fn read(&mut self) -> Result<usize> {
        if self.start > 0 {
            drop(self.buf.drain(0..self.start));
            self.start = 0;
        }

        let end = self.buf.len();
        self.buf.resize(end + 4096, 0);

        let result = self.reader.read(&mut self.buf [end..])?;
        self.buf.resize(end + result, 0);

        Ok(result)
    }

    /// Consume the given amount of data from the internal buffer.
    pub fn consume(&mut self, length: usize) {
        self.start += length;
        self.offset += length;
    }

    /// Returns the unconsumed byte slice of the internal buffer.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[self.start..]
    }

    /// Get amount of already consumed bytes.
    pub fn offset(&self) -> usize {
        self.offset
    }

}


impl<R: ReadWithTimeout + Read> BlobReader<R> {
    /// Reads additional data to the internal buffer using an optional timeout.
    pub fn read_with_timeout(&mut self, timeout: Option<Duration>) -> Result<usize> {
        if self.start > 0 {
            drop(self.buf.drain(0..self.start));
            self.start = 0;
        }

        let end = self.buf.len();
        self.buf.resize(end + 4096, 0);

        let result = self.reader.read_with_timeout(&mut self.buf [end..], timeout)?;
        self.buf.resize(end + result, 0);

        Ok(result)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use test_data::LIVE_DATA_1;

    #[test]
    fn test_new() {
        let bytes = LIVE_DATA_1;

        let br = BlobReader::new(bytes);

        assert_eq!(0, br.buf.len());
        assert_eq!(0, br.start);
    }

    #[test]
    fn test_read() {
        let bytes = LIVE_DATA_1;
        let len = bytes.len();

        let mut br = BlobReader::new(bytes);

        let result = br.read().unwrap();
        assert_eq!(len, result);
        assert_eq!(len, br.buf.len());
        assert_eq!(0, br.start);

        let result = br.read().unwrap();
        assert_eq!(0, result);
        assert_eq!(len, br.buf.len());
        assert_eq!(0, br.start);
    }

    #[test]
    fn test_consume() {
        let bytes = LIVE_DATA_1;
        let len = bytes.len();
        assert!(len > 20);

        let mut br = BlobReader::new(bytes);

        let result = br.read().unwrap();
        assert_eq!(len, result);
        assert_eq!(len, br.buf.len());
        assert_eq!(0, br.start);

        br.consume(10);
        assert_eq!(len, result);
        assert_eq!(len, br.buf.len());
        assert_eq!(10, br.start);

        br.consume(10);
        assert_eq!(len, result);
        assert_eq!(len, br.buf.len());
        assert_eq!(20, br.start);

        let result = br.read().unwrap();
        assert_eq!(0, result);
        assert_eq!(len - 20, br.buf.len());
        assert_eq!(0, br.start);
    }

    #[test]
    fn test_as_bytes() {
        let bytes = LIVE_DATA_1;
        let len = bytes.len();
        assert!(len > 20);

        let mut br = BlobReader::new(bytes);

        {
            let br_bytes = br.as_bytes();
            assert_eq!(0, br_bytes.len());
        }

        let result = br.read().unwrap();

        {
            let br_bytes = br.as_bytes();
            assert_eq!(len, br_bytes.len());
        }

        br.consume(10);
        assert_eq!(len, result);
        assert_eq!(len, br.buf.len());
        assert_eq!(10, br.start);

        {
            let br_bytes = br.as_bytes();
            assert_eq!(len - 10, br_bytes.len());
            assert_eq!(&bytes [10..], br_bytes);
        }

        br.consume(10);
        assert_eq!(len, result);
        assert_eq!(len, br.buf.len());
        assert_eq!(20, br.start);

        {
            let br_bytes = br.as_bytes();
            assert_eq!(len - 20, br_bytes.len());
            assert_eq!(&bytes [20..], br_bytes);
        }

        let result = br.read().unwrap();
        assert_eq!(0, result);
        assert_eq!(len - 20, br.buf.len());
        assert_eq!(0, br.start);
    }
}
