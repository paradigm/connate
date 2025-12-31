/// Copies elements from `src` to `dst`, disallowing overlaps
///
/// # Safety
/// - `dst` and `src` must not overlap.
pub unsafe fn memcpy<T: Copy, const N: usize>(dst: &mut [T; N], src: &[T; N]) {
    if N == 0 {
        return;
    }

    unsafe {
        core::ptr::copy_nonoverlapping(
            src.as_ptr() as *const u8,
            dst.as_mut_ptr() as *mut u8,
            N * core::mem::size_of::<T>(),
        )
    };
}

/// Copies elements from `src` to `dst`, allowing overlaps
///
/// Unlike memcpy, this handles overlapping memory regions correctly.
pub fn memmove<T: Copy, const N: usize>(dst: &mut [T; N], src: &[T; N]) {
    if N == 0 {
        return;
    }

    unsafe {
        core::ptr::copy(
            src.as_ptr() as *const u8,
            dst.as_mut_ptr() as *mut u8,
            N * core::mem::size_of::<T>(),
        )
    };
}

pub fn memzero(buf: &mut [u8]) {
    unsafe { core::ptr::write_bytes(buf.as_mut_ptr(), 0, buf.len()) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memcpy_u8() {
        let src = [1u8, 2, 3, 4, 5];
        let mut dst = [0u8; 5];
        unsafe {
            memcpy(&mut dst, &src);
        }
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memcpy_u32() {
        let src = [0x12345678u32, 0xABCDEF00, 0xDEADBEEF];
        let mut dst = [0u32; 3];
        unsafe {
            memcpy(&mut dst, &src);
        }
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memcpy_u64() {
        let src = [0x123456789ABCDEF0u64, 0xFEDCBA9876543210];
        let mut dst = [0u64; 2];
        unsafe {
            memcpy(&mut dst, &src);
        }
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memcpy_zero_length() {
        let src: [u8; 0] = [];
        let mut dst: [u8; 0] = [];
        unsafe {
            memcpy(&mut dst, &src);
        }
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memcpy_single_element() {
        let src = [42u8];
        let mut dst = [0u8];
        unsafe {
            memcpy(&mut dst, &src);
        }
        assert_eq!(dst, [42]);
    }

    #[test]
    fn test_memmove_basic() {
        let src = [1u8, 2, 3, 4, 5];
        let mut dst = [0u8; 5];
        memmove(&mut dst, &src);
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memmove_zero_length() {
        let src: [u8; 0] = [];
        let mut dst: [u8; 0] = [];
        memmove(&mut dst, &src);
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memmove_u32() {
        let src = [0x11111111u32, 0x22222222, 0x33333333];
        let mut dst = [0u32; 3];
        memmove(&mut dst, &src);
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memmove_single_element() {
        let src = [99u8];
        let mut dst = [0u8];
        memmove(&mut dst, &src);
        assert_eq!(dst, [99]);
    }

    #[test]
    fn test_memzero_basic() {
        let mut buf = [1u8, 2, 3, 4, 5];
        memzero(&mut buf);
        assert_eq!(buf, [0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_memzero_empty() {
        let mut buf: [u8; 0] = [];
        memzero(&mut buf);
        assert_eq!(buf, []);
    }

    #[test]
    fn test_memzero_single_byte() {
        let mut buf = [255u8];
        memzero(&mut buf);
        assert_eq!(buf, [0]);
    }

    #[test]
    fn test_memzero_large() {
        let mut buf = [0xFFu8; 256];
        memzero(&mut buf);
        assert!(buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_memzero_already_zero() {
        let mut buf = [0u8; 10];
        memzero(&mut buf);
        assert_eq!(buf, [0; 10]);
    }
}
