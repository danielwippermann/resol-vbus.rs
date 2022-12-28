use std::{
    fmt,
    hash::{Hash, Hasher},
};

use crate::{header::Header, id_hash::IdHash};

/// The `Datagram` type stores information according to the VBus protocol version 2.x.
///
/// Datagrams are used to issue simple commands with limited amount of payload (like e.g. getting
/// or setting a parameter).
///
/// ## The "identity" of `Datagram` values
///
/// As described in [the corresponding section of the `Header` struct][1] VBus data types use
/// some of their fields as part of their "identity". In addition to the fields used by the
/// `Header` type the `Datagram` type also respects the `command` and (under some conditions) the
/// `param16` fields. That means that two `Datagram` with differing `timestamp`, `param32` and
/// (under some conditions) `param16` fields are still considered "identical", if the other fields
/// match.
///
/// [1]: struct.Header.html#the-identity-of-header-values
#[derive(Clone)]
pub struct Datagram {
    /// The shared `Header` of all VBus protocol types.
    pub header: Header,

    /// The command of this `Datagram`.
    pub command: u16,

    /// The 16-bit parameter attached to this `Datagram`.
    pub param16: i16,

    /// The 32-bit parameter attached to this `Datagram`.
    pub param32: i32,
}

impl Datagram {
    /// Creates an identification string for this `Datagram`.
    ///
    /// The string contains all fields that count towards the "identity" of the `Datagram`:
    ///
    /// - `channel`
    /// - `destination_address`
    /// - `source_address`
    /// - `protocol_version`
    /// - `command`
    /// - `param16` (if `command` equals 0x0900)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Header, Datagram};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let dgram1 = Datagram {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x26,
    ///     },
    ///     command: 0x1718,
    ///     param16: 0x191a,
    ///     param32: 0x1b1c1d1e,
    /// };
    ///
    /// let dgram2 = Datagram {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x26,
    ///     },
    ///     command: 0x0900,
    ///     param16: 0x191a,
    ///     param32: 0x1b1c1d1e,
    /// };
    ///
    /// assert_eq!("11_1213_1415_26_1718_0000", dgram1.id_string());
    /// assert_eq!("11_1213_1415_26_0900_191A", dgram2.id_string());
    /// ```
    pub fn id_string(&self) -> String {
        let info = match self.command {
            0x0900 => self.param16,
            _ => 0,
        };
        format!(
            "{}_{:04X}_{:04X}",
            self.header.id_string(),
            self.command,
            info
        )
    }
}

impl IdHash for Datagram {
    /// Returns an identification hash for this `Datagram`.
    ///
    /// The hash contains all fields that count towards the "identity" of the `Datagram`:
    ///
    /// - `channel`
    /// - `destination_address`
    /// - `source_address`
    /// - `protocol_version`
    /// - `command`
    /// - `param16` (if `command` equals 0x0900)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Header, Datagram, id_hash};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let dgram = Datagram {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x26,
    ///     },
    ///     command: 0x1718,
    ///     param16: 0x191a,
    ///     param32: 0x1b1c1d1e,
    /// };
    ///
    /// assert_eq!(2264775891674525017, id_hash(&dgram));
    /// ```
    fn id_hash<H: Hasher>(&self, h: &mut H) {
        let info = match self.command {
            0x0900 => self.param16,
            _ => 0,
        };

        self.header.id_hash(h);
        self.command.hash(h);
        info.hash(h);
    }
}

impl fmt::Debug for Datagram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Datagram")
            .field("header", &self.header)
            .field("command", &format_args!("0x{:04X}", self.command))
            .field("param16", &format_args!("0x{:04X}", self.param16))
            .field(
                "param32",
                &format_args!("0x{:08X} ({})", self.param32, self.param32),
            )
            .finish()
    }
}

impl AsRef<Header> for Datagram {
    fn as_ref(&self) -> &Header {
        &self.header
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{header::Header, id_hash, utils::utc_timestamp};

    #[test]
    fn test_id_hash() {
        let timestamp = utc_timestamp(1485688933);

        let dgram = Datagram {
            header: Header {
                timestamp,
                channel: 0x11,
                destination_address: 0x1213,
                source_address: 0x1415,
                protocol_version: 0x26,
            },
            command: 0x1718,
            param16: 0x191a,
            param32: 0x1b1c1d1e,
        };

        assert_eq!(2264775891674525017, id_hash(&dgram));

        let dgram = Datagram {
            header: Header {
                timestamp,
                channel: 0x11,
                destination_address: 0x1213,
                source_address: 0x1415,
                protocol_version: 0x26,
            },
            command: 0x0900,
            param16: 0x191a,
            param32: 0x1b1c1d1e,
        };

        assert_eq!(11755850012962607095, id_hash(&dgram));
    }

    #[test]
    fn test_debug_fmt() {
        let timestamp = utc_timestamp(1485688933);

        let dgram = Datagram {
            header: Header {
                timestamp,
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
