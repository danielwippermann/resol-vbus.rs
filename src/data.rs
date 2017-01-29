use packet::Packet;
use datagram::Datagram;
use telegram::Telegram;


/// `Data` is a type that contains one of the supported VBus protocol data variants.
#[derive(Debug)]
pub enum Data {
    /// Contains a `Packet` conforming to VBus protocol version 1.x.
    Packet(Packet),

    /// Contains a `Datagram` conforming to VBus protocol version 2.x.
    Datagram(Datagram),

    /// Contains a `Telegram` conforming to VBus protocol version 3.x.
    Telegram(Telegram),
}


impl Data {

    /// Creates an ID string for the variant inside this `Data`.
    pub fn to_id_string(&self) -> String {
        match *self {
            Data::Packet(ref packet) => packet.to_id_string(),
            Data::Datagram(ref dgram) => dgram.to_id_string(),
            Data::Telegram(ref tgram) => tgram.to_id_string(),
        }
    }

}
