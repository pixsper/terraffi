// Fixture for slice-in-signature validation. Never compiled - the generator only
// parses these files as text.

use terraffi_ctypes::{CArrayPtr, CSlice, CSliceRef, CStringPtr, CVec};

#[repr(C)]
pub struct Payload {
    pub id: u32,
}

/// Legal: a slice as a struct field expands into adjacent members.
#[repr(C)]
pub struct Holder {
    pub items: CSlice<Payload>,
}

/// Illegal: two words in Rust, one pointer in C.
#[unsafe(no_mangle)]
pub extern "C" fn takes_slice_ref(s: CSliceRef<'static, Payload>) {}

/// Illegal: three words in Rust.
#[unsafe(no_mangle)]
pub extern "C" fn takes_vec(v: CVec<Payload>) {}

/// Illegal in a return position too.
#[unsafe(no_mangle)]
pub extern "C" fn returns_slice() -> CSlice<Payload> {
    CSlice::default()
}

/// Legal: a bare pointer with the length passed alongside it.
#[unsafe(no_mangle)]
pub extern "C" fn takes_ptr_and_len(p: CArrayPtr<Payload>, len: usize) {}

/// Legal: pointer-sized types are unaffected.
#[unsafe(no_mangle)]
pub extern "C" fn takes_string(s: CStringPtr) {}

/// Legal: a pointer to a struct that contains a slice.
#[unsafe(no_mangle)]
pub extern "C" fn takes_holder(h: Option<&Holder>) {}

/// Illegal: a C typedef cannot expand into a pointer and a length.
#[terraffi_export]
pub type AliasedSlice = CSlice<Payload>;

/// Illegal: C would see a pointer to a pointer, not to a pointer/length pair.
#[terraffi_export]
#[repr(C)]
pub struct BehindPointer {
    pub items: Option<&'static CSlice<Payload>>,
}

/// Illegal: each element is two words in Rust and one in C.
#[terraffi_export]
#[repr(C)]
pub struct InsideArray {
    pub items: [CSlice<Payload>; 2],
}

/// Legal: a tagged union variant expands the same way a struct field does.
#[terraffi_export]
#[repr(C, u32)]
pub enum Tagged {
    None = 0,
    Many(CSlice<Payload>) = 1,
}

/// Illegal: Rust's unit type becomes `void`, which C rejects as a field.
#[terraffi_export]
#[repr(C)]
pub struct HasUnitField {
    pub nothing: (),
}

/// Illegal: a tuple has no C representation either, and becomes `void`.
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn takes_a_tuple(pair: (u32, u32)) {}

/// Illegal: `void x[4]` is rejected for the same reason as `void x`.
#[terraffi_export]
#[repr(C)]
pub struct HasVoidArray {
    pub items: [(); 4],
}

/// Legal: `void` is fine as a return type.
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn returns_nothing() {}

/// Legal: a pointer to void is an ordinary C pointer.
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn takes_void_pointer(p: *const core::ffi::c_void) {}
