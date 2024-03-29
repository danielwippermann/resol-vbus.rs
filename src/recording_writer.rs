use std::{cmp::max, io::Write};

use crate::{
    data_set::DataSet,
    error::Result,
    recording_encoder::{bytes_from_channel, bytes_from_data, bytes_from_record, length_from_data},
};

/// Allows writing the recorded representation of `DataSet` values to a `Write` trait object.
#[derive(Debug)]
pub struct RecordingWriter<W: Write> {
    writer: W,
}

impl<W: Write> RecordingWriter<W> {
    /// Construct a new `RecordingWriter`.
    pub fn new(writer: W) -> RecordingWriter<W> {
        RecordingWriter { writer }
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

        let mut bytes = vec![0; max_length];

        let buf = &mut bytes[..];
        bytes_from_record(0x44, 14, timestamp, buf);

        self.writer.write_all(&buf[0..14])?;

        let mut current_channel = 0;
        for data in data_set.iter() {
            let channel = data.as_header().channel;
            if current_channel != channel {
                current_channel = channel;

                bytes_from_channel(channel, buf);
                self.writer.write_all(&buf[0..16])?;
            }

            let length = length_from_data(data);
            bytes_from_data(data, buf);
            self.writer.write_all(&buf[0..length])?;
        }

        Ok(())
    }
}

impl<W: Write> AsRef<W> for RecordingWriter<W> {
    fn as_ref(&self) -> &W {
        &self.writer
    }
}

impl<W: Write> AsMut<W> for RecordingWriter<W> {
    fn as_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        recording_reader::RecordingReader, test_data::RECORDING_1, test_utils::test_debug_derive,
    };

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
        assert_eq!(&RECORDING_1[0..740], &writer[0..740]);
    }

    #[test]
    fn test_derived_trait_impls() {
        let mut writer: Vec<u8> = Vec::new();

        let rw = RecordingWriter::new(&mut writer);

        test_debug_derive(&rw);
    }

    #[test]
    fn test_as_ref() {
        let mut writer: Vec<u8> = Vec::new();
        writer.extend_from_slice(RECORDING_1);

        let rw = RecordingWriter::new(&mut writer);

        assert_eq!(RECORDING_1, rw.as_ref().as_slice());
    }

    #[test]
    fn test_as_mut() {
        let mut writer: Vec<u8> = Vec::new();
        writer.extend_from_slice(RECORDING_1);

        let mut rw = RecordingWriter::new(&mut writer);

        assert_eq!(RECORDING_1, rw.as_mut().as_slice());
    }
}
