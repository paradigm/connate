/// memcpy implementation to satisfy Rust core crate requirements
///
/// Without this, some core functions will fail to compile/link.
#[cfg(not(test))]
#[unsafe(no_mangle)]
unsafe extern "C" fn memcpy(
    dest: *mut core::ffi::c_void,
    src: *const core::ffi::c_void,
    n: usize,
) -> *mut core::ffi::c_void {
    // Safety:
    // - This is satisfying a C API requirement which isn't really safe.
    // - Caller must ensure regions do not overlap and are valid for n bytes
    // - We implement this manually to avoid calling core::ptr::copy_nonoverlapping
    //   which would cause infinite recursion since it calls memcpy internally
    unsafe {
        let mut dest_ptr = dest as *mut u8;
        let mut src_ptr = src as *const u8;
        let mut count = n;

        // Copy 8 bytes at a time for better performance, but only if both pointers are aligned
        let both_aligned =
            (dest_ptr as usize).is_multiple_of(8) && (src_ptr as usize).is_multiple_of(8);
        if both_aligned {
            while count >= 8 {
                core::ptr::write_volatile(
                    dest_ptr as *mut u64,
                    core::ptr::read_volatile(src_ptr as *const u64),
                );
                dest_ptr = dest_ptr.add(8);
                src_ptr = src_ptr.add(8);
                count -= 8;
            }
        }

        // Copy remaining bytes one at a time
        while count > 0 {
            core::ptr::write_volatile(dest_ptr, core::ptr::read_volatile(src_ptr));
            dest_ptr = dest_ptr.add(1);
            src_ptr = src_ptr.add(1);
            count -= 1;
        }
    }
    dest
}
