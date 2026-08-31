use bitflags::bitflags;
use serde::Deserialize;
use serde::Serialize;
use std::ffi::CString;
use std::mem::ManuallyDrop;
use terraffi_ctypes::{BoxPtr, CSlice, CStringBuffer, CStringPtr, MutRefPtr, RefPtr};
use terraffi_macro::{DiscriminantEnum, terraffi, terraffi_export};
use terraffi_testdeplib::ExampleDependencyStruct;

/// An opaque struct
pub struct OpaqueStruct;

/// An opaque struct which should not appear in the header as it is unreferenced
pub struct UnreferencedOpaqueStruct;

/// Uses the borrowed string buffer, which shares `CStringBuffer`'s layout and so
/// shares its C struct.
#[terraffi_export]
#[repr(C)]
pub struct ExampleBorrowedBuffer<'a> {
    /// A borrowed null-terminated buffer.
    pub borrowed: terraffi_ctypes::CStringBufferRef<'a>,
    /// The same, optional.
    pub maybe: Option<terraffi_ctypes::CStringBufferRef<'a>>,
}

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
    /// A box ptr member
    pub box_ptr_member: BoxPtr<i32>,
    /// A option box member
    pub option_box_member: Option<Box<i32>>,
}

#[terraffi_export]
#[repr(C)]
pub struct ExampleRefStructure<'a> {
    pub option_ref: Option<&'a ExampleStructure>,
    pub ref_ptr: RefPtr<'a, ExampleStructure>,
    pub mut_option_ref: Option<&'a mut ExampleStructure>,
    pub mut_ref_ptr: MutRefPtr<'a, ExampleStructure>,
}

/// A self-referential structure demonstrating a node that contains a pointer
/// to another instance of its own type, as in a singly-linked list.
#[terraffi_export]
#[repr(C)]
pub struct ExampleSelfReferentialStructure {
    /// The value held by this node.
    pub value: i32,
    /// A pointer to the next node, or null if this is the last node.
    pub next: Option<Box<ExampleSelfReferentialStructure>>,
}

/// One half of a mutually referential pair. Neither type's typedef can be complete
/// before the other is named, so the first one emitted refers to its peer through
/// the struct tag.
#[terraffi_export]
#[repr(C)]
pub struct ExampleMutualA {
    /// A value held by this node.
    pub value: i32,
    /// A pointer to the peer, or null.
    pub peer: Option<Box<ExampleMutualB>>,
}

/// The other half of the mutually referential pair. By the time this is emitted
/// its peer is declared, so it uses the plain typedef name.
#[terraffi_export]
#[repr(C)]
pub struct ExampleMutualB {
    /// A value held by this node.
    pub value: i32,
    /// A pointer to the peer, or null.
    pub peer: Option<Box<ExampleMutualA>>,
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

/// A struct only used inside a data-carrying enum variant, not referenced by any function.
#[repr(C)]
pub struct UnionOnlyStruct {
    /// An x coordinate.
    pub x: f32,
    /// A y coordinate.
    pub y: f32,
}

/// An enum with data-carrying variants, emitted as a tagged union in C.
/// The `DiscriminantEnum` derive generates a companion `ExampleDataEnumKind`
/// enum containing only the discriminant tags.
#[derive(DiscriminantEnum)]
#[terraffi(additional_derives = derive(Serialize, Deserialize))]
#[repr(C)]
pub enum ExampleDataEnum {
    /// Empty variant with no associated data.
    None,
    /// Variant carrying a single unsigned 32-bit integer.
    Foo(u32),
    /// Variant carrying an enum value.
    Bar(ExampleEnum),
    /// Variant carrying a full structure.
    Baz(ExampleStructure),
    /// Variant carrying a struct only used in this enum.
    Qux(UnionOnlyStruct),
}

/// An enum with data-carrying variants, explicit discriminant values, and a fixed
/// `u32` tag width. Demonstrates that variant values are preserved in the generated
/// C discriminant enum.
#[repr(C, u32)]
#[terraffi(discriminant_enum_name = "ExampleDataEnumKind")]
pub enum ExampleDataEnumWithValues {
    /// Empty variant with value 0.
    None = 0,
    /// Variant carrying a `u32`, with value 10.
    Foo(u32) = 10,
    /// Variant carrying an enum, with an auto-incremented value.
    Bar(ExampleEnum),
    /// Variant carrying a structure, with value 2544.
    Baz(ExampleStructure) = 2544,
}

/// Sentinel meaning "no maximum" — non-ASCII text like § must survive
/// generation intact, and the value must emit as a C `<stdint.h>` macro.
#[terraffi_export]
pub const EXAMPLE_NO_MAX: u64 = u64::MAX;

/// Sentinel meaning "no minimum".
#[terraffi_export]
pub const EXAMPLE_NO_MIN: i32 = i32::MIN;

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
/// - `p`: A non-null const pointer to an [`ExampleDataEnumWithValues`].
///
/// # Returns
/// A 32-bit integer status code. Returns `0` on success.
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn param_tagged_union(p: *const ExampleDataEnumWithValues) -> i32 {
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

/// Creates an owned string
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn string_new() -> ManuallyDrop<CStringBuffer> {
    unsafe { CStringBuffer::new("foo").leak() }
}

/// Frees an owned string
#[terraffi_export]
#[unsafe(no_mangle)]
pub extern "C" fn string_free(str: Option<&mut CStringBuffer>) {
    if let Some(str) = str {
        unsafe { str.free() };
    }
}
