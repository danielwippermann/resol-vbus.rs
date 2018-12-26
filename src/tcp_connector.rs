use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use error::{Error, Result};

/// The `TcpConnector` wraps a `TcpStream` to perform the handshake according to the
/// "VBus over TCP specification".
///
/// # Examples
///
/// ```rust,no_run
/// use std::net::TcpStream;
///
/// use resol_vbus::{TcpConnector, LiveDataReader};
///
/// // Create a TCP connection to the DL2
/// let stream = TcpStream::connect("192.168.178.101:7053").expect("Unable to connect to DL2");
///
/// // Use a `TcpConnector` to perform the login handshake into the DL2
/// let mut connector = TcpConnector::new(stream);
/// connector.password = "vbus".to_owned();
/// connector.connect().expect("Unable to connect to DL2");
///
/// // Get back the original TCP connection and hand it to a `LiveDataReader`
/// let stream = connector.into_inner();
/// let mut ldr = LiveDataReader::new(0, stream);
///
/// // Read VBus `Data` values from the `LiveDataReader`
/// while let Some(data) = ldr.read_data().expect("Unable to read data") {
///     // do someting with the data
///     println!("{}", data.id_string());
/// }
/// ```
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
            inner,
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

        if line.starts_with('+') {
            Ok(())
        } else if line.starts_with('-') {
            Err(Error::new(format!("Received negative reply: {}", line)))
        } else {
            Err(Error::new(format!("Received unexpected reply: {}", line)))
        }
    }

    fn transceive<R: Read>(&self, r: &mut BufReader<R>, output: &str) -> Result<()> {
        let line = format!("{}\r\n", output);

        write!(&self.inner, "{}", line)?;

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
