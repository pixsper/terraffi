# terraffi

[![Build](https://github.com/pixsper/terraffi/actions/workflows/build.yml/badge.svg)](https://github.com/pixsper/terraffi/actions)
[![Cargo](https://img.shields.io/crates/v/terraffi.svg)](https://crates.io/crates/terraffi/)
[![docs.rs](https://img.shields.io/docsrs/terraffi)](https://docs.rs/terraffi/latest/terraffi/)
[![Rust version: 1.94+](https://img.shields.io/badge/rust%20version-1.94+-orange)](https://blog.rust-lang.org/2026/03/05/Rust-1.94.0/)

Collection of libraries to assist with creating Rust libraries which expose a **public-facing** C interface via FFI. The goal is not to simply 
create bindings and types that work across the FFI boundary, but ones which match expected C conventions and produce nicely formatted, human-readable C header files.

## Libraries
- **terraffi_ctypes** - Provides a number of types specialized for C interop including `CSlice<T>`, `CVec<T>`,  `CStringPtr` and `CStringBuffer`.
- **terraffi_macro** - Proc macro crate for annotating libraries which produce a C interface.
- **terraffi_gen** - Header file generation.
