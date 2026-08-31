//! Verifies that terraffi's macros work for a crate that depends on the
//! `terraffi` facade alone.

use terraffi::{CDefault, CStringBuffer, CStringPtr, CVec, terraffi_export};

/// Exercises `#[derive(CDefault)]` resolving `CDefault` via the facade.
#[derive(CDefault, PartialEq, Debug)]
#[repr(C)]
pub struct FacadeStruct {
    pub value: i32,
    pub ratio: f32,
}

/// Exercises the `#[terraffi_export]` attribute via the facade.
#[terraffi_export]
#[repr(C)]
pub struct FacadeExported {
    pub count: u32,
}

/// Exercises the re-exported interop types via the facade.
///
/// Annotated so the generator must resolve `terraffi::*` paths, not `terraffi_ctypes::*`.
#[terraffi_export]
#[repr(C)]
pub struct FacadeTypes {
    pub name: CStringPtr,
    pub owned: CStringBuffer,
    pub values: CVec<i32>,
}

#[unsafe(no_mangle)]
pub extern "C" fn facade_function(input: i32) -> i32 {
    input
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_c_default_resolves_through_facade() {
        let v = FacadeStruct::c_default();
        assert_eq!(
            v,
            FacadeStruct {
                value: 0,
                ratio: 0.0
            }
        );
        assert!(v.eq_c_default());
    }

    #[test]
    fn non_default_value_is_not_c_default() {
        let v = FacadeStruct {
            value: 7,
            ratio: 1.5,
        };
        assert!(!v.eq_c_default());
    }
}
