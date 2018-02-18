use std::cmp::max;
use std::io::{Result, Write};

use data_set::DataSet;
use recording_encoder::{length_from_data, bytes_from_record, bytes_from_channel, bytes_from_data};


/// Allows writing the recorded representation of `DataSet` values to a `Write` trait object.
#[derive(Debug)]
pub struct RecordingWriter<W: Write> {
    writer: W,
}


impl<W: Write> RecordingWriter<W> {

    /// Construct a new `RecordingWriter`.
    pub fn new(writer: W) -> RecordingWriter<W> {
        RecordingWriter {
            writer: writer,
        }
    }

    /// Gets a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Gets a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Write the recorded representation of the `DataSet`.
    pub fn write_data_set(&mut self, data_set: &DataSet) -> Result<()> {
        let timestamp = data_set.timestamp;

        let mut data_set: Vec<_> = data_set.iter().collect();
        data_set.sort_by(|l, r| l.as_header().channel.cmp(&r.as_header().channel));

        let max_length = data_set.iter().fold(16, |memo, data| {
            let length = length_from_data(data);
            max(memo, length)
        });

        let mut bytes = Vec::new();
        bytes.resize(max_length, 0u8);

        let buf = &mut bytes [..];
        bytes_from_record(0x44, 14, timestamp, buf);

        self.writer.write_all(&buf [0..14])?;

        let mut current_channel = 0;
        for data in data_set.iter() {
            let channel = data.as_header().channel;
            if current_channel != channel {
                current_channel = channel;

                bytes_from_channel(channel, buf);
                self.writer.write_all(&buf [0..16])?;
            }

            let length = length_from_data(data);
            bytes_from_data(data, buf);
            self.writer.write_all(&buf [0..length])?;
        }

        Ok(())
    }

}


#[cfg(test)]
mod tests {
    use recording_reader::RecordingReader;

    use super::*;

    use test_data::{RECORDING_1};

    #[test]
    fn test_write_data_set() {
        let mut rr = RecordingReader::new(RECORDING_1);

        let data_set = rr.read_data_set().unwrap().unwrap();

        let mut writer: Vec<u8> = Vec::new();

        {
            let mut rw = RecordingWriter::new(&mut writer);

            rw.write_data_set(&data_set).unwrap();
        }

        assert_eq!(740, RECORDING_1.len());
        assert_eq!(740, writer.len());
        assert_eq!(&RECORDING_1 [0..740], &writer [0..740]);
    }
}
