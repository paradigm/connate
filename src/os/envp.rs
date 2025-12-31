use core::ffi::{CStr, c_char};
use core::marker::PhantomData;

#[derive(Clone)]
pub struct Envp<'a> {
    cur: *const *const c_char, // null-terminated
    _pd: PhantomData<&'a CStr>,
}

impl<'a> Envp<'a> {
    /// # Safety
    /// - `envp` must be a valid null-terminated array of pointers that remain valid for the
    ///   duration of `'a`.
    /// - Ideally immediately use ABI-provided envp in main()
    #[inline]
    pub unsafe fn from_raw(envp: *const *const c_char) -> Self {
        Self {
            cur: envp,
            _pd: PhantomData,
        }
    }
}

impl<'a> Iterator for Envp<'a> {
    type Item = (&'a [u8], &'a CStr);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Grab the first item and increment tracking pointer
        //
        // Safety:
        // - self.cur points into ABI-defined envp array, which is valid and null-terminated
        let p = unsafe { *self.cur };

        // If the dereferenced pointer is null, we've hit the end of the envp array
        if p.is_null() {
            return None;
        }

        self.cur = unsafe { self.cur.add(1) };

        // Split at `=`
        let bytes = unsafe { CStr::from_ptr(p) }.to_bytes();
        if let Some(eq_idx) = bytes.iter().position(|&c| c == b'=') {
            let var: &'a [u8] = unsafe { bytes.get_unchecked(..eq_idx) };
            let val_ptr = unsafe { p.add(eq_idx).add(1) };
            let val: &'a CStr = unsafe { CStr::from_ptr(val_ptr) };
            Some((var, val))
        } else {
            Some((bytes, c""))
        }
    }
}
