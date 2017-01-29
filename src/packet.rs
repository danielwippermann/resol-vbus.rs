use header::Header;


/// The `Packet` type stores information according to the VBus protocol version 1.x.
pub struct Packet {
    /// The shared `Header` of all VBus protocol types.
    header: Header,

    /// The command of this `Packet`.
    command: u16,

    /// The number of 4-byte frames attached to this `Packet`.
    frame_count: u8,

    /// The actual data from the frames attached to this `Packet`.
    frame_data: [u8; 508],
}


impl Packet {

    /// Creates an ID string for this `Packet`.
    pub fn to_id_string(&self) -> String {
        format!("{}_{:04X}", self.header.to_id_string(), self.command)
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_id_string() {
        let timestamp = UTC.timestamp(1485688933, 0);

        let frame_data = [0u8; 508];

        let packet = Packet {
            header: Header {
                timestamp: timestamp,
                channel: 0x11,
                destination_address: 0x1213,
                source_address: 0x1415,
                protocol_version: 0x16,
            },
            command: 0x1718,
            frame_count: 0x19,
            frame_data: frame_data,
        };

        assert_eq!("11_1213_1415_16_1718", packet.to_id_string());
    }
}
