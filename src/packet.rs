use std::{
    fmt,
    hash::{Hash, Hasher},
};

use crate::{error::Result, header::Header, id_hash::IdHash};

/// A tuple of identification information about a `Packet` value.
///
/// It consists of the following parts:
///
/// - the channel
/// - the destination address
/// - the source address
/// - the command
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        format!(
            "{:02X}_{:04X}_{:04X}_10_{:04X}",
            self.0, self.1, self.2, self.3
        )
    }
}

/// A trait to get a `PacketId` for a given value.
pub trait ToPacketId {
    /// Get the `PacketId` for a given value.
    fn to_packet_id(&self) -> Result<PacketId>;
}

impl ToPacketId for PacketId {
    fn to_packet_id(&self) -> Result<PacketId> {
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
    fn to_packet_id(&self) -> Result<PacketId> {
        let is_not_hex_char = |c: char| !c.is_ascii_hexdigit();

        if self.len() < 20 {
            return Err(format!("Invalid length of input {self:?}").into());
        }

        let mut parts = self.split('_');

        let channel_str = parts.next().unwrap();
        if channel_str.len() != 2 {
            return Err(format!("Invalid length of channel {channel_str:?}").into());
        }
        if channel_str.chars().any(&is_not_hex_char) {
            return Err(format!("Invalid characters in channel {channel_str:?}").into());
        }
        let channel = u8::from_str_radix(channel_str, 16).unwrap();

        let destination_address_str = parts.next().unwrap();
        if destination_address_str.len() != 4 {
            return Err(format!(
                "Invalid length of destination address {destination_address_str:?}",
            )
            .into());
        }
        if destination_address_str.chars().any(&is_not_hex_char) {
            return Err(format!(
                "Invalid characters in destination address {destination_address_str:?}"
            )
            .into());
        }
        let destination_address = u16::from_str_radix(destination_address_str, 16).unwrap();

        let source_address_str = parts.next().unwrap();
        if source_address_str.len() != 4 {
            return Err(format!("Invalid length of source address {source_address_str:?}").into());
        }
        if source_address_str.chars().any(&is_not_hex_char) {
            return Err(
                format!("Invalid characters in source address {source_address_str:?}").into(),
            );
        }
        let source_address = u16::from_str_radix(source_address_str, 16).unwrap();

        let protocol_version_str = parts.next().unwrap();
        if protocol_version_str.len() != 2 {
            return Err(
                format!("Invalid length of protocol version {protocol_version_str:?}").into(),
            );
        }
        if protocol_version_str.chars().any(&is_not_hex_char) {
            return Err(
                format!("Invalid characters in protocol version {protocol_version_str:?}").into(),
            );
        }
        let protocol_version = u8::from_str_radix(protocol_version_str, 16).unwrap();
        if (protocol_version & 0xF0) != 0x10 {
            return Err(format!("Unsupported protocol version 0x{protocol_version:02X}").into());
        }

        let command_str = parts.next().unwrap();
        if command_str.len() != 4 {
            return Err(format!("Invalid length of command {command_str:?}").into());
        }
        if command_str.chars().any(&is_not_hex_char) {
            return Err(format!("Invalid characters in command {command_str:?}").into());
        }
        let command = u16::from_str_radix(command_str, 16).unwrap();

        Ok(PacketId(
            channel,
            destination_address,
            source_address,
            command,
        ))
    }
}

/// A tuple of identification information about a field in a `Packet` value.
///
/// It consists of the following parts:
///
/// - the packet ID tuple (channel, destination address, source address and command)
/// - the field ID
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    fn to_packet_field_id(&self) -> Result<PacketFieldId<'_>>;
}

impl<'a> ToPacketFieldId for PacketFieldId<'a> {
    fn to_packet_field_id(&self) -> Result<PacketFieldId<'_>> {
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
    fn to_packet_field_id(&self) -> Result<PacketFieldId<'_>> {
        if self.len() < 21 {
            return Err(format!("Invalid length of input {self:?}").into());
        }

        let packet_id = self.to_packet_id()?;

        let field_id = &self[21..];

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
        &self.frame_data[0..end]
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
        &mut self.frame_data[0..end]
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
        PacketId(
            self.header.channel,
            self.header.destination_address,
            self.header.source_address,
            self.command,
        )
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
    fn to_packet_id(&self) -> Result<PacketId> {
        Ok(self.packet_id())
    }
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            frame_data,
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
    use super::*;

    use crate::{
        error::Error,
        test_utils::{
            test_clone_derive, test_copy_derive, test_debug_derive, test_eq_derive,
            test_hash_derive, test_ord_derive, test_partial_eq_derive, test_partial_ord_derive,
        },
        utils::utc_timestamp,
    };

    #[test]
    fn test_packet_id_derived_impls() {
        let packet_id = PacketId(0x11, 0x1213, 0x1415, 0x1718);
        test_debug_derive(&packet_id);
        test_clone_derive(&packet_id);
        test_copy_derive(&packet_id);
        test_partial_eq_derive(&packet_id);
        test_eq_derive(&packet_id);
        test_partial_ord_derive(&packet_id);
        test_ord_derive(&packet_id);
        test_hash_derive(&packet_id);
    }

    #[test]
    fn test_packet_field_id_derived_impls() {
        let packet_field_id = PacketFieldId(PacketId(0x11, 0x1213, 0x1415, 0x1718), "000_2_0");
        test_debug_derive(&packet_field_id);
        test_clone_derive(&packet_field_id);
        test_copy_derive(&packet_field_id);
        test_partial_eq_derive(&packet_field_id);
        test_eq_derive(&packet_field_id);
        test_hash_derive(&packet_field_id);
    }

    #[test]
    fn test_packet_id_string() {
        assert_eq!(
            "11_1213_1415_10_1718",
            PacketId(0x11, 0x1213, 0x1415, 0x1718).packet_id_string()
        );
    }

    #[test]
    fn test_packet_id_to_packet_id() {
        let packet_id = PacketId(0x11, 0x1213, 0x1415, 0x1718);

        let result = packet_id.to_packet_id().expect("Must not fail");

        assert_eq!(packet_id, result);
    }

    #[test]
    fn test_str_to_packet_id() {
        assert_eq!(
            PacketId(0x11, 0x1213, 0x1415, 0x1718),
            "11_1213_1415_10_1718".to_packet_id().unwrap()
        );
        assert_eq!(
            PacketId(0x11, 0x1213, 0x1415, 0x1718),
            "11_1213_1415_10_1718_XXX_X_X".to_packet_id().unwrap()
        );
        assert_eq!(
            Error::new("Invalid length of input \"11_1213_1415_10_171\""),
            "11_1213_1415_10_171".to_packet_id().unwrap_err()
        );
        assert_eq!(
            Error::new("Invalid length of channel \"111\""),
            "111_1213_1415_10_1718".to_packet_id().unwrap_err()
        );
        assert_eq!(
            Error::new("Invalid characters in channel \"1G\""),
            "1G_1213_1415_10_1718".to_packet_id().unwrap_err()
        );
        assert_eq!(
            Error::new("Invalid length of destination address \"12131\""),
            "11_12131_1415_10_1718".to_packet_id().unwrap_err()
        );
        assert_eq!(
            Error::new("Invalid characters in destination address \"121G\""),
            "11_121G_1415_10_1718".to_packet_id().unwrap_err()
        );
        assert_eq!(
            Error::new("Invalid length of source address \"14151\""),
            "11_1213_14151_10_1718".to_packet_id().unwrap_err()
        );
        assert_eq!(
            Error::new("Invalid characters in source address \"141G\""),
            "11_1213_141G_10_1718".to_packet_id().unwrap_err()
        );
        assert_eq!(
            Error::new("Invalid length of protocol version \"101\""),
            "11_1213_1415_101_1718".to_packet_id().unwrap_err()
        );
        assert_eq!(
            Error::new("Invalid characters in protocol version \"1G\""),
            "11_1213_1415_1G_1718".to_packet_id().unwrap_err()
        );
        assert_eq!(
            Error::new("Unsupported protocol version 0x20"),
            "11_1213_1415_20_1718".to_packet_id().unwrap_err()
        );
        assert_eq!(
            Error::new("Invalid length of command \"17181\""),
            "11_1213_1415_10_17181".to_packet_id().unwrap_err()
        );
        assert_eq!(
            Error::new("Invalid characters in command \"171G\""),
            "11_1213_1415_10_171G".to_packet_id().unwrap_err()
        );
    }

    #[test]
    fn test_packet_field_id_packet_id_string() {
        let packet_field_id = PacketFieldId(PacketId(0x11, 0x1213, 0x1415, 0x1718), "019_2_0");

        let result = packet_field_id.packet_id_string();

        assert_eq!("11_1213_1415_10_1718", result);
    }

    #[test]
    fn test_packet_field_id_packet_field_id_string() {
        let packet_field_id = PacketFieldId(PacketId(0x11, 0x1213, 0x1415, 0x1718), "019_2_0");

        let result = packet_field_id.packet_field_id_string();

        assert_eq!("11_1213_1415_10_1718_019_2_0", result);
    }

    #[test]
    fn test_packet_field_id_to_packet_field_id() {
        let packet_field_id = PacketFieldId(PacketId(0x11, 0x1213, 0x1415, 0x1718), "019_2_0");

        let result = packet_field_id.to_packet_field_id().expect("Must not fail");

        assert_eq!(packet_field_id, result);
    }

    #[test]
    fn test_str_to_packet_field_id() {
        let packet_field_id_string = "11_1213_1415_10_1718_019_2_0";

        let result = packet_field_id_string
            .to_packet_field_id()
            .expect("Must not fail");

        assert_eq!(packet_field_id_string, result.packet_field_id_string());

        let result = "11_1213_1415_10_1718".to_packet_field_id().unwrap_err();

        assert_eq!(
            Error::new("Invalid length of input \"11_1213_1415_10_1718\""),
            result
        );
    }

    #[test]
    fn test_valid_frame_data_len() {
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

        assert_eq!(100, packet.valid_frame_data_len());
    }

    #[test]
    fn test_valid_frame_data() {
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

        let frame_data = packet.valid_frame_data();
        assert_eq!(100, frame_data.len());
    }

    #[test]
    fn test_valid_frame_data_mut() {
        let mut packet = Packet {
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

        let frame_data = packet.valid_frame_data_mut();
        assert_eq!(100, frame_data.len());
    }

    #[test]
    fn test_packet_to_packet_id() {
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

        let packet_id = packet.to_packet_id().expect("Must return PacketId");

        assert_eq!(PacketId(0x11, 0x1213, 0x1415, 0x1718), packet_id);
    }

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
