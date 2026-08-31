// Fixture for unresolved-type validation. Never compiled - the generator only
// parses these files as text, so the non-FFI-safe types here are deliberate.

use std::collections::HashMap;

/// Not `#[repr(C)]` and only used behind a pointer, so this is a legitimate
/// opaque handle and must NOT be reported as unresolved.
pub struct OpaqueHandle {
    _private: i32,
}

/// Declared in a hand-written header the user supplies via `add_include`.
/// Indistinguishable from a typo without `assume_declared`.
#[repr(C)]
pub struct FromExternalHeader {
    pub value: i32,
}

#[repr(C)]
pub struct BadFields {
    pub owned: String,
    pub list: Vec<u8>,
    pub map: HashMap<u32, u32>,
    pub borrowed: &'static str,
}

#[unsafe(no_mangle)]
pub extern "C" fn takes_string(s: String) -> i32 {
    let _ = s;
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn takes_opaque(p: *const OpaqueHandle) -> i32 {
    let _ = p;
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn takes_typo(p: *const NoSuchType) -> i32 {
    let _ = p;
    0
}
