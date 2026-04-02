#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

mod arrays;
mod strings;

pub use arrays::*;
pub use strings::*;
