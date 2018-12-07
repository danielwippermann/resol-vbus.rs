use std::io::{Error, ErrorKind, Read, Result, Write};
use std::time::Duration;

use read_with_timeout::ReadWithTimeout;


pub struct Buffer {
    bytes: Vec<u8>,
    read_index: usize,
}


impl Buffer {
    pub fn new() -> Buffer {
        Buffer {
            bytes: Vec::new(),
            read_index: 0,
        }
    }

    pub fn unread_len(&self) -> usize {
        self.bytes.len() - self.read_index
    }

    // pub fn unread_bytes(&self) -> &[u8] {
    //     &self.bytes [self.read_index..]
    // }

    pub fn written_len(&self) -> usize {
        self.bytes.len()
    }

    pub fn written_bytes(&self) -> &[u8] {
        &self.bytes
    }
}


impl Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut bytes = &self.bytes [self.read_index..];
        if bytes.len() > 0 {
            let size = bytes.read(buf)?;
            self.read_index += size;
            Ok(size)
        } else {
            Err(Error::new(ErrorKind::WouldBlock, "Simulated timeout".to_string()))
        }
    }
}


impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.bytes.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.bytes.flush()
    }
}


impl ReadWithTimeout for Buffer {
    fn read_with_timeout(&mut self, buf: &mut [u8], _timeout: Option<Duration>) -> Result<usize> {
        self.read(buf)
    }
}


pub fn to_hex_string(buf: &[u8]) -> String {
    buf.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().concat()
}


#[test]
fn test_buffer() {
    let mut buffer = Buffer::new();

    assert_eq!(0, buffer.unread_len());
    assert_eq!(0, buffer.written_len());

    buffer.write(&[0x01, 0x23, 0x45, 0x67]).unwrap();

    assert_eq!(4, buffer.unread_len());
    assert_eq!(4, buffer.written_len());

    let mut bytes = [0u8; 16];
    let size = buffer.read(&mut bytes).unwrap();

    assert_eq!(4, size);
    assert_eq!(0, buffer.unread_len());
    assert_eq!(4, buffer.written_len());
}

#[test]
fn test_to_hex_string() {
    assert_eq!("", to_hex_string(&[]));
    assert_eq!("01234567", to_hex_string(&[0x01, 0x23, 0x45, 0x67]));
}
