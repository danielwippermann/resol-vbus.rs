use std::io::Write;

use crate::{
    data::Data,
    error::Result,
    live_data_encoder::{bytes_from_data, length_from_data},
};

/// Allows writing the live represenation of `Data` variants to a `Write` trait object.
#[derive(Debug)]
pub struct LiveDataWriter<W: Write> {
    writer: W,
}

impl<W: Write> LiveDataWriter<W> {
    /// Construct a new `LiveDataWriter`.
    pub fn new(writer: W) -> LiveDataWriter<W> {
        LiveDataWriter { writer }
    }

    /// Gets a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Gets a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Write the live representation of the `Data` variant.
    pub fn write_data(&mut self, data: &Data) -> Result<()> {
        let length = length_from_data(data);

        let mut bytes = Vec::new();
        bytes.resize(length, 0u8);

        bytes_from_data(data, &mut bytes);

        self.writer.write_all(&bytes)?;

        Ok(())
    }
}

impl<W: Write> AsRef<W> for LiveDataWriter<W> {
    fn as_ref(&self) -> &W {
        &self.writer
    }
}

impl<W: Write> AsMut<W> for LiveDataWriter<W> {
    fn as_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        live_data_decoder::data_from_checked_bytes,
        test_data::{LIVE_DATA_1, LIVE_TELEGRAM_1},
        utils::utc_timestamp,
    };

    #[test]
    fn test_write_data() {
        let mut buf = Vec::new();

        let timestamp = utc_timestamp(1485688933);
        let channel = 0x11;

        let data1 = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[0..]);

        {
            buf.truncate(0);
            let mut writer = LiveDataWriter::new(&mut buf);
            writer.write_data(&data1).unwrap();
        }
        assert_eq!(&LIVE_DATA_1[0..172], &buf[0..172]);

        let data2 = data_from_checked_bytes(timestamp, channel, &LIVE_DATA_1[352..]);

        {
            buf.truncate(0);
            let mut writer = LiveDataWriter::new(&mut buf);
            writer.write_data(&data2).unwrap();
        }
        assert_eq!(&LIVE_DATA_1[352..368], &buf[0..16]);

        let data3 = data_from_checked_bytes(timestamp, channel, &LIVE_TELEGRAM_1[0..]);

        {
            buf.truncate(0);
            let mut writer = LiveDataWriter::new(&mut buf);
            writer.write_data(&data3).unwrap();
        }
        assert_eq!(&LIVE_TELEGRAM_1[0..17], &buf[0..17]);
    }
}
