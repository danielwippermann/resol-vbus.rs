use std::{collections::hash_map::DefaultHasher, hash::Hasher};

/// A trait to generate an identification hash for any of the VBus data types.
pub trait IdHash {
    /// Creates an identification hash for this VBus data value.
    fn id_hash<H: Hasher>(&self, h: &mut H);
}

/// Calculate the identification hash for a given VBus data value.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::{Header, id_hash};
/// use resol_vbus::utils::utc_timestamp;
///
/// let header = Header {
///     timestamp: utc_timestamp(1485688933),
///     channel: 0x11,
///     destination_address: 0x1213,
///     source_address: 0x1415,
///     protocol_version: 0x16,
/// };
///
/// assert_eq!(8369676560183260683, id_hash(&header));
/// ```
pub fn id_hash<H: IdHash>(h: &H) -> u64 {
    let mut hasher = DefaultHasher::new();
    h.id_hash(&mut hasher);
    hasher.finish()
}
