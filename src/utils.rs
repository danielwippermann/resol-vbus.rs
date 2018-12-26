//! A module containing utitlities functions for processing VBus data.
use chrono::{DateTime, TimeZone, Utc};

/// Calc checksum according to VBus protocol version x.0.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::utils::calc_checksum_v0;
///
/// assert_eq!(0x7F, calc_checksum_v0(&[]));
/// assert_eq!(0x00, calc_checksum_v0(&[ 0x7F ]));
/// assert_eq!(0x01, calc_checksum_v0(&[ 0x7F, 0x7F ]));
/// assert_eq!(0x34, calc_checksum_v0(&[ 0x10, 0x00, 0x11, 0x7E, 0x10, 0x00, 0x01, 0x1B ]));
/// ```
pub fn calc_checksum_v0(buf: &[u8]) -> u8 {
    buf.iter().fold(0x7F, |acc, &x| (0x80 + acc - x) & 0x7F)
}

/// Calc and compare checksum according to VBus protocol version x.0.
///
/// This function calculates the checksum over all but the last bytes in the provided slice and
/// compares the calculated checksum with the last byte in the slice, returning the result of the
/// comparison.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::utils::calc_and_compare_checksum_v0;
///
/// assert_eq!(true, calc_and_compare_checksum_v0(&[ 0x10, 0x00, 0x11, 0x7E, 0x10, 0x00, 0x01, 0x1B, 0x34 ]));
/// assert_eq!(false, calc_and_compare_checksum_v0(&[ 0x10, 0x00, 0x11, 0x7E, 0x10, 0x00, 0x01, 0x1B, 0x00 ]));
/// ```
pub fn calc_and_compare_checksum_v0(buf: &[u8]) -> bool {
    let idx = buf.len() - 1;
    calc_checksum_v0(&buf[0..idx]) == buf[idx]
}

/// Calc and set checksum according to VBus protocol version x.0.
///
/// This function calculates the checksum over all but the last bytes in the provided slice and
/// compares the calculated checksum with the last byte in the slice.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::utils::calc_and_set_checksum_v0;
///
/// let mut buf = [ 0x10, 0x00, 0x11, 0x7E, 0x10, 0x00, 0x01, 0x1B, 0x00 ];
/// calc_and_set_checksum_v0(&mut buf [..]);
/// assert_eq!(0x34, buf [8]);
/// ```
pub fn calc_and_set_checksum_v0(buf: &mut [u8]) {
    let idx = buf.len() - 1;
    buf[idx] = calc_checksum_v0(&buf[0..idx]);
}

/// Copy bytes from `src` to `dst`, extracting the MSB into a separate byte and appending it at the end of `dst`.
///
/// # Panics
///
/// The function panics if the `dst` slice is not exactly one byte longer than the `src` slice.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::utils::copy_bytes_extracting_septett;
///
/// let src = &[ 0x07, 0x01, 0x4c, 0x00, 0x82, 0x01, 0xff, 0x00, 0xb8, 0x22, 0xf6, 0x00, 0x00, 0x00, 0x00, 0x00 ];
/// let mut dst = [0u8; 20];
///
/// copy_bytes_extracting_septett(&mut dst [0..5], &src [0..4]);
/// copy_bytes_extracting_septett(&mut dst [5..10], &src [4..8]);
/// copy_bytes_extracting_septett(&mut dst [10..15], &src [8..12]);
/// copy_bytes_extracting_septett(&mut dst [15..20], &src [12..16]);
///
/// assert_eq!(&[ 0x07, 0x01, 0x4c, 0x00, 0x00, 0x02, 0x01, 0x7f, 0x00, 0x05, 0x38, 0x22, 0x76, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00 ], &dst);
/// ```
pub fn copy_bytes_extracting_septett(dst: &mut [u8], src: &[u8]) {
    let septett_idx = src.len();
    if dst.len() != septett_idx + 1 {
        panic!("Destination must be one byte larger than source");
    }

    dst[septett_idx] = src.iter().enumerate().fold(0u8, |acc, (idx, &b)| {
        dst[idx] = b & 0x7F;
        let mask = if b >= 0x80 { 1 << idx } else { 0 };
        acc | mask
    });
}

/// Copy bytes from `src` to `dst`, injecting the MSBs stored in a separate byte at the end of `src`.
///
/// # Panics
///
/// The function panics if the `src` slice is not exactly one byte longer than the `dst` slice.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::utils::copy_bytes_injecting_septett;
///
/// let src = &[ 0x07, 0x01, 0x4c, 0x00, 0x00, 0x02, 0x01, 0x7f, 0x00, 0x05, 0x38, 0x22, 0x76, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00 ];
/// let mut dst = [0u8; 16];
///
/// copy_bytes_injecting_septett(&mut dst [0..4], &src [0..5]);
/// copy_bytes_injecting_septett(&mut dst [4..8], &src [5..10]);
/// copy_bytes_injecting_septett(&mut dst [8..12], &src [10..15]);
/// copy_bytes_injecting_septett(&mut dst [12..16], &src [15..20]);
///
/// assert_eq!(&[ 0x07, 0x01, 0x4c, 0x00, 0x82, 0x01, 0xff, 0x00, 0xb8, 0x22, 0xf6, 0x00, 0x00, 0x00, 0x00, 0x00 ], &dst);
/// ```
pub fn copy_bytes_injecting_septett(dst: &mut [u8], src: &[u8]) {
    let septett_idx = dst.len();
    if src.len() != septett_idx + 1 {
        panic!("Source must be one byte larger than destination");
    }

    let septett = src[septett_idx];
    for (idx, dst_b) in dst.iter_mut().enumerate() {
        let b = src[idx];
        let mask = if (septett & (1 << idx)) != 0 {
            0x80
        } else {
            0x00
        };
        *dst_b = b | mask;
    }
}

/// Checks a slice of bytes whether one of them has its MSB set.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::utils::has_msb_set;
///
/// let bytes = &[ 0x00, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80 ];
///
/// assert_eq!(false, has_msb_set(&bytes [0..8]));
/// assert_eq!(true, has_msb_set(&bytes [0..9]));
/// ```
pub fn has_msb_set(buf: &[u8]) -> bool {
    buf.iter().any(|b| b & 0x80 != 0)
}

const CRC16_TABLE: &[u16] = &[
    0x0000, 0x1189, 0x2312, 0x329B, 0x4624, 0x57AD, 0x6536, 0x74BF, 0x8C48, 0x9DC1, 0xAF5A, 0xBED3,
    0xCA6C, 0xDBE5, 0xE97E, 0xF8F7, 0x1081, 0x0108, 0x3393, 0x221A, 0x56A5, 0x472C, 0x75B7, 0x643E,
    0x9CC9, 0x8D40, 0xBFDB, 0xAE52, 0xDAED, 0xCB64, 0xF9FF, 0xE876, 0x2102, 0x308B, 0x0210, 0x1399,
    0x6726, 0x76AF, 0x4434, 0x55BD, 0xAD4A, 0xBCC3, 0x8E58, 0x9FD1, 0xEB6E, 0xFAE7, 0xC87C, 0xD9F5,
    0x3183, 0x200A, 0x1291, 0x0318, 0x77A7, 0x662E, 0x54B5, 0x453C, 0xBDCB, 0xAC42, 0x9ED9, 0x8F50,
    0xFBEF, 0xEA66, 0xD8FD, 0xC974, 0x4204, 0x538D, 0x6116, 0x709F, 0x0420, 0x15A9, 0x2732, 0x36BB,
    0xCE4C, 0xDFC5, 0xED5E, 0xFCD7, 0x8868, 0x99E1, 0xAB7A, 0xBAF3, 0x5285, 0x430C, 0x7197, 0x601E,
    0x14A1, 0x0528, 0x37B3, 0x263A, 0xDECD, 0xCF44, 0xFDDF, 0xEC56, 0x98E9, 0x8960, 0xBBFB, 0xAA72,
    0x6306, 0x728F, 0x4014, 0x519D, 0x2522, 0x34AB, 0x0630, 0x17B9, 0xEF4E, 0xFEC7, 0xCC5C, 0xDDD5,
    0xA96A, 0xB8E3, 0x8A78, 0x9BF1, 0x7387, 0x620E, 0x5095, 0x411C, 0x35A3, 0x242A, 0x16B1, 0x0738,
    0xFFCF, 0xEE46, 0xDCDD, 0xCD54, 0xB9EB, 0xA862, 0x9AF9, 0x8B70, 0x8408, 0x9581, 0xA71A, 0xB693,
    0xC22C, 0xD3A5, 0xE13E, 0xF0B7, 0x0840, 0x19C9, 0x2B52, 0x3ADB, 0x4E64, 0x5FED, 0x6D76, 0x7CFF,
    0x9489, 0x8500, 0xB79B, 0xA612, 0xD2AD, 0xC324, 0xF1BF, 0xE036, 0x18C1, 0x0948, 0x3BD3, 0x2A5A,
    0x5EE5, 0x4F6C, 0x7DF7, 0x6C7E, 0xA50A, 0xB483, 0x8618, 0x9791, 0xE32E, 0xF2A7, 0xC03C, 0xD1B5,
    0x2942, 0x38CB, 0x0A50, 0x1BD9, 0x6F66, 0x7EEF, 0x4C74, 0x5DFD, 0xB58B, 0xA402, 0x9699, 0x8710,
    0xF3AF, 0xE226, 0xD0BD, 0xC134, 0x39C3, 0x284A, 0x1AD1, 0x0B58, 0x7FE7, 0x6E6E, 0x5CF5, 0x4D7C,
    0xC60C, 0xD785, 0xE51E, 0xF497, 0x8028, 0x91A1, 0xA33A, 0xB2B3, 0x4A44, 0x5BCD, 0x6956, 0x78DF,
    0x0C60, 0x1DE9, 0x2F72, 0x3EFB, 0xD68D, 0xC704, 0xF59F, 0xE416, 0x90A9, 0x8120, 0xB3BB, 0xA232,
    0x5AC5, 0x4B4C, 0x79D7, 0x685E, 0x1CE1, 0x0D68, 0x3FF3, 0x2E7A, 0xE70E, 0xF687, 0xC41C, 0xD595,
    0xA12A, 0xB0A3, 0x8238, 0x93B1, 0x6B46, 0x7ACF, 0x4854, 0x59DD, 0x2D62, 0x3CEB, 0x0E70, 0x1FF9,
    0xF78F, 0xE606, 0xD49D, 0xC514, 0xB1AB, 0xA022, 0x92B9, 0x8330, 0x7BC7, 0x6A4E, 0x58D5, 0x495C,
    0x3DE3, 0x2C6A, 0x1EF1, 0x0F78,
];

/// Calculate the CRC16 checksum over a slice of bytes.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::utils::calc_crc16;
///
/// assert_eq!(0x0000, calc_crc16(&[]));
/// assert_eq!(0xF078, calc_crc16(&[ 0x00 ]));
/// ```
pub fn calc_crc16(buf: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for byte in buf {
        crc = (crc >> 8) ^ CRC16_TABLE[(crc ^ u16::from(*byte)) as usize & 0xFF];
    }
    crc ^ 0xFFFF
}

/// Return a Utc timestamp for the given UNIX epoch seconds.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::utils::utc_timestamp;
///
/// assert_eq!("2017-01-29 11:22:13 UTC", utc_timestamp(1485688933).to_string());
/// ```
pub fn utc_timestamp(secs: i64) -> DateTime<Utc> {
    Utc.timestamp(secs, 0)
}
