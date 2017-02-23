use std::net::TcpStream;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Result, Write};


/// The `TcpConnector` wraps a `TcpStream` to perform the handshake according to the
/// "VBus over TCP specification".
#[derive(Debug)]
pub struct TcpConnector {
    inner: TcpStream,

    /// An optional via tag used to connect to a device over VBus.net.
    pub via_tag: Option<String>,

    /// A password string used to connect to the device.
    pub password: String,

    /// An optional channel number used to connect to a DL3.
    pub channel: Option<u8>,
}


impl TcpConnector {

    /// Constructs a new `TcpConnector` using the given `TcpStream`.
    pub fn new(inner: TcpStream) -> TcpConnector {
        TcpConnector {
            inner: inner,
            via_tag: None,
            channel: None,
            password: "vbus".to_owned(),
        }
    }

    /// Consumes the `TcpConnector` and returns the inner `TcpStream`.
    pub fn into_inner(self) -> TcpStream {
        self.inner
    }

    /// Perform the handshake according to the "VBus over TCP specification".
    pub fn connect(&self) -> Result<()> {
        let mut r = BufReader::new(&self.inner);

        self.read_response(&mut r)?;

        if let Some(ref via_tag) = self.via_tag {
            self.transceive(&mut r, &format!("CONNECT {}", via_tag))?;
        }

        self.transceive(&mut r, &format!("PASS {}", self.password))?;

        if let Some(channel) = self.channel {
            self.transceive(&mut r, &format!("CHANNEL {}", channel))?;
        }

        self.transceive(&mut r, "DATA")?;

        Ok(())
    }

    fn read_response<R: Read>(&self, r: &mut BufReader<R>) -> Result<()> {
        let mut line = String::new();

        r.read_line(&mut line)?;
        // println!("Response: {:?}", line);

        if line.starts_with('+') {
            Ok(())
        } else if line.starts_with('-') {
            Err(Error::new(ErrorKind::Other, line))
        } else {
            Err(Error::new(ErrorKind::Other, line))
        }
    }

    fn transceive<R: Read>(&self, r: &mut BufReader<R>, output: &str) -> Result<()> {
        // println!("Request: {:?}", output);

        write!(&self.inner, "{}\r\n", output)?;

        self.read_response(r)
    }

}


#[cfg(test)]
mod tests {
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    use super::*;

    #[test]
    fn test_connect() {
        let listener = TcpListener::bind("127.0.0.1:7053").unwrap();

        let addr = listener.local_addr().unwrap();

        let t = thread::spawn(move || {
            let stream = TcpStream::connect(addr).unwrap();

            let mut connector = TcpConnector::new(stream);
            connector.via_tag = Some("via_tag".to_owned());
            connector.channel = Some(0x11);

            connector.connect().unwrap();
        });

        let (stream, _) = listener.accept().unwrap();

        let mut r = BufReader::new(&stream);

        let mut line = String::new();

        write!(&stream, "+HELLO\r\n").unwrap();

        r.read_line(&mut line).unwrap();
        assert_eq!("CONNECT via_tag\r\n", line);
        line.clear();

        write!(&stream, "+OK\r\n").unwrap();

        r.read_line(&mut line).unwrap();
        assert_eq!("PASS vbus\r\n", line);
        line.clear();

        write!(&stream, "+OK\r\n").unwrap();

        r.read_line(&mut line).unwrap();
        assert_eq!("CHANNEL 17\r\n", line);
        line.clear();

        write!(&stream, "+OK\r\n").unwrap();

        r.read_line(&mut line).unwrap();
        assert_eq!("DATA\r\n", line);
        line.clear();

        write!(&stream, "+OK\r\n").unwrap();

        t.join().unwrap();
    }
}
