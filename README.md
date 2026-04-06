# terraffi

[![Build](https://github.com/pixsper/terraffi/actions/workflows/build.yml/badge.svg)](https://github.com/pixsper/terraffi/actions)
[![Cargo](https://img.shields.io/crates/v/terraffi.svg)](https://crates.io/crates/terraffi/)
[![docs.rs](https://img.shields.io/docsrs/terraffi)](https://docs.rs/terraffi/latest/terraffi/)
[![Rust version: 1.94+](https://img.shields.io/badge/rust%20version-1.94+-orange)](https://blog.rust-lang.org/2026/03/05/Rust-1.94.0/)

Collection of libraries to assist with creating Rust libraries which expose a **public-facing** C interface via FFI. The goal is not to simply 
create bindings and types that work across the FFI boundary, but ones which match expected C conventions and produce nicely formatted, human-readable C header files.

## Crates
- **terraffi_ctypes** - Provides a number of types specialized for C interop including `CSlice<T>`, `CVec<T>`,  `CStringPtr` and `CStringBuffer`.
- **terraffi_macro** - Proc macro crate for annotating libraries which produce a C interface.
- **terraffi_gen** - Header file generation.

## Getting Started

### Writing C Compatible Rust Code

Add a dependency to terraffi_ctypes and terraffi_macro or run:

```console
cargo add terraffi_ctypes terraffi_macro
```

By default, Terrafi will export all C compatible public functions, and any types referenced by their parameters. To force the export of a type, 
use the `#[terraffi_export]` macro.

#### Functions

A C compatible function must be annotated with `#[unsafe(no_mangle)]` and declared as `pub extern "C"`. For example:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn example_function(foo: i32) -> i32 {
    foo
}
```

This will generate:

```c
int32_t example_function(int32_t foo);
```

#### Structs

A C compatible struct type must be annotated with `#[repr(C)]` or `#[repr(transparent)]`:

```rust
#[repr(C)]
pub struct ExampleStruct {
    pub foo: i32,
    pub bar: f32,
}

#[repr(transparent)]
pub struct ExampleTransparentStruct([u8; 16]);
```

This will generate:

```c
typedef struct example_struct_t {
    int32_t foo;
    float bar;
} example_struct;

typedef uint8_t example_transparent_struct_t[16];
```

#### Enums

A C compatible enum  must be annotated with `#[repr(C)]`:

```rust
#[repr(C)]
pub enum ExampleEnum {
    None = 0,
    Foo = 1,
    Bar = 2,
}
```

This will generate:

```c
typedef enum example_enum_e {
    EXAMPLE_ENUM_NONE = 0,
    EXAMPLE_ENUM_FOO = 1,
    EXAMPLE_ENUM_BAR = 2
} example_enum_e;
```

#### Discriminated Enums

A C compatible discriminated enum must be annotated with `#[repr(C)]` (or optionally `#[repr(C, u32)]`):

```rust
#[repr(C, u32)]
pub enum ExampleDiscriminatedEnum {
    None = 0,
    Foo(u32) = 1,
    Bar(f32) = 2
}
```

This will generate:

```c
typedef enum example_discriminated_enum_kind_e {
    EXAMPLE_DISCRIMINATED_ENUM_KIND_NONE = 0,
    EXAMPLE_DISCRIMINATED_ENUM_KIND_FOO = 1,
    EXAMPLE_DISCRIMINATED_ENUM_KIND_BAR = 2
} example_discriminated_enum_kind_e;

typedef struct example_discriminated_enum_t {
    example_discriminated_enum_kind_e kind;
    union {
        uint32_t foo;
        float bar;
    };
} example_discriminated_enum_t;
```

#### Bitflags

Terraffi supports parsing the `bitflags!` macro from the [bitflags](https://github.com/bitflags/bitflags) crate:

```rust
bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct ExampleFlags: u32 {
        const Foo = 0b00000001;
        const Bar = 0b00000010;
        const Baz = 0b00000100;
    }
}
```

This will generate:

```c
typedef uint32_t example_flags_t;
#define EXAMPLE_FLAGS_FOO ((example_flags_t)0x1)
#define EXAMPLE_FLAGS_BAR ((example_flags_t)0x2)
#define EXAMPLE_FLAGS_BAZ ((example_flags_t)0x4)
```

### Generating the C Header

The actual header generation can be done from anywhere, but is most ergonomically integrated as part of a `build.rs` script

Add a build dependency to terraffi_gen or run:

```console
cargo add --build terraffi_gen
```

Then either create a `build.rs` script in your crate directory or add the following to an existing script:

```rust
fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let crate_name = Path::new(&crate_dir).file_name().unwrap().to_str().unwrap();
    
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_filename = Path::new(&out_dir).ancestors()
        .nth(3)
        .unwrap().
        join(format!("{}.h", crate_name.to_ascii_lowercase()));

    let header = terraffi_gen::TerraffiGeneratorBuilder::new()
        .build(crate_dir)
        .generate()
        .unwrap();

    std::fs::write(out_filename, header).unwrap();
}
```

This will automatically generate the header from the source of the crate being built, and save it to a file named `[crate_name].h` in the target directory.