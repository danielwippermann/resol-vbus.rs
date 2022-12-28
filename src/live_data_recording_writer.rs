use std::io::Write;

use chrono::{DateTime, Utc};

use crate::{
    error::Result,
    recording_encoder::{bytes_from_record, bytes_from_timestamp},
};

/// A `RecordingWriter` for type 0x88 live data recordings.
#[derive(Debug)]
pub struct LiveDataRecordingWriter<W: Write> {
    writer: W,
}

impl<W: Write> LiveDataRecordingWriter<W> {
    /// Construct a new `LiveDataRecordingWriter<T>` instance.
    pub fn new(writer: W) -> LiveDataRecordingWriter<W> {
        LiveDataRecordingWriter { writer }
    }

    /// Gets a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Gets a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Write a type 0x88 live data record.
    pub fn write_raw_data(
        &mut self,
        start_timestamp: DateTime<Utc>,
        end_timestamp: DateTime<Utc>,
        data: &[u8],
    ) -> Result<()> {
        let data_length = data.len();
        let record_length = 22 + data_length;

        let mut bytes = Vec::new();
        bytes.resize(record_length, 0u8);

        let buf = &mut bytes[..];

        bytes_from_record(0x88, record_length as u16, start_timestamp, buf);
        bytes_from_timestamp(end_timestamp, &mut buf[14..22]);
        buf[22..].copy_from_slice(data);

        self.writer.write_all(buf)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        test_data::LIVE_DATA_RECORDING_1, test_utils::test_debug_derive,
        utils::utc_timestamp_with_nsecs,
    };

    #[test]
    fn test_derived_impls() {
        let mut bytes: Vec<u8> = Vec::new();
        let ldrw = LiveDataRecordingWriter::new(&mut bytes);
        test_debug_derive(&ldrw);
    }

    #[test]
    fn test_write_live_data() {
        let mut bytes: Vec<u8> = Vec::new();

        {
            let mut ldrw = LiveDataRecordingWriter::new(&mut bytes);

            let start_timestamp = utc_timestamp_with_nsecs(1486857602, 94000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857602, 95000000);
            let data = &[
                170, 16, 0, 17, 126, 16, 0, 1, 27, 52, 56, 34, 56, 34, 5, 70, 61, 126, 121, 127,
                14, 62, 56, 34, 56, 34, 5, 70, 56, 34, 56, 34, 5, 70, 56, 34, 56, 34, 5, 70, 2, 2,
                56, 34, 4, 29,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857602, 283000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857602, 284000000);
            let data = &[
                15, 39, 15, 39, 0, 19, 15, 39, 70, 5, 0, 126, 15, 39, 15, 39, 0, 19, 15, 39, 15,
                39, 0, 19, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857602, 474000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857602, 474000000);
            let data = &[
                0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 15,
                39, 15, 39, 0, 19, 15, 39, 15, 39, 0, 19,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857602, 683000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857602, 704000000);
            let data = &[
                0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 16,
                14, 0, 0, 0, 97, 1, 6, 0, 0, 0, 120, 0, 0, 0, 0, 0, 127,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857602, 863000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857602, 864000000);
            let data = &[0, 0, 0, 0, 0, 127];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857603, 123000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857603, 124000000);
            let data = &[
                170, 21, 0, 17, 126, 16, 0, 1, 10, 64, 2, 10, 0, 0, 0, 115, 56, 34, 56, 34, 5, 70,
                0, 0, 0, 0, 0, 127, 1, 11, 0, 0, 0, 115, 1, 6, 0, 0, 0, 120,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857603, 314000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857603, 314000000);
            let data = &[
                4, 8, 0, 0, 0, 115, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0,
                0, 0, 0, 0, 127,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857603, 534000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857603, 535000000);
            let data = &[
                170, 85, 102, 17, 126, 16, 0, 2, 14, 21, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0,
                0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857603, 763000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857603, 764000000);
            let data = &[
                0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0,
                0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857604, 114000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857604, 114000000);
            let data = &[170, 0, 0, 17, 126, 32, 0, 5, 0, 0, 0, 0, 0, 0, 0, 75];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857605, 94000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857605, 94000000);
            let data = &[
                170, 16, 0, 17, 126, 16, 0, 1, 27, 52, 56, 34, 56, 34, 5, 70, 60, 126, 121, 127,
                14, 63, 56, 34, 56, 34, 5, 70, 56, 34, 56, 34, 5, 70, 56, 34, 56, 34, 5, 70, 2, 2,
                56, 34, 4, 29,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857605, 283000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857605, 284000000);
            let data = &[
                15, 39, 15, 39, 0, 19, 15, 39, 70, 5, 0, 126, 15, 39, 15, 39, 0, 19, 15, 39, 15,
                39, 0, 19, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857605, 474000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857605, 474000000);
            let data = &[
                0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 15,
                39, 15, 39, 0, 19, 15, 39, 15, 39, 0, 19,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857605, 703000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857605, 704000000);
            let data = &[
                0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 16,
                14, 0, 0, 0, 97, 1, 6, 0, 0, 0, 120, 0, 0, 0, 0, 0, 127,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857605, 863000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857605, 863000000);
            let data = &[0, 0, 0, 0, 0, 127];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857606, 123000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857606, 124000000);
            let data = &[
                170, 21, 0, 17, 126, 16, 0, 1, 10, 64, 2, 10, 0, 0, 0, 115, 56, 34, 56, 34, 5, 70,
                0, 0, 0, 0, 0, 127, 1, 11, 0, 0, 0, 115, 1, 6, 0, 0, 0, 120,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857606, 314000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857606, 314000000);
            let data = &[
                4, 8, 0, 0, 0, 115, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0, 0, 0, 0, 0, 127, 0,
                0, 0, 0, 0, 127,
            ];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();

            let start_timestamp = utc_timestamp_with_nsecs(1486857606, 503000000);
            let end_timestamp = utc_timestamp_with_nsecs(1486857606, 504000000);
            let data = &[170, 0, 0, 17, 126, 32, 0, 5, 0, 0, 0, 0, 0, 0, 0, 75];
            ldrw.write_raw_data(start_timestamp, end_timestamp, data)
                .unwrap();
        }

        assert_eq!(1006, LIVE_DATA_RECORDING_1.len());
        assert_eq!(1006, bytes.len());
        assert_eq!(LIVE_DATA_RECORDING_1, &bytes[..]);
    }
}
