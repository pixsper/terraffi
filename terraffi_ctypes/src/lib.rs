#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PtrError {
    NullHandle,
    HandleTargetNonNull,
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
