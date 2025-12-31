use core::ffi::{CStr, c_char};

/// Ergonomic wrapper over ABI standard argv pointer.
pub struct Argv<'a> {
    raw: &'a [*const c_char],
}

impl<'a> Argv<'a> {
    /// # Safety
    /// - `argv..argv+argc` must be valid for reads during `'a`.
    /// - Ideally immediately use ABI-provided argc+argv in main()
    pub const unsafe fn from_raw(argc: isize, argv: *const *const c_char) -> Self {
        Self {
            raw: unsafe { core::slice::from_raw_parts(argv, argc as usize) },
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.raw.len()
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    // TODO: const once
    // https://github.com/rust-lang/rust/issues/137737
    // merges
    #[inline]
    pub fn first(&self) -> Option<&'a CStr> {
        self.raw.first().map(|&p| unsafe { CStr::from_ptr(p) })
    }

    // TODO: const once
    // https://github.com/rust-lang/rust/issues/137737
    // merges
    #[inline]
    pub fn get(&self, i: usize) -> Option<&'a CStr> {
        self.raw.get(i).map(|&p| unsafe { CStr::from_ptr(p) })
    }

    // TODO: const once
    // https://github.com/rust-lang/rust/issues/137737
    // merges
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &'a CStr> + 'a {
        self.raw.iter().map(|&p| unsafe { CStr::from_ptr(p) })
    }

    #[inline]
    pub fn pop(&mut self) -> Option<&'a CStr> {
        let (&head, tail) = self.raw.split_first()?;
        // SAFETY: caller guaranteed argv pointers are valid for reads during 'a.
        let head_cstr = unsafe { CStr::from_ptr(head) };
        self.raw = tail;
        Some(head_cstr)
    }
}
