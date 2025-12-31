use core::ffi::c_char;

/// Implement the `strlen` symbol expected by `core::ffi::CStr` in `no_std` builds.
#[cfg(not(test))]
#[unsafe(no_mangle)]
unsafe extern "C" fn strlen(s: *const c_char) -> usize {
    if s.is_null() {
        return 0;
    }

    let mut len = 0usize;
    // SAFETY: Caller guarantees `s` points to a valid C string.
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }
    len
}
