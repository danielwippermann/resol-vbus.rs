use std::{
    io::{Read, Result},
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::{
    blob_buffer::BlobBuffer,
    read_with_timeout::ReadWithTimeout,
};


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
///     match length_from_bytes(&br) {
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
    buf: BlobBuffer,
}


impl<R: Read> BlobReader<R> {

    /// Constructs a new `BlobReader<T>`.
    pub fn new(reader: R) -> BlobReader<R> {
        BlobReader {
            reader: reader,
            buf: BlobBuffer::new(),
        }
    }

    /// Consumes this `BlobReader`, returning its inner `Read` value.
    pub fn into_inner(self) -> R {
        self.reader
    }

    /// Reads additional data to the internal buffer.
    pub fn read(&mut self) -> Result<usize> {
        let mut buf = Vec::new();
        buf.resize(4096, 0);

        let size = self.reader.read(&mut buf)?;
        self.buf.extend_from_slice(&buf [0..size]);

        Ok(size)
    }

}

impl<R: Read> Deref for BlobReader<R> {
    type Target = BlobBuffer;

    fn deref(&self) -> &BlobBuffer {
        &self.buf
    }
}

impl<R: Read> DerefMut for BlobReader<R> {
    fn deref_mut(&mut self) -> &mut BlobBuffer {
        &mut self.buf
    }
}

impl<R: ReadWithTimeout + Read> BlobReader<R> {
    /// Reads additional data to the internal buffer using an optional timeout.
    pub fn read_with_timeout(&mut self, timeout: Option<Duration>) -> Result<usize> {
        let mut buf = Vec::new();
        buf.resize(4096, 0);

        let size = self.reader.read_with_timeout(&mut buf, timeout)?;
        self.buf.extend_from_slice(&buf [0..size]);

        Ok(size)
    }
}


#[cfg(test)]
impl<R: Read> AsMut<R> for BlobReader<R> {
    fn as_mut(&mut self) -> &mut R {
        &mut self.reader
    }
}


#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    use test_data::LIVE_DATA_1;
    use test_utils::Buffer;

    #[test]
    fn test_new() {
        let bytes = LIVE_DATA_1;

        let br = BlobReader::new(bytes);

        assert_eq!(0, br.buf.len());
    }

    #[test]
    fn test_read() {
        let bytes = LIVE_DATA_1;
        let len = bytes.len();

        let mut br = BlobReader::new(bytes);

        let result = br.read().unwrap();
        assert_eq!(len, result);
        assert_eq!(len, br.buf.len());

        let result = br.read().unwrap();
        assert_eq!(0, result);
        assert_eq!(len, br.buf.len());
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

        br.consume(10);
        assert_eq!(len - 10, br.buf.len());

        br.consume(10);
        assert_eq!(len - 20, br.buf.len());

        let result = br.read().unwrap();
        assert_eq!(0, result);
        assert_eq!(len - 20, br.buf.len());
    }

    #[test]
    fn test_read_with_timeout() {
        let timeout = Some(Duration::from_millis(1));

        let mut br = BlobReader::new(Buffer::new());

        assert_eq!(0, br.len());

        assert_eq!(true, br.read_with_timeout(timeout).is_err());

        assert_eq!(0, br.len());

        br.as_mut().write(&LIVE_DATA_1 [0..172]).unwrap();

        assert_eq!(172, br.read_with_timeout(timeout).unwrap());
        assert_eq!(172, br.len());

        br.as_mut().write(&LIVE_DATA_1 [172..232]).unwrap();

        assert_eq!(60, br.read_with_timeout(timeout).unwrap());
        assert_eq!(232, br.len());

        br.as_mut().write(&LIVE_DATA_1 [232..242]).unwrap();

        assert_eq!(10, br.read_with_timeout(timeout).unwrap());
        assert_eq!(242, br.len());
    }
}
