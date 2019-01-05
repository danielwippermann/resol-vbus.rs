use std::io::Read;

use resol_vbus::{DataSet, LiveDataRecordingReader, RecordingReader};

use crate::app_error::Result;

pub trait DataSetReader {
    fn read_data_set(&mut self) -> Result<Option<DataSet>>;
}

impl<R: Read> DataSetReader for LiveDataRecordingReader<R> {
    fn read_data_set(&mut self) -> Result<Option<DataSet>> {
        while let Some(data) = self.read_data()? {
            if !data.is_packet() {
                continue;
            }

            let timestamp = data.as_header().timestamp;

            let mut data_set = DataSet::new();
            data_set.add_data(data);
            data_set.timestamp = timestamp;

            return Ok(Some(data_set));
        }

        Ok(None)
    }
}

impl<R: Read> DataSetReader for RecordingReader<R> {
    fn read_data_set(&mut self) -> Result<Option<DataSet>> {
        Ok(self.read_data_set()?)
    }
}
