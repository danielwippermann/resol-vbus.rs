use std::fmt::{Debug, Error, Formatter};

use header::Header;


/// The `Datagram` type stores information according to the VBus protocol version 2.x.
pub struct Datagram {
    /// The shared `Header` of all VBus protocol types.
    header: Header,

    /// The command of this `Datagram`.
    command: u16,

    /// The 16-bit parameter attached to this `Datagram`.
    param16: i16,

    /// The 32-bit parameter attached to this `Datagram`.
    param32: i32,
}


impl Datagram {

    /// Creates an ID string for this `Datagram`.
    pub fn to_id_string(&self) -> String {
        let info = match self.command {
            0x0900 => self.param16,
            _ => 0,
        };
        format!("{}_{:04X}_{:04X}", self.header.to_id_string(), self.command, info)
    }

}


impl Debug for Datagram {

    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.write_fmt(format_args!("Datagram {{ header: {:?}, command: 0x{:04X}, param16: 0x{:04X}, param32: 0x{:08X} ({}) }}", self.header, self.command, self.param16, self.param32, self.param32))
    }

}


#[cfg(test)]
mod tests {
    use chrono::{TimeZone, UTC};

    use header::Header;

    use super::*;

    #[test]
    fn test_to_id_string() {
        let timestamp = UTC.timestamp(1485688933, 0);

        let dgram = Datagram {
            header: Header {
                timestamp: timestamp,
                channel: 0x11,
                destination_address: 0x1213,
                source_address: 0x1415,
                protocol_version: 0x26,
            },
            command: 0x1718,
            param16: 0x191a,
            param32: 0x1b1c1d1e,
        };

        assert_eq!("11_1213_1415_26_1718_0000", dgram.to_id_string());

        let dgram = Datagram {
            header: Header {
                timestamp: timestamp,
                channel: 0x11,
                destination_address: 0x1213,
                source_address: 0x1415,
                protocol_version: 0x26,
            },
            command: 0x0900,
            param16: 0x191a,
            param32: 0x1b1c1d1e,
        };

        assert_eq!("11_1213_1415_26_0900_191A", dgram.to_id_string());
    }

    #[test]
    fn test_debug_fmt() {
        let timestamp = UTC.timestamp(1485688933, 0);

        let dgram = Datagram {
            header: Header {
                timestamp: timestamp,
                channel: 0x11,
                destination_address: 0x1213,
                source_address: 0x1415,
                protocol_version: 0x26,
            },
            command: 0x1718,
            param16: 0x191a,
            param32: 0x1b1c1d1e,
        };

        let result = format!("{:?}", dgram);

        assert_eq!("Datagram { header: Header { timestamp: 2017-01-29T11:22:13Z, channel: 0x11, destination_address: 0x1213, source_address: 0x1415, protocol_version: 0x26 }, command: 0x1718, param16: 0x191A, param32: 0x1B1C1D1E (454827294) }", result);
    }
}
