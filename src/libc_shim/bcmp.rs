/// bcmp implementation to satisfy Rust core crate requirements
///
/// Without this, some core functions will fail to compile/link.
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(
    a: *const core::ffi::c_void,
    b: *const core::ffi::c_void,
    n: usize,
) -> i32 {
    if n == 0 {
        return 0;
    }

    // Manual implementation to avoid calling memcmp which could cause recursion
    // bcmp only needs to return 0 if equal, non-zero if different
    unsafe {
        let mut a_ptr = a as *const u8;
        let mut b_ptr = b as *const u8;
        let mut count = n;

        // Compare 8 bytes at a time for better performance if both pointers are aligned
        if (a_ptr as usize).is_multiple_of(8) && (b_ptr as usize).is_multiple_of(8) {
            while count >= 8 {
                let a_val = core::ptr::read_volatile(a_ptr as *const u64);
                let b_val = core::ptr::read_volatile(b_ptr as *const u64);
                if a_val != b_val {
                    return 1;
                }
                a_ptr = a_ptr.add(8);
                b_ptr = b_ptr.add(8);
                count -= 8;
            }
        }

        // Compare remaining bytes one at a time
        while count > 0 {
            let a_byte = core::ptr::read_volatile(a_ptr);
            let b_byte = core::ptr::read_volatile(b_ptr);
            if a_byte != b_byte {
                return 1;
            }
            a_ptr = a_ptr.add(1);
            b_ptr = b_ptr.add(1);
            count -= 1;
        }

        0
    }
}
