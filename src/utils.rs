/// Calc checksum according to VBus protocol version x.0.
pub fn calc_checksum_v0(buf: &[u8]) -> u8 {
    buf.iter().fold(0x7F, |acc, &x| (0x80 + acc - x) & 0x7F)
}


/// Calc and compare checksum according to VBus protocol version x.0.
pub fn calc_and_compare_checksum_v0(buf: &[u8]) -> bool {
    let idx = buf.len() - 1;
    calc_checksum_v0(&buf [0..idx]) == buf [idx]
}


/// Calc and set checksum according to VBus protocol version x.0.
pub fn calc_and_set_checksum_v0(buf: &mut [u8]) {
    let idx = buf.len() - 1;
    buf [idx] = calc_checksum_v0(&buf [0..idx]);
}


/// Copy bytes from `src` to `dst`, extracting the MSB into a separate byte and appending it at the end of `dst`.
pub fn copy_bytes_extracting_septett(dst: &mut [u8], src: &[u8]) {
    let septett_idx = src.len();
    if dst.len() != septett_idx + 1 {
        panic!("Destination must be one byte larger than source");
    }

    dst [septett_idx] = src.iter().enumerate().fold(0u8, |acc, (idx, &b)| {
        dst [idx] = b & 0x7F;
        let mask = if b >= 0x80 { 1 << idx } else { 0 };
        acc | mask
    });
}


/// Copy bytes from `src` to `dst`, injecting the MSBs stored in a separate byte at the end of `src`.
pub fn copy_bytes_injecting_septett(dst: &mut [u8], src: &[u8]) {
    let septett_idx = dst.len();
    if src.len() != septett_idx + 1 {
        panic!("Source must be one byte larger than destination");
    }

    let septett = src [septett_idx];
    for (idx, dst_b) in dst.iter_mut().enumerate() {
        let b = src [idx];
        let mask = if (septett & (1 << idx)) != 0 { 0x80 } else { 0x00 };
        *dst_b = b | mask;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_checksum_v0() {
        assert_eq!(0x7F, calc_checksum_v0(&[]));
        assert_eq!(0x00, calc_checksum_v0(&[ 0x7F ]));
        assert_eq!(0x01, calc_checksum_v0(&[ 0x7F, 0x7F ]));
        assert_eq!(0x34, calc_checksum_v0(&[ 0x10, 0x00, 0x11, 0x7E, 0x10, 0x00, 0x01, 0x1B ]));
    }

    #[test]
    fn test_calc_and_compare_checksum_v0() {
        assert_eq!(true, calc_and_compare_checksum_v0(&[ 0x10, 0x00, 0x11, 0x7E, 0x10, 0x00, 0x01, 0x1B, 0x34 ]));
        assert_eq!(false, calc_and_compare_checksum_v0(&[ 0x10, 0x00, 0x11, 0x7E, 0x10, 0x00, 0x01, 0x1B, 0x00 ]));
    }

    #[test]
    fn test_calc_and_set_checksum_v0() {
        let mut buf = [ 0x10, 0x00, 0x11, 0x7E, 0x10, 0x00, 0x01, 0x1B, 0x00 ];
        calc_and_set_checksum_v0(&mut buf [..]);
        assert_eq!(0x34, buf [8]);
    }

    #[test]
    fn test_copy_bytes_extracting_septett() {
        let src = &[ 0x07, 0x01, 0x4c, 0x00, 0x82, 0x01, 0xff, 0x00, 0xb8, 0x22, 0xf6, 0x00, 0x00, 0x00, 0x00, 0x00 ];
        let mut dst = [0u8; 20];

        copy_bytes_extracting_septett(&mut dst [0..5], &src [0..4]);
        copy_bytes_extracting_septett(&mut dst [5..10], &src [4..8]);
        copy_bytes_extracting_septett(&mut dst [10..15], &src [8..12]);
        copy_bytes_extracting_septett(&mut dst [15..20], &src [12..16]);

        assert_eq!(&[ 0x07, 0x01, 0x4c, 0x00, 0x00, 0x02, 0x01, 0x7f, 0x00, 0x05, 0x38, 0x22, 0x76, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00 ], &dst);
    }

    #[test]
    fn test_copy_bytes_injecting_septett() {
        let src = &[ 0x07, 0x01, 0x4c, 0x00, 0x00, 0x02, 0x01, 0x7f, 0x00, 0x05, 0x38, 0x22, 0x76, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00 ];
        let mut dst = [0u8; 16];

        copy_bytes_injecting_septett(&mut dst [0..4], &src [0..5]);
        copy_bytes_injecting_septett(&mut dst [4..8], &src [5..10]);
        copy_bytes_injecting_septett(&mut dst [8..12], &src [10..15]);
        copy_bytes_injecting_septett(&mut dst [12..16], &src [15..20]);

        assert_eq!(&[ 0x07, 0x01, 0x4c, 0x00, 0x82, 0x01, 0xff, 0x00, 0xb8, 0x22, 0xf6, 0x00, 0x00, 0x00, 0x00, 0x00 ], &dst);
    }
}
