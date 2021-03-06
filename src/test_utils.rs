#![allow(dead_code)]

use std::io::{Read, Result, Write};

pub struct Buffer {
    bytes: Vec<u8>,
    read_index: usize,
    read_call_count: usize,
    write_call_count: usize,
    is_eof: bool,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer {
            bytes: Vec::new(),
            read_index: 0,
            read_call_count: 0,
            write_call_count: 0,
            is_eof: false,
        }
    }

    pub fn reset(&mut self) {
        self.bytes.clear();
        self.read_index = 0;
        self.read_call_count = 0;
        self.write_call_count = 0;
        self.is_eof = false;
    }

    pub fn set_eof(&mut self) {
        self.is_eof = true;
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

    pub fn read_call_count(&self) -> usize {
        self.read_call_count
    }

    pub fn write_call_count(&self) -> usize {
        self.write_call_count
    }
}

impl Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let len = (&self.bytes[self.read_index..]).read(buf)?;
        self.read_index += len;
        Ok(len)
    }
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.write_call_count += 1;

        self.bytes.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.bytes.flush()
    }
}

pub fn to_hex_string(buf: &[u8]) -> String {
    buf.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .concat()
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

    buffer.set_eof();

    let size = buffer.read(&mut bytes).unwrap();

    assert_eq!(size, 0);
}

#[test]
fn test_to_hex_string() {
    assert_eq!("", to_hex_string(&[]));
    assert_eq!("01234567", to_hex_string(&[0x01, 0x23, 0x45, 0x67]));
}
