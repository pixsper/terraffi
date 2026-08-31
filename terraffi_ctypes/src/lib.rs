//! Types specialised for C interop across an FFI boundary.
//!
//! Each type here is `#[repr(C)]` or `#[repr(transparent)]` and has a layout C can
//! rely on: owning buffers ([`CVec`], [`CSlice`], [`CStringBuffer`]), borrowed views
//! ([`CSliceRef`], [`CStringPtr`], [`CArrayPtr`]) and opaque handles ([`CHandle`]).
//!
//! Pointer-sized types keep that size when wrapped in [`Option`], so
//! `Option<CStringPtr>` can be passed where C expects a possibly-null `char*`.
//!
//! # Features
//!
//! `std` (default) implies `alloc`. Without `alloc` the owning types can be received
//! from C but not constructed or dropped, leaving the borrowed and pointer types for
//! interop where the C side owns the memory. `serde` adds `Serialize` for the
//! interop types, and `Deserialize` where `alloc` is also enabled.

#![no_std]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

/// A handle did not hold the pointer state an operation required.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PtrError {
    /// The handle itself was null, so it has not been initialised.
    NullHandle,
    /// The handle's target was expected to be null, but held a value.
    HandleTargetNonNull,
    /// The handle's target was expected to hold a value, but was null.
    HandleTargetNull,
}

impl core::fmt::Display for PtrError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PtrError::NullHandle => f.write_str("expected a handle, found a null pointer"),
            PtrError::HandleTargetNonNull => {
                f.write_str("expected the handle target to be null, found a non-null pointer")
            }
            PtrError::HandleTargetNull => {
                f.write_str("expected the handle target to be non-null, found a null pointer")
            }
        }
    }
}

impl core::error::Error for PtrError {}

mod arrays;
mod ptr;
mod slice;
mod strings;
mod vec;

pub use arrays::*;
pub use ptr::*;
pub use slice::*;
pub use strings::*;
pub use vec::*;

/// Trait which can be implemented on `repr(C)` types to provide a zeroed value, which matches the default
/// representation of the type in C.
///
/// # Safety
///
/// Should only be implemented on `repr(C)` types or where it can be guaranteed that `mem::zeroed()` is a valid
/// representation of the type.
pub unsafe trait CDefault: Sized {
    /// Returns the all-zero value of this type, matching how C zero-initialises it.
    fn c_default() -> Self {
        unsafe { core::mem::zeroed() }
    }

    /// Returns `true` if this value equals [`CDefault::c_default`].
    fn eq_c_default(&self) -> bool;
}

/// Tests for the subset of the API available without an allocator.
///
/// Every other test module in this crate is gated on `alloc`, so these are the only
/// tests that run in a bare `no_std` build.
#[cfg(test)]
mod no_alloc_tests {
    use super::*;
    use core::ffi::{CStr, c_char};
    use core::fmt::Write;
    use core::ptr::NonNull;

    /// A `core::fmt::Write` sink over a fixed stack buffer, so formatting can be
    /// exercised with no allocator present.
    struct StackBuf {
        buf: [u8; 64],
        len: usize,
    }

    impl StackBuf {
        fn new() -> Self {
            Self {
                buf: [0; 64],
                len: 0,
            }
        }

        fn as_str(&self) -> &str {
            core::str::from_utf8(&self.buf[..self.len]).expect("sink holds valid UTF-8")
        }
    }

    impl Write for StackBuf {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            let end = self.len + bytes.len();
            if end > self.buf.len() {
                return Err(core::fmt::Error);
            }
            self.buf[self.len..end].copy_from_slice(bytes);
            self.len = end;
            Ok(())
        }
    }

    fn as_ref(cstr: &CStr) -> CStringPtrRef<'_> {
        let ptr = NonNull::new(cstr.as_ptr() as *mut c_char).expect("CStr is never null");
        // SAFETY: `cstr` is a valid null-terminated C string living as long as the borrow.
        unsafe { CStringPtrRef::from_ptr(ptr) }
    }

    #[test]
    fn pointer_types_keep_their_layout_without_alloc() {
        assert_eq!(size_of::<CStringPtr>(), size_of::<*const c_char>());
        assert_eq!(size_of::<Option<CStringPtr>>(), size_of::<*const c_char>());
        assert_eq!(size_of::<CStringPtrRef<'_>>(), size_of::<*const c_char>());
        assert_eq!(
            size_of::<Option<CStringPtrRef<'_>>>(),
            size_of::<*const c_char>()
        );
    }

    #[test]
    fn display_needs_no_allocator() {
        let mut out = StackBuf::new();
        write!(out, "{}", as_ref(c"hello")).expect("formatting fits the buffer");
        assert_eq!(out.as_str(), "hello");
    }

    #[test]
    fn display_replaces_invalid_utf8_without_allocator() {
        // 0xff is never valid UTF-8 and must render as U+FFFD.
        let mut out = StackBuf::new();
        write!(out, "{}", as_ref(c"a\xffb")).expect("formatting fits the buffer");
        assert_eq!(out.as_str(), "a\u{fffd}b");
    }

    #[test]
    fn borrowed_accessors_need_no_allocator() {
        let r = as_ref(c"terraffi");
        assert_eq!(r.as_bytes(), b"terraffi");
        assert_eq!(r.as_bytes_with_nul(), b"terraffi\0");
        assert_eq!(r.as_c_str(), c"terraffi");
    }
}
