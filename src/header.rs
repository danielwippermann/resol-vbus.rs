use std::fmt;
use std::hash::{Hash, Hasher};

use chrono::{DateTime, UTC};

use id_hash::IdHash;

/// All VBus data types consist of a `Header` element.
///
/// Just like the fact that the first 6 bytes of each VBus live byte stream are the same (SYNC to
/// protocol version), the `Header` struct is the common type for all concrete VBus data types.
///
/// In addition to the information stored within the first 6 bytes of the VBus header (destination
/// and source addresses as well as the protocol version), the `Header` type also stores the
/// VBus channel associated with this data as well as the point in time the data was received.
///
/// ## The "identity" of `Header` values
///
/// The fields in the `Header` struct can be separated into two categories:
///
/// 1. Fields that are used to identify the `Header` and (for concrete VBus data types) its payload:
///     - `channel`
///     - `source_address`
///     - `destination_address`
///     - `protocol_version`
/// 2. Fields that are not used to identify the `Header`:
///     - `timestamp`
///
/// Two `Header` values with different `timestamp` fields are considered identical, if all of their
/// other fields match.
///
/// This is also respected by the `id_hash` and `id_string` functions. They return the same result
/// for VBus data values that are considered "identical", allowing some fields to differ.
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
    /// Creates the common identification string prefix for this `Header`.
    ///
    /// The string contains all fields that count towards the "identity" of the `Header`:
    ///
    /// - `channel`
    /// - `destination_address`
    /// - `source_address`
    /// - `protocol_version`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Header};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let header = Header {
    ///     timestamp: utc_timestamp(1485688933),
    ///     channel: 0x11,
    ///     destination_address: 0x1213,
    ///     source_address: 0x1415,
    ///     protocol_version: 0x16,
    /// };
    ///
    /// assert_eq!("11_1213_1415_16", header.id_string());
    /// ```
    pub fn id_string(&self) -> String {
        format!(
            "{:02X}_{:04X}_{:04X}_{:02X}",
            self.channel, self.destination_address, self.source_address, self.protocol_version
        )
    }
}

impl IdHash for Header {
    /// Returns an identification hash for this `Header`.
    ///
    /// The hash contains all fields that count towards the "identity" of the `Header`:
    ///
    /// - `channel`
    /// - `destination_address`
    /// - `source_address`
    /// - `protocol_version`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Header, id_hash};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let header = Header {
    ///     timestamp: utc_timestamp(1485688933),
    ///     channel: 0x11,
    ///     destination_address: 0x1213,
    ///     source_address: 0x1415,
    ///     protocol_version: 0x16,
    /// };
    ///
    /// assert_eq!(8369676560183260683, id_hash(&header));
    /// ```
    fn id_hash<H: Hasher>(&self, h: &mut H) {
        self.channel.hash(h);
        self.destination_address.hash(h);
        self.source_address.hash(h);
        self.protocol_version.hash(h);
    }
}

impl fmt::Debug for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Header")
            .field("timestamp", &self.timestamp)
            .field("channel", &format_args!("0x{:02X}", self.channel))
            .field(
                "destination_address",
                &format_args!("0x{:04X}", self.destination_address),
            )
            .field(
                "source_address",
                &format_args!("0x{:04X}", self.source_address),
            )
            .field(
                "protocol_version",
                &format_args!("0x{:02X}", self.protocol_version),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use utils::utc_timestamp;

    use super::*;

    #[test]
    fn test_debug_fmt() {
        let header = Header {
            timestamp: utc_timestamp(1485688933),
            channel: 0x11,
            destination_address: 0x1213,
            source_address: 0x1415,
            protocol_version: 0x16,
        };

        let result = format!("{:?}", header);

        assert_eq!("Header { timestamp: 2017-01-29T11:22:13Z, channel: 0x11, destination_address: 0x1213, source_address: 0x1415, protocol_version: 0x16 }", result);
    }
}
