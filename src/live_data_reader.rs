use std::io::Read;

use crate::{data::Data, error::Result, live_data_buffer::LiveDataBuffer};

/// Allows reading `Data` variants from a `Read` trait object.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::{FileListReader, LiveDataReader};
///
/// let files: Vec<_> = std::env::args().skip(1).collect();
///
/// let flr = FileListReader::new(files);
///
/// let mut ldr = LiveDataReader::new(0, flr);
///
/// while let Some(data) = ldr.read_data().unwrap() {
///     // process the data
///     println!("{}", data.id_string());
/// }
/// ```
#[derive(Debug)]
pub struct LiveDataReader<R: Read> {
    buf: LiveDataBuffer,
    reader: R,
}

impl<R: Read> LiveDataReader<R> {
    /// Constructs a `LiveDataReader`.
    pub fn new(channel: u8, reader: R) -> LiveDataReader<R> {
        LiveDataReader {
            buf: LiveDataBuffer::new(channel),
            reader,
        }
    }

    fn read_to_buf(&mut self) -> Result<usize> {
        let mut buf = Vec::new();
        buf.resize(4096, 0);

        let size = self.reader.read(&mut buf)?;
        self.buf.extend_from_slice(&buf[0..size]);

        Ok(size)
    }

    /// Read from the stream until a valid blob of data is found.
    pub fn read_bytes(&mut self) -> Result<Option<&[u8]>> {
        let has_bytes = loop {
            if self.buf.peek_length().is_some() {
                break true;
            }

            if self.read_to_buf()? == 0 {
                break false;
            }
        };

        if has_bytes {
            Ok(self.buf.read_bytes())
        } else {
            Ok(None)
        }
    }

    /// Read from the stream until a valid `Data` variant can be decoded.
    pub fn read_data(&mut self) -> Result<Option<Data>> {
        loop {
            if let Some(data) = self.buf.read_data() {
                break Ok(Some(data));
            }

            if self.read_to_buf()? == 0 {
                break Ok(None);
            }
        }
    }
}

impl<R: Read> AsRef<R> for LiveDataReader<R> {
    fn as_ref(&self) -> &R {
        &self.reader
    }
}

impl<R: Read> AsMut<R> for LiveDataReader<R> {
    fn as_mut(&mut self) -> &mut R {
        &mut self.reader
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{test_data::LIVE_DATA_1, test_utils::test_debug_derive};

    #[test]
    fn test_read_bytes() {
        let mut ldr = LiveDataReader::new(0, LIVE_DATA_1);

        for expected_len in [172, 70, 16, 94, 16].iter() {
            let result = ldr.read_bytes().expect("No error").expect("Expected data");
            assert_eq!(*expected_len, result.len());
        }

        let result = ldr.read_bytes().expect("No error");
        assert_eq!(None, result);

        let mut ldr = LiveDataReader::new(0, &LIVE_DATA_1[1..]);

        for expected_len in [70, 16, 94, 16].iter() {
            let result = ldr.read_bytes().expect("No error").expect("Expected data");
            assert_eq!(*expected_len, result.len());
        }

        let result = ldr.read_bytes().expect("No error");
        assert_eq!(None, result);
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

    #[test]
    fn test_derived_impls() {
        let channel = 0x11;

        let ldr = LiveDataReader::new(channel, LIVE_DATA_1);

        test_debug_derive(&ldr);
    }

    #[test]
    fn test_as_ref() {
        let channel = 0x11;

        let ldr = LiveDataReader::new(channel, LIVE_DATA_1);

        let inner = ldr.as_ref();

        assert_eq!(&LIVE_DATA_1, inner);
    }

    #[test]
    fn test_as_mut() {
        let channel = 0x11;

        let mut ldr = LiveDataReader::new(channel, LIVE_DATA_1);

        let inner = ldr.as_mut();

        assert_eq!(&LIVE_DATA_1, inner);
    }
}
