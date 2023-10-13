use chrono::{DateTime, Utc};

/// A comment stored as a type 0x99 record in the VBus recording file format.
#[derive(Clone, Debug)]
pub struct RecordingComment {
    /// The timestamp that corresponds to the comment record.
    pub timestamp: DateTime<Utc>,
    comment: Vec<u8>,
}

impl RecordingComment {
    /// Construct a `RecordingComment`.
    pub fn new(timestamp: DateTime<Utc>, comment: Vec<u8>) -> RecordingComment {
        RecordingComment { timestamp, comment }
    }

    /// Return the comment bytes.
    pub fn comment(&self) -> &[u8] {
        &self.comment
    }
}
