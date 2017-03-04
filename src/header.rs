use std::fmt::{Debug, Error, Formatter};
use std::hash::{Hash, Hasher};

use chrono::{DateTime, UTC};

use id_hash::IdHash;


/// All VBus data types consist of a `Header` element.
#[derive(Clone)]
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
    pub fn id_string(&self) -> String {
        format!("{:02X}_{:04X}_{:04X}_{:02X}", self.channel, self.destination_address, self.source_address, self.protocol_version)
    }

}


impl IdHash for Header {

    fn id_hash<H: Hasher>(&self, h: &mut H) {
        self.channel.hash(h);
        self.destination_address.hash(h);
        self.source_address.hash(h);
        self.protocol_version.hash(h);
    }

}


impl Debug for Header {

    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("Header")
            .field("timestamp", &self.timestamp)
            .field("channel", &format_args!("0x{:02X}", self.channel))
            .field("destination_address", &format_args!("0x{:04X}", self.destination_address))
            .field("source_address", &format_args!("0x{:04X}", self.source_address))
            .field("protocol_version", &format_args!("0x{:02X}", self.protocol_version))
            .finish()
    }

}


#[cfg(test)]
mod tests {
    use chrono::{TimeZone, UTC};

    use super::*;

    #[test]
    fn test_id_string() {
        let timestamp = UTC.timestamp(1485688933, 0);

        let header = Header {
            timestamp: timestamp,
            channel: 0x11,
            destination_address: 0x1213,
            source_address: 0x1415,
            protocol_version: 0x16,
        };

        assert_eq!("11_1213_1415_16", header.id_string());
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
