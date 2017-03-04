use std::collections::hash_map::DefaultHasher;
use std::fmt::{Debug, Error, Formatter};
use std::hash::{Hash, Hasher};

use chrono::{DateTime, UTC};


/// A trait to generate an identification hash for any of the VBus data types.
pub trait IdHash {
    /// Creates an ID hash for this `Header`.
    fn id_hash<H: Hasher>(&self, h: &mut H);
}


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

    /// Creates an ID hash for this `Header`.
    fn id_hash<H: Hasher>(&self, h: &mut H) {
        self.channel.hash(h);
        self.destination_address.hash(h);
        self.source_address.hash(h);
        self.protocol_version.hash(h);
    }

}


/// Calculate the ID hash for a given VBus `Data` value.
pub fn id_hash<H: IdHash>(h: &H) -> u64 {
    let mut hasher = DefaultHasher::new();
    h.id_hash(&mut hasher);
    hasher.finish()
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

    #[test]
    fn test_id_hash() {
        let timestamp = UTC.timestamp(1485688933, 0);

        let header = Header {
            timestamp: timestamp,
            channel: 0x11,
            destination_address: 0x1213,
            source_address: 0x1415,
            protocol_version: 0x16,
        };

        let result = id_hash(&header);

        assert_eq!(8369676560183260683, result);
    }
}
