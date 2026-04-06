#![allow(unused)]

/// A simple struct defined in a dependency crate, used to verify
/// that types from local dependency crates are included in the generated header.
#[repr(C)]
pub struct ExampleDependencyStruct {
    foo: i32,
}
