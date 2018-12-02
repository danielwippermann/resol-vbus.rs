use std::io::{Read, Result, Write};
use std::time::{Duration, Instant};

use chrono::UTC;

use data::Data;
use datagram::Datagram;
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

}
