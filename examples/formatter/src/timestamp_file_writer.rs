use std::{
    fs::File,
    io::{Error, ErrorKind, Result, Write},
};

use resol_vbus::chrono::{DateTime, Local, Utc};

pub struct TimestampFileWriter {
    filename_pattern: String,
    local_timezone: bool,
    timestamp: DateTime<Utc>,
    timestamp_changed: bool,
    current_filename: Option<String>,
    current_file: Option<File>,
}

impl TimestampFileWriter {
    pub fn new(filename_pattern: String, local_timezone: bool) -> TimestampFileWriter {
        TimestampFileWriter {
            filename_pattern,
            local_timezone,
            timestamp: Utc::now(),
            timestamp_changed: true,
            current_filename: None,
            current_file: None,
        }
    }

    pub fn set_timestamp(&mut self, timestamp: DateTime<Utc>) -> Result<bool> {
        self.timestamp = timestamp;
        self.timestamp_changed = true;

        self.check_timestamp_change()
    }

    pub fn filename(&self) -> Option<&str> {
        match self.current_filename {
            Some(ref s) => Some(s.as_str()),
            None => None,
        }
    }

    fn check_timestamp_change(&mut self) -> Result<bool> {
        if self.timestamp_changed {
            self.timestamp_changed = false;

            let filename = if self.local_timezone {
                self.timestamp.with_timezone(&Local).format(&self.filename_pattern).to_string()
            } else {
                self.timestamp.format(&self.filename_pattern).to_string()
            };

            let filename = Some(filename);
            if self.current_filename != filename {
                let file = File::create(filename.as_ref().unwrap())?;

                self.current_filename = filename;
                self.current_file = Some(file);

                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
}

impl Write for TimestampFileWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.check_timestamp_change()?;

        if let Some(ref mut file) = self.current_file {
            file.write(buf)
        } else {
            Err(Error::new(ErrorKind::Other, "No file created!"))
        }
    }

    fn flush(&mut self) -> Result<()> {
        if let Some(ref mut file) = self.current_file {
            file.flush()
        } else {
            Ok(())
        }
    }
}
