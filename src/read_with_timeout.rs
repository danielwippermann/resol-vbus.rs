use std::io::{Read, Result};
use std::net::TcpStream;
use std::time::Duration;

/// A trait to support reading using a timeout.
pub trait ReadWithTimeout {
    /// Reads data using an optional timeout.
    fn read_with_timeout(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> Result<usize>;
}

impl ReadWithTimeout for TcpStream {
    fn read_with_timeout(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> Result<usize> {
        self.set_read_timeout(timeout)?;
        self.read(buf)
    }
}
