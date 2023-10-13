pub fn i16_from_le_bytes(bytes: &[u8]) -> i16 {
    i16::from_le_bytes(bytes [0..2].try_into().unwrap())
}

pub fn i16_to_le_bytes(bytes: &mut [u8], value: i16) {
    bytes [0..2].copy_from_slice(&value.to_le_bytes())
}

pub fn u16_from_le_bytes(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes [0..2].try_into().unwrap())
}

pub fn u16_to_le_bytes(bytes: &mut [u8], value: u16) {
    bytes [0..2].copy_from_slice(&value.to_le_bytes())
}

pub fn i32_from_le_bytes(bytes: &[u8]) -> i32 {
    i32::from_le_bytes(bytes [0..4].try_into().unwrap())
}

pub fn i32_to_le_bytes(bytes: &mut [u8], value: i32) {
    bytes [0..4].copy_from_slice(&value.to_le_bytes())
}

pub fn i64_from_le_bytes(bytes: &[u8]) -> i64 {
    i64::from_le_bytes(bytes [0..8].try_into().unwrap())
}

pub fn i64_to_le_bytes(bytes: &mut [u8], value: i64) {
    bytes [0..8].copy_from_slice(&value.to_le_bytes())
}
