use std::{
    io,
    ops::{Deref, Index},
    slice::SliceIndex,
};

/// A size-adating buffer to store bytes in. The buffer grows when data is
/// stored into it. The contents can then be consumed which results in
/// the buffer dropping the consumed data before new data are appended.
#[derive(Clone, Debug, Default)]
pub struct BlobBuffer {
    buf: Vec<u8>,
    start: usize,
    offset: usize,
}

impl BlobBuffer {
    /// Constructs a new `BlobBuffer`.
    pub fn new() -> BlobBuffer {
        BlobBuffer::default()
    }

    /// Provide additional data to the internal buffer.
    pub fn extend_from_slice(&mut self, data: &[u8]) {
        if self.start > 0 {
            drop(self.buf.drain(0..self.start));
            self.start = 0;
        }

        self.buf.extend_from_slice(data);
    }

    /// Consume the given amount of data from the internal buffer.
    pub fn consume(&mut self, length: usize) {
        self.start += length;
        self.offset += length;
    }

    /// Returns the unconsumed byte length of the internal buffer.
    pub fn len(&self) -> usize {
        self.buf.len() - self.start
    }

    /// Returns whether the internal buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buf.len() == self.start
    }

    /// Get amount of already consumed bytes.
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl Deref for BlobBuffer {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.buf[self.start..]
    }
}

impl<I> Index<I> for BlobBuffer
where
    I: SliceIndex<[u8]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.deref()[index]
    }
}

impl io::Read for BlobBuffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let src_len = self.len();
        let dst_len = buf.len();
        let len = if src_len < dst_len { src_len } else { dst_len };
        buf[0..len].copy_from_slice(&self[0..len]);
        self.consume(len);
        Ok(len)
    }
}

impl io::Write for BlobBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // nop
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use crate::test_utils::{test_clone_derive, test_debug_derive};

    use super::*;

    #[test]
    fn test_derived_impls() {
        let bb = BlobBuffer::default();
        test_debug_derive(&bb);
        test_clone_derive(&bb);
    }

    #[test]
    fn test() {
        let mut bb = BlobBuffer::new();

        assert_eq!(0, bb.buf.len());
        assert_eq!(0, bb.start);
        assert_eq!(0, bb.offset);
        assert_eq!(0, bb.len());
        assert_eq!(true, bb.is_empty());
        assert_eq!(0, bb.offset());

        bb.extend_from_slice(&[0x00, 0x01, 0x02, 0x03]);

        assert_eq!(4, bb.buf.len());
        assert_eq!(0, bb.start);
        assert_eq!(0, bb.offset);
        assert_eq!(4, bb.len());
        assert_eq!(false, bb.is_empty());
        assert_eq!(&[0x00, 0x01, 0x02, 0x03], &*bb);
        assert_eq!(0, bb.offset());

        bb.consume(2);

        assert_eq!(4, bb.buf.len());
        assert_eq!(2, bb.start);
        assert_eq!(2, bb.offset);
        assert_eq!(2, bb.len());
        assert_eq!(false, bb.is_empty());
        assert_eq!(&[0x02, 0x03], &*bb);
        assert_eq!(2, bb.offset());

        bb.consume(1);

        assert_eq!(4, bb.buf.len());
        assert_eq!(3, bb.start);
        assert_eq!(3, bb.offset);
        assert_eq!(1, bb.len());
        assert_eq!(false, bb.is_empty());
        assert_eq!(3, bb.offset());
        assert_eq!(&[0x03], &*bb);

        bb.extend_from_slice(&[0x04, 0x05, 0x06, 0x07]);

        assert_eq!(5, bb.buf.len());
        assert_eq!(0, bb.start);
        assert_eq!(3, bb.offset);
        assert_eq!(5, bb.len());
        assert_eq!(false, bb.is_empty());
        assert_eq!(3, bb.offset());
        assert_eq!(&[0x03, 0x04, 0x05, 0x06, 0x07], &*bb);

        // Deref trait impl
        assert_eq!(&[0x03, 0x04, 0x05, 0x06, 0x07], &(*bb));

        // Index trait impl
        assert_eq!(0x05, bb[2]);
        assert_eq!(&[0x05, 0x06], &bb[2..4]);

        // Read trait impl
        let mut buf = [0u8; 2];
        assert_eq!(bb.read(&mut buf).expect("No read error"), 2);
        assert_eq!(&[0x03, 0x04], &buf);
        assert_eq!(5, bb.buf.len());
        assert_eq!(2, bb.start);
        assert_eq!(5, bb.offset);
        assert_eq!(3, bb.len());
        assert_eq!(false, bb.is_empty());
        assert_eq!(5, bb.offset());

        assert_eq!(bb.read(&mut buf).expect("No read error"), 2);
        assert_eq!(&[0x05, 0x06], &buf);
        assert_eq!(5, bb.buf.len());
        assert_eq!(4, bb.start);
        assert_eq!(7, bb.offset);
        assert_eq!(1, bb.len());
        assert_eq!(false, bb.is_empty());
        assert_eq!(7, bb.offset());

        // Write trait impl
        assert_eq!(bb.write(&buf).expect("No write error"), 2);
        bb.flush().expect("No flush error");
        assert_eq!(3, bb.buf.len());
        assert_eq!(0, bb.start);
        assert_eq!(7, bb.offset);
        assert_eq!(3, bb.len());
        assert_eq!(false, bb.is_empty());
        assert_eq!(7, bb.offset());
        assert_eq!(&[0x07, 0x05, 0x06], &*bb);
    }
}
