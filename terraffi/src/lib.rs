//! Build Rust libraries that expose a **public-facing** C interface via FFI.
//!
//! The goal is not simply to create bindings and types that work across the FFI
//! boundary, but ones which match expected C conventions and produce nicely
//! formatted, human-readable C header files.
//!
//! This crate is a facade that re-exports the runtime half of terraffi:
//!
//! - [`terraffi_ctypes`] — C interop types such as [`CSlice`], [`CVec`],
//!   [`CStringPtr`] and [`CStringBuffer`].
//! - [`terraffi_macro`] — the `#[terraffi_export]` attribute and derive macros
//!   (enabled by the `macros` feature, on by default).
//!
//! Header generation lives in a separate crate, `terraffi_gen`, because it is a
//! **build dependency** rather than a runtime one:
//!
//! ```toml
//! [dependencies]
//! terraffi = "0.1"
//!
//! [build-dependencies]
//! terraffi_gen = "0.1"
//! ```
//!
//! # Example
//!
//! ```
//! #[repr(C)]
//! pub struct ExampleStruct {
//!     pub foo: i32,
//!     pub bar: f32,
//! }
//!
//! #[unsafe(no_mangle)]
//! pub extern "C" fn example_function(value: i32) -> i32 {
//!     value
//! }
//! ```
//!
//! `terraffi_gen` turns the above into:
//!
//! ```c
//! typedef struct example_struct {
//!     int32_t foo;
//!     float bar;
//! } example_struct_t;
//!
//! int32_t example_function(int32_t value);
//! ```
//!
//! # Features
//!
//! | Feature  | Default | Description |
//! |----------|---------|-------------|
//! | `std`    | yes     | Standard library support. Implies `alloc`. |
//! | `alloc`  | yes     | Owning types ([`CVec`], [`CStringBuffer`]). Requires an allocator. |
//! | `macros` | yes     | Re-export the attribute and derive macros. |
//! | `serde`  | no      | `Serialize` / `Deserialize` implementations for the interop types. |
//!
//! Disabling `alloc` targets pure `no_std`. The owning types can then no longer be
//! constructed or dropped, leaving the borrowed and pointer types for interop where
//! the C side owns the memory.
//!
//! [`CSlice`]: terraffi_ctypes::CSlice
//! [`CVec`]: terraffi_ctypes::CVec
//! [`CStringPtr`]: terraffi_ctypes::CStringPtr
//! [`CStringBuffer`]: terraffi_ctypes::CStringBuffer

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub use terraffi_ctypes::*;

#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub use terraffi_macro::*;

/// The underlying C interop types crate.
///
/// Everything here is also re-exported at the root of this crate; this alias
/// exists so generated code and documentation can name the crate explicitly.
pub use terraffi_ctypes as ctypes;

/// The underlying proc-macro crate.
#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub use terraffi_macro as macros;
