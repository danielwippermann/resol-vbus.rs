pub fn to_hex_string(buf: &[u8]) -> String {
    buf.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().concat()
}


#[test]
fn test_to_hex_string() {
    assert_eq!("", to_hex_string(&[]));
    assert_eq!("01234567", to_hex_string(&[0x01, 0x23, 0x45, 0x67]));
}
