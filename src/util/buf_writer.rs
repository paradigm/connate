//! Safe, panic-free buffer writer with offset tracking

use crate::err::Errno;

pub struct BufWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> BufWriter<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Push bytes onto the buffer, returning an error if it would overflow
    pub fn push(&mut self, data: &[u8]) -> Result<(), Errno> {
        let end = self.pos.checked_add(data.len()).ok_or(Errno::EOVERFLOW)?;

        if end > self.buf.len() {
            return Err(Errno::EOVERFLOW);
        }

        // Use unsafe pointer copy to avoid panic branch from copy_from_slice's length assertion.
        //
        // # SAFETY
        //
        // - We've verified that self.pos + data.len() <= self.buf.len(), and thus we know the
        // destination slice has exactly data.len() bytes available.
        // - Rust type system ensures both source and destination are valid for data.len() bytes
        // - Rust type system ensures they don't overlap (destination is a mutable slice we own).
        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.buf.as_mut_ptr().add(self.pos),
                data.len(),
            )
        };
        self.pos = end;
        Ok(())
    }

    /// Get the current write position
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Reset the write position to 0
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    /// Get a reference to the populated segment of the buffer
    pub fn as_slice(&self) -> &[u8] {
        // # SAFETY
        //
        // We've been actively tracking pos relative to buffer size to ensure this works.
        unsafe { self.buf.get_unchecked(0..self.pos) }
    }
}
