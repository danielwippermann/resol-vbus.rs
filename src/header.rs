use std::fmt::{Debug, Error, Formatter};

use chrono::{DateTime, UTC};


/// All VBus data types consist of a `Header` element.
pub struct Header {
    /// The timestamp when this `Header` was received.
    pub timestamp: DateTime<UTC>,

    /// The channel number on which this `Header` was received.
    pub channel: u8,

    /// The destination address of this `Header`.
    pub destination_address: u16,

    /// The source address of this `Header`.
    pub source_address: u16,

    /// The VBus protocol version of this `Header`.
    pub protocol_version: u8,
}


impl Header {

    /// Creates an ID prefix for this `Header`.
    pub fn to_id_string(&self) -> String {
        format!("{:02X}_{:04X}_{:04X}_{:02X}", self.channel, self.destination_address, self.source_address, self.protocol_version)
    }

}


impl Debug for Header {

    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.write_fmt(format_args!("Header {{ timestamp: {:?}, channel: 0x{:02X}, destination_address: 0x{:04X}, source_address: 0x{:04X}, protocol_version: 0x{:02X} }}", self.timestamp, self.channel, self.destination_address, self.source_address, self.protocol_version))
    }

}


#[cfg(test)]
mod tests {
    use chrono::{TimeZone, UTC};

    use super::*;

    #[test]
    fn test_to_id_string() {
        let timestamp = UTC.timestamp(1485688933, 0);

        let header = Header {
            timestamp: timestamp,
            channel: 0x11,
            destination_address: 0x1213,
            source_address: 0x1415,
            protocol_version: 0x16,
        };

        assert_eq!("11_1213_1415_16", header.to_id_string());
    }

    #[test]
    fn test_debug_fmt() {
        let timestamp = UTC.timestamp(1485688933, 0);

        let header = Header {
            timestamp: timestamp,
            channel: 0x11,
            destination_address: 0x1213,
            source_address: 0x1415,
            protocol_version: 0x16,
        };

        let result = format!("{:?}", header);

        assert_eq!("Header { timestamp: 2017-01-29T11:22:13Z, channel: 0x11, destination_address: 0x1213, source_address: 0x1415, protocol_version: 0x16 }", result);
    }
}
