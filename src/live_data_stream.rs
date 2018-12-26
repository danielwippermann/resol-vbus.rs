use std::io::{Read, Write};
use std::time::{Duration, Instant};

use chrono::UTC;

use data::Data;
use datagram::Datagram;
use error::Result;
use header::Header;
use live_data_reader::LiveDataReader;
use live_data_writer::LiveDataWriter;
use read_with_timeout::ReadWithTimeout;


/// Allows reading and writing live data.
#[derive(Debug)]
pub struct LiveDataStream<R: Read + ReadWithTimeout, W: Write> {
    channel: u8,
    self_address: u16,
    reader: LiveDataReader<R>,
    writer: LiveDataWriter<W>,
}


impl<R: Read + ReadWithTimeout, W: Write> LiveDataStream<R, W> {

    /// Constructs a `LiveDataStream`.
    pub fn new(channel: u8, self_address: u16, reader: R, writer: W) -> Result<LiveDataStream<R, W>> {
        Ok(LiveDataStream {
            channel,
            self_address,
            reader: LiveDataReader::new(channel, reader),
            writer: LiveDataWriter::new(writer),
        })
    }

    fn create_datagram(&self, destination_address: u16, command: u16, param16: i16, param32: i32) -> Data {
        Data::Datagram(Datagram {
            header: Header {
                timestamp: UTC::now(),
                channel: self.channel,
                destination_address,
                source_address: self.self_address,
                protocol_version: 0x20,
            },
            command,
            param16,
            param32,
        })
    }

    fn receive_internal(&mut self, timeout: Duration) -> Result<Option<Data>> {
        match self.reader.read_data_with_timeout(Some(timeout)) {
            Ok(Some(rx_data)) => {
                Ok(Some(rx_data))
            },
            Ok(None) => {
                Ok(None)
            },
            Err(_) => {
                Ok(None)
            },
        }
    }

    /// Transmit data to the stream.
    pub fn transmit(&mut self, tx_data: &Data) -> Result<()> {
        self.writer.write_data(tx_data)
    }

    /// Receive data from the stream.
    pub fn receive(&mut self, timeout_ms: u32) -> Result<Option<Data>> {
        let timeout = Duration::from_millis(timeout_ms as u64);
        self.receive_internal(timeout)
    }

    /// Transmit and receive data from the stream.
    pub fn transceive<F>(&mut self, tx_data: Option<Data>, tries: u32, initial_timeout_ms: u32, timeout_incr_ms: i32, filter: F) -> Result<Option<Data>> where F: Fn(&Data) -> bool {
        let mut timeout = Duration::from_millis(initial_timeout_ms as u64);
        let timeout_incr = Duration::from_millis(timeout_incr_ms as u64);

        for _try in 0..tries {
            let start_at = Instant::now();

            if let Some(ref tx_data) = tx_data {
                self.transmit(tx_data)?;
            }

            loop {
                let now = Instant::now();

                let dur = now - start_at;
                if dur > timeout {
                    break;
                }

                match self.receive_internal(timeout - dur)? {
                    Some(rx_data) => {
                        if filter(&rx_data) {
                            return Ok(Some(rx_data));
                        }
                    },
                    None => {
                        break;
                    },
                }
            }

            timeout = timeout + timeout_incr;
        }

        Ok(None)
    }

    /// Wait for a bus offer datagram from the controller.
    pub fn wait_for_free_bus(&mut self) -> Result<Option<Data>> {
        self.transceive(None, 1, 20000, 0, |data| {
            match *data {
                Data::Datagram(ref dgram) => dgram.command == 0x0500,
                _ => false,
            }
        })
    }

    /// Release the bus control back to the controller.
    pub fn release_bus(&mut self, address: u16) -> Result<Option<Data>> {
        let tx_data = self.create_datagram(address, 0x0600, 0, 0);

        self.transceive(Some(tx_data), 2, 1500, 0, |data| {
            match *data {
                Data::Packet(_) => true,
                _ => false,
            }
        })
    }

    /// Get value from controller.
    pub fn get_value_by_index(&mut self, address: u16, value_index: i16, sub_index: u8) -> Result<Option<Data>> {
        let tx_data = self.create_datagram(address, 0x0300 | (sub_index as u16), value_index, 0);
        let self_address = self.self_address;

        self.transceive(Some(tx_data), 3, 500, 500, |data| {
            match *data {
                Data::Datagram(ref dgram) => {
                    if dgram.header.destination_address != self_address {
                        false
                    } else if dgram.header.source_address != address {
                        false
                    } else if dgram.command != (0x0100 | (sub_index as u16)) {
                        false
                    } else if dgram.param16 != value_index {
                        false
                    } else {
                        true
                    }
                },
                _ => false,
            }
        })
    }

    /// Set value in controller.
    pub fn set_value_by_index(&mut self, address: u16, value_index: i16, sub_index: u8, value: i32) -> Result<Option<Data>> {
        let tx_data = self.create_datagram(address, 0x0200 | (sub_index as u16), value_index, value);
        let self_address = self.self_address;

        self.transceive(Some(tx_data), 3, 500, 500, |data| {
            match *data {
                Data::Datagram(ref dgram) => {
                    if dgram.header.destination_address != self_address {
                        false
                    } else if dgram.header.source_address != address {
                        false
                    } else if dgram.command != (0x0100 | (sub_index as u16)) {
                        false
                    } else if dgram.param16 != value_index {
                        false
                    } else {
                        true
                    }
                },
                _ => false,
            }
        })
    }

    /// Get value index by ID hash.
    pub fn get_value_id_hash_by_index(&mut self, address: u16, value_index: i16) -> Result<Option<Data>> {
        let tx_data = self.create_datagram(address, 0x1000, value_index, 0);
        let self_address = self.self_address;

        self.transceive(Some(tx_data), 3, 500, 500, |data| {
            match *data {
                Data::Datagram(ref dgram) => {
                    if dgram.header.destination_address != self_address {
                        false
                    } else if dgram.header.source_address != address {
                        false
                    } else if (dgram.command != 0x0100) && (dgram.command != 0x1001) {
                        false
                    } else if dgram.param16 != value_index {
                        false
                    } else {
                        true
                    }
                },
                _ => false,
            }
        })
    }

    /// Get value index by ID hash.
    pub fn get_value_index_by_id_hash(&mut self, address: u16, value_id_hash: i32) -> Result<Option<Data>> {
        let tx_data = self.create_datagram(address, 0x1100, 0, value_id_hash);
        let self_address = self.self_address;

        self.transceive(Some(tx_data), 3, 500, 500, |data| {
            match *data {
                Data::Datagram(ref dgram) => {
                    if dgram.header.destination_address != self_address {
                        false
                    } else if dgram.header.source_address != address {
                        false
                    } else if (dgram.command != 0x0100) && (dgram.command != 0x1101) {
                        false
                    } else if dgram.param32 != value_id_hash {
                        false
                    } else {
                        true
                    }
                },
                _ => false,
            }
        })
    }

    /// Get caps 1.
    pub fn get_caps1(&mut self, address: u16) -> Result<Option<Data>> {
        let tx_data = self.create_datagram(address, 0x1300, 0, 0);
        let self_address = self.self_address;

        self.transceive(Some(tx_data), 3, 500, 500, |data| {
            match *data {
                Data::Datagram(ref dgram) => {
                    if dgram.header.destination_address != self_address {
                        false
                    } else if dgram.header.source_address != address {
                        false
                    } else if dgram.command != 0x1301 {
                        false
                    } else {
                        true
                    }
                },
                _ => false,
            }
        })
    }

    /// Begin a bulk value transaction.
    pub fn begin_bulk_value_transaction(&mut self, address: u16, tx_timeout: i32) -> Result<Option<Data>> {
        let tx_data = self.create_datagram(address, 0x1400, 0, tx_timeout);
        let self_address = self.self_address;

        self.transceive(Some(tx_data), 3, 500, 500, |data| {
            match *data {
                Data::Datagram(ref dgram) => {
                    if dgram.header.destination_address != self_address {
                        false
                    } else if dgram.header.source_address != address {
                        false
                    } else if dgram.command != 0x1401 {
                        false
                    } else {
                        true
                    }
                },
                _ => false,
            }
        })
    }

    /// Commit a bulk value transaction.
    pub fn commit_bulk_value_transaction(&mut self, address: u16) -> Result<Option<Data>> {
        let tx_data = self.create_datagram(address, 0x1402, 0, 0);
        let self_address = self.self_address;

        self.transceive(Some(tx_data), 3, 500, 500, |data| {
            match *data {
                Data::Datagram(ref dgram) => {
                    if dgram.header.destination_address != self_address {
                        false
                    } else if dgram.header.source_address != address {
                        false
                    } else if dgram.command != 0x1403 {
                        false
                    } else {
                        true
                    }
                },
                _ => false,
            }
        })
    }

    /// Rollback a bulk value transaction.
    pub fn rollback_bulk_value_transaction(&mut self, address: u16) -> Result<Option<Data>> {
        let tx_data = self.create_datagram(address, 0x1404, 0, 0);
        let self_address = self.self_address;

        self.transceive(Some(tx_data), 3, 500, 500, |data| {
            match *data {
                Data::Datagram(ref dgram) => {
                    if dgram.header.destination_address != self_address {
                        false
                    } else if dgram.header.source_address != address {
                        false
                    } else if dgram.command != 0x1405 {
                        false
                    } else {
                        true
                    }
                },
                _ => false,
            }
        })
    }

    /// Set a value during a bulk value transaction.
    pub fn set_bulk_value_by_index(&mut self, address: u16, value_index: i16, sub_index: u8, value: i32) -> Result<Option<Data>> {
        let tx_data = self.create_datagram(address, 0x1500 | (sub_index as u16), value_index, value);
        let self_address = self.self_address;
        let resp_command = 0x1600 | (sub_index as u16);

        self.transceive(Some(tx_data), 3, 500, 500, |data| {
            match *data {
                Data::Datagram(ref dgram) => {
                    if dgram.header.destination_address != self_address {
                        false
                    } else if dgram.header.source_address != address {
                        false
                    } else if dgram.command != resp_command {
                        false
                    } else {
                        true
                    }
                },
                _ => false,
            }
        })
    }

    #[cfg(test)]
    pub fn reader_mut(&mut self) -> &mut R {
        self.reader.as_mut()
    }

    #[cfg(test)]
    pub fn writer_mut(&mut self) -> &mut W {
        self.writer.as_mut()
    }
}


#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io::{Write};

    use live_data_decoder::{length_from_bytes, data_from_checked_bytes};
    use live_data_encoder::{length_from_data, bytes_from_data};
    use stream_blob_length::StreamBlobLength;
    use utils::utc_timestamp;

    use super::*;

    use test_data::LIVE_DATA_1;
    use test_utils::Buffer;


    #[test]
    fn test_new() {
        let mut lds = LiveDataStream::new(0, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        assert_eq!(0, lds.reader_mut().unread_len());
        assert_eq!(0, lds.writer_mut().written_len());
    }

    #[test]
    fn test_transmit() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        let timestamp = utc_timestamp(1544209081);

        let data1 = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [0..]);

        lds.transmit(&data1).unwrap();

        assert_eq!(0, lds.reader_mut().unread_len());
        assert_eq!(0, lds.reader_mut().read_call_count());
        assert_eq!(172, lds.writer_mut().written_len());
        assert_eq!(1, lds.writer_mut().write_call_count());

        assert_eq!(StreamBlobLength::BlobLength(172), length_from_bytes(&lds.writer_mut().written_bytes()));
    }

    #[test]
    fn test_receive() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        lds.reader_mut().write(&LIVE_DATA_1 [0..172]).unwrap();

        let data1 = lds.receive(1000).unwrap().unwrap();

        assert_eq!("00_0010_7E11_10_0100", data1.id_string());

        lds.reader_mut().write(&LIVE_DATA_1 [172..232]).unwrap();

        let data2 = lds.receive(1000).unwrap();

        assert_eq!(None, data2);

        lds.reader_mut().write(&LIVE_DATA_1 [232..242]).unwrap();

        let data3 = lds.receive(1000).unwrap().unwrap();

        assert_eq!("00_0015_7E11_10_0100", data3.id_string());
    }

    #[test]
    fn test_transceive() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        let timestamp = utc_timestamp(1544209081);

        let tx_data1 = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1 [0..]);

        let rx_data_list = RefCell::new(Vec::new());

        lds.reader_mut().write(&LIVE_DATA_1 [172..258]).unwrap();

        let result1 = lds.transceive(Some(tx_data1), 3, 500, 500, |rx_data| {
            rx_data_list.borrow_mut().push(rx_data.clone());
            false
        }).unwrap();

        let rx_data_list = rx_data_list.into_inner();

        assert_eq!(None, result1);
        assert_eq!(2, rx_data_list.len());
        assert_eq!(3, lds.writer_mut().write_call_count());
        assert_eq!(3 * 172, lds.writer_mut().written_len());
        assert_eq!(4, lds.reader_mut().read_call_count());
        assert_eq!(0, lds.reader_mut().unread_len());

        lds.writer_mut().reset();
        lds.reader_mut().reset();

        lds.reader_mut().write(&LIVE_DATA_1 [172..258]).unwrap();

        let result2 = lds.transceive(None, 1, 500, 0, |_rx_data| {
            true
        }).unwrap();

        assert_eq!(true, result2.is_some());
        assert_eq!(0, lds.writer_mut().write_call_count());
        assert_eq!(0, lds.writer_mut().written_len());
        assert_eq!(1, lds.reader_mut().read_call_count());
        assert_eq!(0, lds.reader_mut().unread_len());

        let result3 = lds.transceive(None, 1, 500, 0, |_rx_data| {
            true
        }).unwrap();

        assert_eq!(true, result3.is_some());
        assert_eq!(0, lds.writer_mut().write_call_count());
        assert_eq!(0, lds.writer_mut().written_len());
        assert_eq!(1, lds.reader_mut().read_call_count());
        assert_eq!(0, lds.reader_mut().unread_len());

        let result4 = lds.transceive(None, 1, 500, 0, |_rx_data| {
            true
        }).unwrap();

        assert_eq!(None, result4);
        assert_eq!(0, lds.writer_mut().write_call_count());
        assert_eq!(0, lds.writer_mut().written_len());
        assert_eq!(2, lds.reader_mut().read_call_count());
        assert_eq!(0, lds.reader_mut().unread_len());
    }

    fn read_written_data(lds: &mut LiveDataStream<Buffer, Buffer>) -> Option<Data> {
        match length_from_bytes(lds.writer_mut().written_bytes()) {
            StreamBlobLength::BlobLength(size) => {
                let mut bytes = [0u8; 1024];
                lds.writer_mut().read(&mut bytes [0..size]).unwrap();
                let data = data_from_checked_bytes(utc_timestamp(1544209081), 0, &bytes [0..size]);
                Some(data)
            },
            _ => None,
        }
    }

    fn write_datagram(lds: &mut LiveDataStream<Buffer, Buffer>, destination_address: u16, source_address: u16, command: u16, param16: i16, param32: i32) {
        let data = Data::Datagram(Datagram {
            header: Header {
                timestamp: UTC::now(),
                channel: 0,
                destination_address,
                source_address,
                protocol_version: 0x20,
            },
            command,
            param16,
            param32,
        });

        let mut bytes = [0u8; 1024];

        let size = length_from_data(&data);

        bytes_from_data(&data, &mut bytes [0..size]);

        lds.reader_mut().write(&bytes [0..size]).unwrap();
    }

    #[test]
    fn test_wait_for_free_bus() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        write_datagram(&mut lds, 0x0000, 0x7E11, 0x0500, 0, 0);

        assert_eq!("00_0000_7E11_20_0500_0000", lds.wait_for_free_bus().unwrap().unwrap().id_string());

        assert_eq!(None, lds.wait_for_free_bus().unwrap());
    }

    #[test]
    fn test_release_bus() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        lds.reader_mut().write(&LIVE_DATA_1 [0..172]).unwrap();

        assert_eq!("00_0010_7E11_10_0100", lds.release_bus(0x7E11).unwrap().unwrap().id_string());
        assert_eq!(16, lds.writer_mut().written_len());
        assert_eq!("00_7E11_0020_20_0600_0000", read_written_data(&mut lds).unwrap().id_string());

        lds.writer_mut().reset();

        assert_eq!(None, lds.release_bus(0x7E11).unwrap());
        assert_eq!(32, lds.writer_mut().written_len());
        assert_eq!("00_7E11_0020_20_0600_0000", read_written_data(&mut lds).unwrap().id_string());
        assert_eq!("00_7E11_0020_20_0600_0000", read_written_data(&mut lds).unwrap().id_string());
    }

    #[test]
    fn test_get_value_by_index() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        let value_index = 0x1234;
        let value = 0x56789abc;

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x0101, value_index, value);

        assert_eq!("00_0020_7E11_20_0101_0000", lds.get_value_by_index(0x7E11, value_index, 1).unwrap().unwrap().id_string());
        assert_eq!(16, lds.writer_mut().written_len());
        assert_eq!("00_7E11_0020_20_0301_0000", read_written_data(&mut lds).unwrap().id_string());

        lds.writer_mut().reset();

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x0100, value_index, value);

        assert_eq!(None, lds.get_value_by_index(0x7E11, value_index, 1).unwrap());

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x0301, value_index, value);

        assert_eq!(None, lds.get_value_by_index(0x7E11, value_index, 1).unwrap());

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x0101, value_index + 1, value);

        assert_eq!(None, lds.get_value_by_index(0x7E11, value_index, 1).unwrap());
    }

    #[test]
    fn test_set_value_by_index() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        let value_index = 0x1234;
        let value = 0x56789abc;

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x0101, value_index, value);

        assert_eq!("00_0020_7E11_20_0101_0000", lds.set_value_by_index(0x7E11, value_index, 1, value).unwrap().unwrap().id_string());
        assert_eq!(16, lds.writer_mut().written_len());
        assert_eq!("00_7E11_0020_20_0201_0000", read_written_data(&mut lds).unwrap().id_string());

        lds.writer_mut().reset();

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x0100, value_index, value);

        assert_eq!(None, lds.set_value_by_index(0x7E11, value_index, 1, value).unwrap());

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x0201, value_index, value);

        assert_eq!(None, lds.set_value_by_index(0x7E11, value_index, 1, value).unwrap());

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x0101, value_index + 1, value);

        assert_eq!(None, lds.set_value_by_index(0x7E11, value_index, 1, value).unwrap());
    }

    #[test]
    fn test_get_value_id_hash_by_index() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        let value_index = 0x1234;
        let value_id_hash = 0x56789abc;

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x0100, value_index, value_id_hash);

        assert_eq!("00_0020_7E11_20_0100_0000", lds.get_value_id_hash_by_index(0x7E11, value_index).unwrap().unwrap().id_string());
        assert_eq!(16, lds.writer_mut().written_len());
        assert_eq!("00_7E11_0020_20_1000_0000", read_written_data(&mut lds).unwrap().id_string());
    }

    #[test]
    fn test_get_value_index_by_id_hash() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        let value_index = 0x1234;
        let value_id_hash = 0x56789abc;

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x0100, value_index, value_id_hash);

        assert_eq!("00_0020_7E11_20_0100_0000", lds.get_value_index_by_id_hash(0x7E11, value_id_hash).unwrap().unwrap().id_string());
        assert_eq!(16, lds.writer_mut().written_len());
        assert_eq!("00_7E11_0020_20_1100_0000", read_written_data(&mut lds).unwrap().id_string());
    }

    #[test]
    fn test_get_caps1() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x1301, 0, 0);

        assert_eq!("00_0020_7E11_20_1301_0000", lds.get_caps1(0x7E11).unwrap().unwrap().id_string());
        assert_eq!(16, lds.writer_mut().written_len());
        assert_eq!("00_7E11_0020_20_1300_0000", read_written_data(&mut lds).unwrap().id_string());
    }

    #[test]
    fn test_begin_bulk_value_transaction() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x1401, 0, 0);

        assert_eq!("00_0020_7E11_20_1401_0000", lds.begin_bulk_value_transaction(0x7E11, 10).unwrap().unwrap().id_string());
        assert_eq!(16, lds.writer_mut().written_len());
        assert_eq!("00_7E11_0020_20_1400_0000", read_written_data(&mut lds).unwrap().id_string());
    }

    #[test]
    fn test_commit_bulk_value_transaction() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x1403, 0, 0);

        assert_eq!("00_0020_7E11_20_1403_0000", lds.commit_bulk_value_transaction(0x7E11).unwrap().unwrap().id_string());
        assert_eq!(16, lds.writer_mut().written_len());
        assert_eq!("00_7E11_0020_20_1402_0000", read_written_data(&mut lds).unwrap().id_string());
    }

    #[test]
    fn test_rollback_bulk_value_transaction() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x1405, 0, 0);

        assert_eq!("00_0020_7E11_20_1405_0000", lds.rollback_bulk_value_transaction(0x7E11).unwrap().unwrap().id_string());
        assert_eq!(16, lds.writer_mut().written_len());
        assert_eq!("00_7E11_0020_20_1404_0000", read_written_data(&mut lds).unwrap().id_string());
    }

    #[test]
    fn test_set_bulk_value_by_index() {
        let channel = 0x00;

        let mut lds = LiveDataStream::new(channel, 0x0020, Buffer::new(), Buffer::new()).unwrap();

        let value_index = 0x1234;
        let value = 0x56789abc;

        write_datagram(&mut lds, 0x0020, 0x7E11, 0x1601, 0, 0);

        assert_eq!("00_0020_7E11_20_1601_0000", lds.set_bulk_value_by_index(0x7E11, value_index, 1, value).unwrap().unwrap().id_string());
        assert_eq!(16, lds.writer_mut().written_len());
        assert_eq!("00_7E11_0020_20_1501_0000", read_written_data(&mut lds).unwrap().id_string());
    }
}
