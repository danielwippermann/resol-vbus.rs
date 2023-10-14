use chrono::{DateTime, Utc};

/// A comment stored as a type 0x99 record in the VBus recording file format.
#[derive(Clone, Debug, Default)]
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

#[cfg(test)]
mod tests {
    use crate::{
        test_utils::{test_clone_derive, test_debug_derive},
        utils::current_timestamp,
    };

    use super::*;

    // "1 junk byte(s) received"
    const COMMENT_BYTES_1: &'static [u8] = &[
        0x31, 0x20, 0x6a, 0x75, 0x6e, 0x6b, 0x20, 0x62, 0x79, 0x74, 0x65, 0x28, 0x73, 0x29, 0x20,
        0x72, 0x65, 0x63, 0x65, 0x69, 0x76, 0x65, 0x64,
    ];

    #[test]
    fn test_derived_trait_impls() {
        let comment = RecordingComment::default();
        test_clone_derive(&comment);
        test_debug_derive(&comment);
    }

    #[test]
    fn test_new() {
        let timestamp = current_timestamp();
        let comment = Vec::from(COMMENT_BYTES_1);

        let comment = RecordingComment::new(timestamp, comment);

        assert_eq!(&timestamp, &comment.timestamp);
        assert_eq!(COMMENT_BYTES_1, &comment.comment);
    }

    #[test]
    fn test_comment() {
        let timestamp = current_timestamp();
        let comment = Vec::from(COMMENT_BYTES_1);

        let comment = RecordingComment::new(timestamp, comment);

        let comment_slice = comment.comment();

        assert_eq!(COMMENT_BYTES_1, comment_slice);
    }
}
