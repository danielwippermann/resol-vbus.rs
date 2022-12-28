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

#[cfg(test)]
mod tests {
    use crate::test_utils::{test_debug_derive, test_partial_eq_derive};

    use super::*;

    #[test]
    fn test_derived_impl() {
        let sbl = StreamBlobLength::BlobLength(0);
        test_debug_derive(&sbl);
        test_partial_eq_derive(&sbl);
    }
}
