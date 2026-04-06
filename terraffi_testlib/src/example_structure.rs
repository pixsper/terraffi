use bitflags::bitflags;
use serde::Deserialize;
use serde::Serialize;
use terraffi_ctypes::{CSlice, CStringPtr};
use terraffi_macro::{DiscriminantEnum, terraffi, terraffi_export};
use terraffi_testdeplib::ExampleDependencyStruct;

/// An opaque struct
pub struct OpaqueStruct;

/// An opaque struct which should not appear in the header as it is unreferenced
pub struct UnreferencedOpaqueStruct;

/// A structure demonstrating various field types supported by terraffi,
/// including primitives, enums, nullable strings, slices, and types from
/// dependency crates.
#[repr(C)]
pub struct ExampleStructure {
    /// A single-precision floating point value.
    pub float_member: f32,
    /// A 32-bit signed integer value.
    pub int_member: i32,
    /// An enum member demonstrating enum field support.
    pub enum_member: ExampleEnum,
    /// An optional owned C string pointer, nullable in the generated header.
    pub string_pointer_member: Option<CStringPtr>,
    /// A slice of integers, expanded to a length + pointer pair in C.
    pub array_member: CSlice<i32>,
    /// A struct from a dependency crate.
    pub struct_member: ExampleDependencyStruct,
    /// A slice member
    pub slice_member: [u8; 16],
}

#[terraffi_export]
#[repr(transparent)]
#[derive(Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExampleSliceType([u8; 16]);

/// A simple C-compatible enum with unit variants and no explicit discriminant values.
#[repr(C)]
pub enum ExampleEnum {
    /// Represents the absence of a value.
    None,
    /// First variant.
    Foo,
    /// Second variant.
    Bar,
}

/// A C-compatible enum with explicitly assigned discriminant values,
/// including gaps in the numbering.
#[repr(C)]
pub enum ExampleEnumWithValues {
    /// Default variant with value 0.
    None = 0,
    /// Variant with an explicit value of 10.
    Foo = 10,
    /// Variant with an auto-incremented value.
    Bar,
    /// Variant with a large explicit value.
    Baz = 2544,
}

/// A struct only used inside a discriminated union variant, not referenced by any function.
#[repr(C)]
pub struct UnionOnlyStruct {
    /// An x coordinate.
    pub x: f32,
    /// A y coordinate.
    pub y: f32,
}

/// A discriminated (tagged) union demonstrating variants that carry associated data.
/// The `DiscriminantEnum` derive generates a companion `ExampleDiscriminatedEnumKind`
/// enum containing only the discriminant tags.
#[derive(DiscriminantEnum)]
#[terraffi(additional_derives = derive(Serialize, Deserialize))]
#[repr(C)]
pub enum ExampleDiscriminatedEnum {
    /// Empty variant with no associated data.
    None,
    /// Variant carrying a single unsigned 32-bit integer.
    Foo(u32),
    /// Variant carrying an enum value.
    Bar(ExampleEnum),
    /// Variant carrying a full structure.
    Baz(ExampleStructure),
    /// Variant carrying a struct only used in this union.
    Qux(UnionOnlyStruct),
}

/// A discriminated union with explicit discriminant values and a fixed underlying
/// representation of `u32`. Demonstrates that variant values are preserved in the
/// generated C kind enum.
#[repr(C, u32)]
#[terraffi(discriminant_enum_name = "ExampleDiscriminatedEnumKind")]
pub enum ExampleDiscriminatedEnumWithValues {
    /// Empty variant with value 0.
    None = 0,
    /// Variant carrying a `u32`, with value 10.
    Foo(u32) = 10,
    /// Variant carrying an enum, with an auto-incremented value.
    Bar(ExampleEnum),
    /// Variant carrying a structure, with value 2544.
    Baz(ExampleStructure) = 2544,
}

bitflags! {
    /// A set of bitflags demonstrating `bitflags!` macro support in terraffi.
    /// Emitted as a typedef with `#define` constants in the generated C header.
    #[repr(transparent)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct ExampleFlags: u32 {
        /// First flag (bit 0).
        const Foo = 0b00000001;
        /// Second flag (bit 1).
        const Bar = 0b00000010;
        /// Third flag (bit 2).
        const Baz = 0b00000100;

        // The source may set any bits
        const _ = !0;
    }
}

/// Accepts an enum by value.
///
/// # Parameters
/// - `v`: The enum value to process.
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn param_enum(v: ExampleEnumWithValues) {}

/// Accepts a const pointer to a structure.
///
/// # Parameters
/// - `p`: A non-null const pointer to an [`ExampleStructure`].
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn param_pointer(p: *const ExampleStructure) {}

/// Accepts a mutable pointer to a structure.
///
/// # Parameters
/// - `p`: A non-null mutable pointer to an [`ExampleStructure`].
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn param_pointer_mut(p: *mut ExampleStructure) {}

/// Accepts an optional immutable reference, emitted as a nullable const pointer in C.
///
/// # Parameters
/// - `p`: An optional reference to an [`ExampleStructure`], or `None` for null.
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn param_option(p: Option<&ExampleStructure>) {}

/// Accepts an optional mutable reference, emitted as a nullable pointer in C.
///
/// # Parameters
/// - `p`: An optional mutable reference to an [`ExampleStructure`], or `None` for null.
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn param_option_mut(p: Option<&mut ExampleStructure>) {}

/// Accepts an optional owned C string, emitted as a nullable `char*` in C.
///
/// # Parameters
/// - `p`: An optional owned C string pointer, or `None` for null.
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn param_string(p: Option<CStringPtr>) {}

/// Accepts a const pointer to a tagged union.
///
/// # Parameters
/// - `p`: A non-null const pointer to an [`ExampleDiscriminatedEnumWithValues`].
///
/// # Returns
/// A 32-bit integer status code. Returns `0` on success.
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn param_tagged_union(p: *const ExampleDiscriminatedEnumWithValues) -> i32 {
    0
}

/// Accepts a bitflags value by copy.
///
/// # Parameters
/// - `f`: A set of [`ExampleFlags`] bitflags.
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn param_flags(f: ExampleFlags) {}

/// Accepts an opaque struct by pointer
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn param_opaque_struct_pointer(f: Option<&OpaqueStruct>) {}
