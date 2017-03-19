use std::fmt::{Debug, Error, Formatter};
use std::hash::{Hash, Hasher};

use id_hash::IdHash;
use header::Header;


/// The `Packet` type stores information according to the VBus protocol version 1.x.
///
/// Packets are used to transmit larger amount of information (up to 508 bytes of payload) relying
/// on the fact that both sides of the communication know how that payload is structured and how to
/// extract the information out of it.
///
/// ## The "identity" of `Packet` values
///
/// As described in [the corresponding section of the `Header` struct][1] VBus data types use
/// some of their fields as part of their "identity". In addition to the fields used by the
/// `Header` type the `Packet` type also respects the `command` field. That means that two `Packet`
/// with differing `timestamp`, `frame_count` and `frame_data` fields are still considered
/// "identical", if the other fields match.
///
/// [1]: struct.Header.html#the-identity-of-header-values
///
/// ## The payload of `Packet` values
///
/// The VBus Protocol Specification describes that all the fields used for the `Packet`'s
/// "identity" can also be used to determine the structure of the payload contained in the
/// `frame_data` field. The [`Specification`][2] type can be used to decode the payload
/// information.
///
/// [2]: struct.Specification.html
pub struct Packet {
    /// The shared `Header` of all VBus protocol types.
    pub header: Header,

    /// The command of this `Packet`.
    pub command: u16,

    /// The number of 4-byte frames attached to this `Packet`.
    pub frame_count: u8,

    /// The actual data from the frames attached to this `Packet`.
    pub frame_data: [u8; 508],
}


impl Packet {

    /// Return the length of the valid area of the `frame_data`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Header, Packet};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let packet = Packet {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x16,
    ///     },
    ///     command: 0x1718,
    ///     frame_count: 0x19,
    ///     frame_data: [0u8; 508],
    /// };
    ///
    /// assert_eq!(100, packet.valid_frame_data_len());
    /// ```
    pub fn valid_frame_data_len(&self) -> usize {
        self.frame_count as usize * 4
    }

    /// Return the valid area of the `frame_data` immutably.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Header, Packet};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let packet = Packet {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x16,
    ///     },
    ///     command: 0x1718,
    ///     frame_count: 0x19,
    ///     frame_data: [0u8; 508],
    /// };
    ///
    /// assert_eq!(508, packet.frame_data.len());
    /// assert_eq!(100, packet.valid_frame_data().len());
    /// ```
    pub fn valid_frame_data(&self) -> &[u8] {
        let end = self.valid_frame_data_len();
        &self.frame_data [0..end]
    }

    /// Return the valid area of the `frame_data` mutably.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Header, Packet};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let mut packet = Packet {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x16,
    ///     },
    ///     command: 0x1718,
    ///     frame_count: 0x19,
    ///     frame_data: [0u8; 508],
    /// };
    ///
    /// assert_eq!(508, packet.frame_data.len());
    /// assert_eq!(100, packet.valid_frame_data_mut().len());
    /// ```
    pub fn valid_frame_data_mut(&mut self) -> &mut [u8] {
        let end = self.valid_frame_data_len();
        &mut self.frame_data [0..end]
    }

    /// Returns a tuple containing identification information about this `Packet`.
    ///
    /// The tuple contains all fields that count towards the "identity" of the `Packet` with the
    /// exception of the `protocol_version` (since it must be 1.0 to be a `Packet` anyway):
    ///
    /// - `channel`
    /// - `destination_address`
    /// - `source_address`
    /// - `command`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Header, Packet};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let packet = Packet {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x16,
    ///     },
    ///     command: 0x1718,
    ///     frame_count: 0x19,
    ///     frame_data: [0u8; 508],
    /// };
    ///
    /// assert_eq!((0x11, 0x1213, 0x1415, 0x1718), packet.packet_id_tuple());
    /// ```
    pub fn packet_id_tuple(&self) -> (u8, u16, u16, u16) {
        (self.header.channel, self.header.destination_address, self.header.source_address, self.command)
    }

    /// Creates an identification string for this `Packet`.
    ///
    /// The string contains all fields that count towards the "identity" of the `Packet`:
    ///
    /// - `channel`
    /// - `destination_address`
    /// - `source_address`
    /// - `protocol_version`
    /// - `command`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Header, Packet};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let packet = Packet {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x16,
    ///     },
    ///     command: 0x1718,
    ///     frame_count: 0x19,
    ///     frame_data: [0u8; 508],
    /// };
    ///
    /// assert_eq!("11_1213_1415_16_1718", packet.id_string());
    /// ```
    pub fn id_string(&self) -> String {
        format!("{}_{:04X}", self.header.id_string(), self.command)
    }

}


impl IdHash for Packet {

    /// Returns an identification hash for this `Packet`.
    ///
    /// The hash contains all fields that count towards the "identity" of the `Packet`:
    ///
    /// - `channel`
    /// - `destination_address`
    /// - `source_address`
    /// - `protocol_version`
    /// - `command`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Header, Packet, id_hash};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let packet = Packet {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x16,
    ///     },
    ///     command: 0x1718,
    ///     frame_count: 0x19,
    ///     frame_data: [0u8; 508],
    /// };
    ///
    /// assert_eq!(2215810099849021132, id_hash(&packet));
    /// ```
    fn id_hash<H: Hasher>(&self, h: &mut H) {
        self.header.id_hash(h);
        self.command.hash(h);
    }

}


impl Debug for Packet {

    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("Packet")
            .field("header", &self.header)
            .field("command", &format_args!("0x{:04X}", self.command))
            .field("frame_count", &format_args!("0x{:02X}", self.frame_count))
            .field("frame_data", &format_args!("..."))
            .finish()
    }

}


impl Clone for Packet {

    fn clone(&self) -> Self {
        let mut frame_data = [0u8; 508];
        frame_data.copy_from_slice(&self.frame_data);

        Packet {
            header: self.header.clone(),
            command: self.command,
            frame_count: self.frame_count,
            frame_data: frame_data,
        }
    }

}


impl AsRef<Header> for Packet {

    fn as_ref(&self) -> &Header {
        &self.header
    }

}


#[cfg(test)]
mod tests {
    use utils::utc_timestamp;
    use header::Header;

    use super::*;

    #[test]
    fn test_debug_fmt() {
        let packet = Packet {
            header: Header {
                timestamp: utc_timestamp(1485688933),
                channel: 0x11,
                destination_address: 0x1213,
                source_address: 0x1415,
                protocol_version: 0x16,
            },
            command: 0x1718,
            frame_count: 0x19,
            frame_data: [0u8; 508],
        };

        let result = format!("{:?}", packet);

        assert_eq!("Packet { header: Header { timestamp: 2017-01-29T11:22:13Z, channel: 0x11, destination_address: 0x1213, source_address: 0x1415, protocol_version: 0x16 }, command: 0x1718, frame_count: 0x19, frame_data: ... }", result);
    }
}
