/// Provides information whether a slice of bytes contains a valid blob of data.
#[derive(Debug, PartialEq)]
pub enum StreamBlobLength {
    /// The slice of bytes starts with a valid blob of given size.
    BlobLength(usize),

    /// The slice of bytes may start with a valid blob, but is still incomplete to be certain.
    Partial,

    /// The slice of bytes does not start with a valid blob.
    Malformed,
}
