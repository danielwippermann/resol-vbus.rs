use std::fmt::{Debug, Error, Formatter};
use std::hash::{Hash, Hasher};

use id_hash::IdHash;
use header::Header;


/// The `Telegram` type stores information according to the VBus protocol version 3.x.
///
/// Telegrams are used to transmit small amount of information (up to 21 bytes of payload).
///
/// ## The "identity" of `Telegram` values
///
/// As described in [the corresponding section of the `Header` struct][1] VBus data types use
/// some of their fields as part of their "identity". In addition to the fields used by the
/// `Header` type the `Telegram` type also respects the `command` field. That means that two
/// `Telegram` with differing `timestamp` and `frame_data` fields are still considered
/// "identical", if the other fields match.
///
/// [1]: struct.Header.html#the-identity-of-header-values
pub struct Telegram {
    /// The shared `Header` of all VBus protocol types.
    pub header: Header,

    /// The command of this `Telegram`.
    pub command: u8,

    /// The actual data from the frames attached to this `Telegram`.
    pub frame_data: [u8; 21],
}


impl Telegram {

    /// Get number of frames from a VBus protocol version 3.x command.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::Telegram;
    ///
    /// assert_eq!(0, Telegram::frame_count_from_command(0x1F));
    /// assert_eq!(1, Telegram::frame_count_from_command(0x3F));
    /// assert_eq!(2, Telegram::frame_count_from_command(0x5F));
    /// assert_eq!(3, Telegram::frame_count_from_command(0x7F));
    /// ```
    pub fn frame_count_from_command(command: u8) -> u8 {
        command >> 5
    }

    /// Get number of 7-byte frames attached to this `Telegram`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Telegram, Header};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let tgram = Telegram {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x36,
    ///     },
    ///     command: 0x37,
    ///     frame_data: [0u8; 21],
    /// };
    ///
    /// assert_eq!(1, tgram.frame_count());
    /// ```
    pub fn frame_count(&self) -> u8 {
        Telegram::frame_count_from_command(self.command)
    }

    /// Creates an identification string for this `Telegram`.
    ///
    /// The string contains all fields that count towards the "identity" of the `Telegram`:
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
    /// use resol_vbus::{Telegram, Header};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let tgram = Telegram {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x36,
    ///     },
    ///     command: 0x17,
    ///     frame_data: [0u8; 21],
    /// };
    ///
    /// assert_eq!("11_1213_1415_36_17", tgram.id_string());
    /// ```
    pub fn id_string(&self) -> String {
        format!("{}_{:02X}", self.header.id_string(), self.command)
    }

}


impl IdHash for Telegram {

    /// Returns an identification hash for this `Telegram`.
    ///
    /// The hash contains all fields that count towards the "identity" of the `Telegram`:
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
    /// use resol_vbus::{Header, Telegram, id_hash};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let tgram = Telegram {
    ///     header: Header {
    ///         timestamp: utc_timestamp(1485688933),
    ///         channel: 0x11,
    ///         destination_address: 0x1213,
    ///         source_address: 0x1415,
    ///         protocol_version: 0x36,
    ///     },
    ///     command: 0x17,
    ///     frame_data: [0u8; 21],
    /// };
    ///
    /// assert_eq!(7671625633196679790, id_hash(&tgram));
    /// ```
    fn id_hash<H: Hasher>(&self, h: &mut H) {
        self.header.id_hash(h);
        self.command.hash(h);
    }

}


impl Debug for Telegram {

    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("Telegram")
            .field("header", &self.header)
            .field("command", &format_args!("0x{:02X}", self.command))
            .field("frame_data", &format_args!("..."))
            .finish()
    }

}


impl Clone for Telegram {

    fn clone(&self) -> Self {
        let mut frame_data = [0u8; 21];
        frame_data.copy_from_slice(&self.frame_data);

        Telegram {
            header: self.header.clone(),
            command: self.command,
            frame_data: frame_data,
        }
    }

}


impl AsRef<Header> for Telegram {

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
        let tgram = Telegram {
            header: Header {
                timestamp: utc_timestamp(1485688933),
                channel: 0x11,
                destination_address: 0x1213,
                source_address: 0x1415,
                protocol_version: 0x36,
            },
            command: 0x17,
            frame_data: [0u8; 21],
        };

        let result = format!("{:?}", tgram);

        assert_eq!("Telegram { header: Header { timestamp: 2017-01-29T11:22:13Z, channel: 0x11, destination_address: 0x1213, source_address: 0x1415, protocol_version: 0x36 }, command: 0x17, frame_data: ... }", result);
    }
}
