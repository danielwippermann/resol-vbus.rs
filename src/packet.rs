use std::fmt::{Debug, Error, Formatter};
use std::hash::{Hash, Hasher};

use id_hash::IdHash;
use header::Header;


/// A tuple of identification information about a `Packet` value.
///
/// It consists of the following parts:
///
/// - the channel
/// - the destination address
/// - the source address
/// - the command
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PacketId(pub u8, pub u16, pub u16, pub u16);


impl PacketId {

    /// Create an ID string for the given `PacketId` value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::PacketId;
    ///
    /// assert_eq!("11_1213_1415_10_1718", PacketId(0x11, 0x1213, 0x1415, 0x1718).packet_id_string());
    /// ```
    pub fn packet_id_string(&self) -> String {
        format!("{:02X}_{:04X}_{:04X}_10_{:04X}", self.0, self.1, self.2, self.3)
    }

}


/// A trait to get a `PacketId` for a given value.
pub trait ToPacketId {

    /// Get the `PacketId` for a given value.
    fn to_packet_id(&self) -> Result<PacketId, String>;

}


impl ToPacketId for PacketId {

    fn to_packet_id(&self) -> Result<PacketId, String> {
        Ok(*self)
    }

}


impl ToPacketId for str {

    /// Parse the string into a packet ID tuple.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use resol_vbus::{PacketId, ToPacketId};
    ///
    /// assert_eq!(PacketId(0x11, 0x1213, 0x1415, 0x1718), "11_1213_1415_10_1718".to_packet_id().unwrap());
    /// ```
    fn to_packet_id(&self) -> Result<PacketId, String> {
        let is_not_hex_char = |c| {
            match c {
                '0'...'9' | 'A'...'F' | 'a'...'f' => false,
                _ => true,
            }
        };

        if self.len() < 20 {
            return Err(format!("Invalid length of input {:?}", self));
        }

        let mut parts = self.split('_');

        let channel_str = parts.next().unwrap();
        if channel_str.len() != 2 {
            return Err(format!("Invalid length of channel {:?}", channel_str));
        }
        if channel_str.chars().any(&is_not_hex_char) {
            return Err(format!("Invalid characters in channel {:?}", channel_str));
        }
        let channel = u8::from_str_radix(channel_str, 16).unwrap();

        let destination_address_str = parts.next().unwrap();
        if destination_address_str.len() != 4 {
            return Err(format!("Invalid length of destination address {:?}", destination_address_str));
        }
        if destination_address_str.chars().any(&is_not_hex_char) {
            return Err(format!("Invalid characters in destination address {:?}", destination_address_str));
        }
        let destination_address = u16::from_str_radix(destination_address_str, 16).unwrap();

        let source_address_str = parts.next().unwrap();
        if source_address_str.len() != 4 {
            return Err(format!("Invalid length of source address {:?}", source_address_str));
        }
        if source_address_str.chars().any(&is_not_hex_char) {
            return Err(format!("Invalid characters in source address {:?}", source_address_str));
        }
        let source_address = u16::from_str_radix(source_address_str, 16).unwrap();

        let protocol_version_str = parts.next().unwrap();
        if protocol_version_str.len() != 2 {
            return Err(format!("Invalid length of protocol version {:?}", protocol_version_str));
        }
        if protocol_version_str.chars().any(&is_not_hex_char) {
            return Err(format!("Invalid characters in protocol version {:?}", protocol_version_str));
        }
        let protocol_version = u8::from_str_radix(protocol_version_str, 16).unwrap();
        if (protocol_version & 0xF0) != 0x10 {
            return Err(format!("Unsupported protocol version 0x{:02X}", protocol_version));
        }

        let command_str = parts.next().unwrap();
        if command_str.len() != 4 {
            return Err(format!("Invalid length of command {:?}", command_str));
        }
        if command_str.chars().any(&is_not_hex_char) {
            return Err(format!("Invalid characters in command {:?}", command_str));
        }
        let command = u16::from_str_radix(command_str, 16).unwrap();

        Ok(PacketId(channel, destination_address, source_address, command))
    }

}


/// A tuple of identification information about a field in a `Packet` value.
///
/// It consists of the following parts:
///
/// - the packet ID tuple (channel, destination address, source address and command)
/// - the field ID
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PacketFieldId<'a>(pub PacketId, pub &'a str);


impl<'a> PacketFieldId<'a> {

    /// Get the packet ID string for a given `PacketFieldId` value.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use resol_vbus::{PacketId, PacketFieldId};
    ///
    /// let packet_field_id = PacketFieldId(PacketId(0x11, 0x1213, 0x1415, 0x1718), "012_4_0");
    /// assert_eq!("11_1213_1415_10_1718", packet_field_id.packet_id_string());
    /// ```
    pub fn packet_id_string(&self) -> String {
        self.0.packet_id_string()
    }

    /// Get the packet field ID string for a given `PacketFieldId` value.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use resol_vbus::{PacketId, PacketFieldId};
    ///
    /// let packet_field_id = PacketFieldId(PacketId(0x11, 0x1213, 0x1415, 0x1718), "012_4_0");
    /// assert_eq!("11_1213_1415_10_1718_012_4_0", packet_field_id.packet_field_id_string());
    /// ```
    pub fn packet_field_id_string(&self) -> String {
        format!("{}_{}", self.packet_id_string(), self.1)
    }

}


/// A trait to get a `PacketFieldId` for a given value.
pub trait ToPacketFieldId {

    /// Get the `PacketFieldId` for a given value.
    fn to_packet_field_id(&self) -> Result<PacketFieldId, String>;

}


impl<'a> ToPacketFieldId for PacketFieldId<'a> {

    fn to_packet_field_id(&self) -> Result<PacketFieldId, String> {
        Ok(*self)
    }

}


impl ToPacketFieldId for str {

    /// Parse the string into a packet field ID tuple.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use resol_vbus::{PacketId, PacketFieldId, ToPacketFieldId};
    ///
    /// assert_eq!(PacketFieldId(PacketId(0x11, 0x1213, 0x1415, 0x1718), "012_4_0"), "11_1213_1415_10_1718_012_4_0".to_packet_field_id().unwrap());
    /// ```
    fn to_packet_field_id(&self) -> Result<PacketFieldId, String> {
        if self.len() < 21 {
            return Err(format!("Invalid length of input {:?}", self));
        }

        let packet_id = self.to_packet_id()?;

        let field_id = &self [21..];

        Ok(PacketFieldId(packet_id, field_id))
    }

}


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

    /// Returns identification information about this `Packet`.
    ///
    /// The result contains all fields that count towards the "identity" of the `Packet` with the
    /// exception of the `protocol_version` (since it must be 1.x to be a `Packet` anyway):
    ///
    /// - `channel`
    /// - `destination_address`
    /// - `source_address`
    /// - `command`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Header, Packet, PacketId};
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
    /// assert_eq!(PacketId(0x11, 0x1213, 0x1415, 0x1718), packet.packet_id());
    /// ```
    pub fn packet_id(&self) -> PacketId {
        PacketId(self.header.channel, self.header.destination_address, self.header.source_address, self.command)
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


impl ToPacketId for Packet {

    fn to_packet_id(&self) -> Result<PacketId, String> {
        Ok(self.packet_id())
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
