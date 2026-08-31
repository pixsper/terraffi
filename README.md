# terraffi

[![Build](https://github.com/pixsper/terraffi/actions/workflows/build.yml/badge.svg)](https://github.com/pixsper/terraffi/actions)
[![Cargo](https://img.shields.io/crates/v/terraffi.svg)](https://crates.io/crates/terraffi/)
[![docs.rs](https://img.shields.io/docsrs/terraffi)](https://docs.rs/terraffi/latest/terraffi/)
[![Rust version: 1.93+](https://img.shields.io/badge/rust%20version-1.93+-orange)](https://blog.rust-lang.org/2026/01/22/Rust-1.93.0/)

Collection of libraries to assist with creating Rust libraries which expose a **public-facing** C interface via FFI. The goal is not to simply 
create bindings and types that work across the FFI boundary, but ones which match expected C conventions and produce nicely formatted, human-readable C header files.

## Crates

Terraffi has a split between runtime and generation time crates:

- **terraffi** - A facade crate for all runtime-features including C interop types
  (`CSlice<T>`, `CVec<T>`, `CStringPtr`, `CStringBuffer`) and the annotation macros.
- **terraffi_gen** - Header file generation functionality. Not required at runtime (can be referenced via `[build-dependencies]`).

The remaining crates are implementation detail, re-exported through `terraffi`:
**terraffi_ctypes** (the interop types), **terraffi_macro** (the proc macros), and
**terraffi_helpers** (shared internals). You can depend on `terraffi_ctypes` and
`terraffi_macro` directly if you would rather not pull in the facade - the macros
resolve their paths either way.

### Features

| Feature  | Default | Description |
|----------|---------|-------------|
| `std`    | yes     | Standard library support. Implies `alloc`. |
| `alloc`  | yes     | Owning types (`CVec`, `CStringBuffer`). Requires an allocator. |
| `macros` | yes     | The annotation and derive macros. |
| `serde`  | no      | `Serialize` / `Deserialize` implementations for the interop types. |

The `alloc` feature can be disabled for pure `no_std` targets. Without it the owning
types (`CVec`, `CSlice`, `CStringBuffer`) cannot be constructed or dropped, leaving the
borrowed and pointer types (`CSliceRef`, `CArrayPtr`, `CStringPtr`, `CStringPtrRef` and
friends) for interop where the C side owns the memory.


## Getting Started

### Writing C Compatible Rust Code

Add a dependency to terraffi or run:

```console
cargo add terraffi
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
typedef struct example_struct {
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
typedef enum example_enum {
    EXAMPLE_ENUM_NONE = 0,
    EXAMPLE_ENUM_FOO = 1,
    EXAMPLE_ENUM_BAR = 2
} example_enum_e;
```

#### Enums With Data

A Rust enum whose variants carry data has no direct C equivalent. Terraffi emits it as a
**tagged union**: a struct pairing a *discriminant enum* naming the variant with an
anonymous union holding the payload.

Annotate it with `#[repr(C)]`. If you also assign explicit discriminant values, Rust
requires the tag width alongside it — `#[repr(C, u32)]`:

```rust
#[repr(C, u32)]
pub enum Value {
    None = 0,
    Foo(u32) = 1,
    Bar(f32) = 2
}
```

This will generate:

```c
typedef enum value_kind {
    VALUE_KIND_NONE = 0,
    VALUE_KIND_FOO = 1,
    VALUE_KIND_BAR = 2
} value_kind_e;

typedef struct value {
    value_kind_e kind;
    union {
        uint32_t foo;
        float bar;
    };
} value_t;
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

#### Constants

A `pub const` annotated with `#[terraffi_export]` is emitted as a cast `#define`:

```rust
#[terraffi_export]
pub const EXAMPLE_NO_MAX: u64 = u64::MAX;

#[terraffi_export]
pub const EXAMPLE_NO_MIN: i32 = i32::MIN;
```

```c
#define EXAMPLE_NO_MAX ((uint64_t)UINT64_MAX)
#define EXAMPLE_NO_MIN ((int32_t)INT32_MIN)
```

#### Opaque Types

A type that is *not* `#[repr(C)]` and is only ever used behind a pointer is emitted as an
opaque forward declaration. This is how you hand C a handle to Rust state it must not
inspect:

```rust
pub struct OpaqueStruct;

#[unsafe(no_mangle)]
pub extern "C" fn use_opaque(p: Option<&OpaqueStruct>) {}
```

```c
typedef struct opaque_struct opaque_struct_t;

void use_opaque(const opaque_struct_t* p);
```

Because the definition is not emitted, C can hold and pass the pointer but cannot
dereference it or learn its size.

#### Pointers and References

Rust's pointer and reference forms all map onto C pointers. `Option` collapses to a
nullable pointer rather than adding a discriminant, so an optional reference costs nothing
extra in the layout.

Taking a struct that terraffi names `payload_t` in C:

```rust
#[repr(C)]
pub struct Payload {
    pub id: u32,
}
```

a field `x` of each pointer form is emitted as:

| Rust field | Emitted C |
|------------|-----------|
| `x: *const Payload` | `const payload_t* x;` |
| `x: *mut Payload` | `payload_t* x;` |
| `x: Option<&Payload>`, `x: RefPtr<Payload>` | `const payload_t* x;` |
| `x: Option<&mut Payload>`, `x: MutRefPtr<Payload>` | `payload_t* x;` |
| `x: Option<Box<Payload>>`, `x: BoxPtr<Payload>` | `payload_t* x;` |
| `x: Option<extern "C" fn(u32) -> i32>` | `int32_t (*x)(uint32_t);` |

`RefPtr`, `MutRefPtr` and `BoxPtr` come from terraffi; see
[C Interop Types](#c-interop-types) for the rest of that family and their memory handling.

A type may refer to itself, which C requires be written through the `struct` tag:

```rust
#[repr(C)]
pub struct Node {
    pub value: i32,
    pub next: Option<Box<Node>>,
}
```

```c
typedef struct node {
    int32_t value;
    struct node* next;
} node_t;
```

#### Doc Comments

Rust doc comments are carried across as Doxygen blocks. `# Parameters` and `# Returns`
sections become `@param` and `@return`, and `[`Type`]` intra-doc links become `@ref`:

```rust
/// Accepts a const pointer to a tagged union.
///
/// # Parameters
/// - `p`: A non-null const pointer to a [`Value`].
///
/// # Returns
/// A 32-bit integer status code. Returns `0` on success.
#[unsafe(no_mangle)]
pub extern "C" fn param_tagged_union(p: *const Value) -> i32 { 0 }
```

```c
/** Accepts a const pointer to a tagged union.
 *
 * @param p A non-null const pointer to a @ref Value.
 *
 * @return A 32-bit integer status code. Returns `0` on success.
 */
int32_t param_tagged_union(const value_t* p);
```

#### Export Attributes

By default terraffi exports every public `extern "C"` function, plus every type reachable
from one. Two attributes override that per item:

```rust
/// Forces export even when nothing references this type, and fails the build if the
/// type could never be exported.
#[terraffi_export]
#[repr(C)]
pub struct AlwaysExported { pub value: i32 }

/// Held back even though it is public and C-compatible.
#[terraffi_ignore]
#[repr(C)]
pub struct NeverExported { pub value: i32 }
```

`#[terraffi_export]` is also a compile-time check. It fails to compile if applied to a
struct or enum that is not `#[repr(C)]`, or to a function that is not `pub extern "C"` with
`#[unsafe(no_mangle)]` — so a type drifts out of C compatibility at `cargo build`, not when
someone later reads the header.

The defaults themselves are configurable; see [Export Defaults](#export-defaults).

### Derive Macros

#### `DiscriminantEnum`

A tagged union already generates its discriminant enum in the header. `DiscriminantEnum`
additionally generates it in Rust, so both sides can name the same discriminants:

```rust
#[derive(DiscriminantEnum)]
#[repr(C, u32)]
pub enum Value {
    None = 0,
    Foo(u32) = 1,
    Bar(f32) = 2,
}

// Generates `ValueKind`, plus `Value::kind()` returning it.
assert_eq!(Value::Foo(1).kind(), ValueKind::Foo);
```

The generated name and method are configurable through the `terraffi` helper attribute:

```rust
#[derive(DiscriminantEnum)]
#[terraffi(discriminant_enum_name = "ValueTag", discriminant_method_name = "tag")]
#[terraffi(additional_derives = derive(Serialize, Deserialize))]
#[repr(C, u32)]
pub enum Value {
    None = 0,
    Foo(u32) = 1,
    Bar(f32) = 2,
}
```

#### `CDefault`

C code routinely zero-initialises a struct and expects that to be a valid value. `CDefault`
implements exactly that, so Rust agrees with `memset(&v, 0, sizeof v)`:

```rust
#[derive(CDefault, PartialEq)]
#[repr(C)]
pub struct Config {
    pub retries: u32,
    pub timeout_ms: u32,
}

let c = Config::c_default();     // all-zero
assert!(c.eq_c_default());
```

It only applies to `#[repr(C)]` types, and fails to compile otherwise.

### Generating the C Header

The actual header generation can be done from anywhere, but is most ergonomically integrated as part of a `build.rs` script

Add a build dependency to terraffi_gen or run:

```console
cargo add --build terraffi_gen
```

Then either create a `build.rs` script in your crate directory or add the following to an existing script:

```rust
use std::path::Path;

fn main() {
    println!("cargo::rerun-if-changed=src");

    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let crate_name = Path::new(&crate_dir).file_name().unwrap().to_str().unwrap();
    
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_filename = Path::new(&out_dir).ancestors()
        .nth(3)
        .unwrap()
        .join(format!("{}.h", crate_name.to_ascii_lowercase()));

    let header = terraffi_gen::TerraffiGeneratorBuilder::new()
        .build(crate_dir)
        .generate()
        .unwrap();

    std::fs::write(out_filename, header).unwrap();
}
```

This will automatically generate the header from the source of the crate being built, and save it to a file named `[crate_name].h` in the target directory.
### Configuring Generation

Everything below is optional; anything left unset takes the default shown.
`TerraffiGeneratorBuilder::new()` and `::default()` are equivalent.

#### Naming

Prefixes and suffixes are applied *before* case conversion, so they participate in it:

```rust
TerraffiGeneratorBuilder::new()
    .typename_prefix("mylibrary_")     // both structs and enums
    .struct_prefix("s_")          // or each separately
    .enum_prefix("e_")
    .constant_prefix("MYLIBRARY_")
    .typename_suffix("_t")        // both; defaults are "_t" and "_e"
    .struct_suffix("_t")
    .enum_suffix("_e")
    .constant_suffix("")
    .discriminant_enum_suffix("Kind")
```

Case conventions are set per kind of identifier. Only conventions that produce valid C
identifiers are offered — `Snake`, `UpperSnake`, `Pascal`, `Camel`, `Ada`, `Flat` and
`UpperFlat`:

```rust
use terraffi_gen::Case;

TerraffiGeneratorBuilder::new()
    .typename_case(Case::Snake)         // default
    .parameter_case(Case::Snake)        // default
    .field_case(Case::Snake)            // default
    .enum_member_case(Case::UpperSnake) // default
    .constant_case(Case::UpperSnake)    // default
    .prefix_enum_cases_with_typename(true) // default; C has no enum namespacing
```

A single type can opt out of all of it. The replacement is used verbatim — no prefix,
suffix or case conversion:

```rust
TerraffiGeneratorBuilder::new()
    .rename_type("CStringBuffer", "mylibrary_string_t")
```

#### Export Defaults

```rust
TerraffiGeneratorBuilder::new()
    .export_public_functions()        // default: every pub extern "C" fn
    .export_only_annotated_functions()
    .export_only_annotated_types()    // default: annotated types, plus anything referenced
    .export_public_types()            // every pub C-compatible type, referenced or not
```

#### Header Contents

```rust
TerraffiGeneratorBuilder::new()
    .header_comment("Generated by build.rs — do not edit.")  // block comment at the top
    .header_guard("MY_LIBRARY_H")   // default: derived from the crate directory name
    .add_std_includes(true)         // default: <stdint.h>, <stddef.h>, <stdbool.h>
    .add_include("my_other_header.h")
    .add_macro_definition(CMacro::new(
        "MY_HELPER(x)",
        "Doc comment rendered above the macro.",
        " ((x) * 2)",
    ))
```

`export_macro` prepends a macro to every function declaration and emits a
platform-detection block that defines it, which is what you want for a shared library:

```rust
TerraffiGeneratorBuilder::new().export_macro("DLL_API")
```

```c
#if defined _WIN32 || defined __CYGWIN__
    #define DLL_API __declspec(dllimport)
#elif __GNUC__ >= 4
    #define DLL_API __attribute__ ((visibility ("default")))
#else
    #define DLL_API
#endif
```

#### Dependency Scanning

Types defined in dependency crates are included by default, so a struct you re-export from
another crate of your own still lands in the header. Individual crates can be skipped, and
scanning can be turned off entirely:

```rust
TerraffiGeneratorBuilder::new()
    .exclude_crate("some_dependency")
    .disable_scan_dependencies()
```

Turning it off does not stop exported items *referencing* those types, so pair it with
`add_include` and `assume_declared` for a header that declares them.

### Unsupported Types

A Rust type with no C equivalent has no name terraffi can emit, so generation fails rather
than writing a header that will not compile:

```
terraffi could not resolve 2 types to a C declaration:

  `String` in struct `Config`, field `name`
      `String` is not FFI-safe: use `CStringBuffer` for an owned string, or `CStringPtr` for a borrowed one.
  `Widget` in function `make_widget`, return type
      no C declaration for `Widget` was found. Check the spelling, that the type is
      `#[repr(C)]` or `#[repr(transparent)]`, and that its crate is being scanned.
```

A type that is not `#[repr(C)]` and is only used behind a pointer is emitted as an opaque
forward declaration, so opaque handles are not affected.

If a type is declared in a hand-written header you pull in with `add_include`, terraffi
cannot see it. Name it with `assume_declared`:

```rust
TerraffiGeneratorBuilder::new()
    .add_include("my_other_header.h")
    .assume_declared("ExternallyDeclaredType")
```

To generate anyway and inspect the references yourself, use `allow_unresolved_types`, then
read them back from `TerraffiGenerator::unresolved_types`.

### Errors

`generate` returns `Result<String, TerraffiError>`. `TerraffiError` is a `#[non_exhaustive]`
enum, so a build script can react to a specific failure rather than only printing it:

```rust
match generator.generate() {
    Ok(header) => std::fs::write(out_filename, header).unwrap(),
    Err(TerraffiError::UnresolvedTypes(e)) => {
        for u in &e.unresolved {
            println!("cargo::warning={} in {}", u.type_name, u.location);
        }
        panic!("header would not compile");
    }
    Err(e) => panic!("{e}"),
}
```

It names no third-party type, so a breaking release of `syn` or `cargo_metadata` cannot
change this crate's public API. It is `Send + Sync`, and converts into `Box<dyn Error>` with
`?` for build scripts that return one.

## C Interop Types

**Terraffi does not require these types to be used.** A `#[repr(C)]` struct of primitives, enums and
raw pointers generates a perfectly good header without any of them, and everything in
[Getting Started](#getting-started) works that way.

They exist to bridge two sets of idioms that do not naturally meet. Rust wants ownership,
lifetimes and slices; C wants a pointer, or a pointer and a length. Writing that by hand
means either giving up the Rust side (raw pointers and manual `from_raw_parts` everywhere)
or giving up the C side (exposing Rust's layout and asking C to cope). These types let both
sides keep their idioms: on the Rust side they own, borrow, `Deref` and iterate like the
types you would otherwise reach for, and on the C side terraffi emits the shape a C
programmer expects rather than the Rust wrapper.

The examples below use a struct that terraffi names `payload_t` in C:

```rust
#[repr(C)]
pub struct Payload {
    pub id: u32,
}
```

### How Terraffi Recognises Them

Types are matched on the **final segment of the path as written**, so all of these are
understood:

```rust
use terraffi::CSlice;

#[repr(C)]
pub struct Recognised {
    pub a: CSlice<Payload>,
    pub b: terraffi::CSlice<Payload>,
    pub c: terraffi_ctypes::CSlice<Payload>,
}
```

The terraffi crates are excluded from dependency scanning, so these types never appear in
the header as ordinary scanned struct definitions — they are translated, not emitted.

Matching on the written name has one consequence worth knowing: terraffi does not see
through a type alias. `type Items = CSlice<Payload>;` is treated as a type named `Items`,
not as a slice, and is rejected rather than silently mis-emitted. Write the type directly
in exported items.

### What Terraffi Does With Them

Five behaviours, none of which apply to an ordinary `#[repr(C)]` type:

**1. The wrapper is erased.** `CStringPtr` does not become a one-field struct in C; it
becomes `const char*`. C sees the pointer it would have written itself.

**2. Slices and vectors expand into adjacent members.** A `CSlice<T>` field becomes a
pointer member plus a `_len` member named after the field, and `CVec<T>` adds `_capacity`.
This mirrors the Rust layout exactly, so the two sides agree on the ABI.

**3. `CStringBuffer` is synthesised.** It is the one type that stays a struct, because it
carries its length alongside its pointer and is passed and returned by value. Terraffi
emits its definition into your header even though `terraffi_ctypes` is never scanned.

**4. `Option` costs nothing.** Every pointer-sized type keeps that size inside an
`Option`, so `Option<CStringPtr>` is still one `char*` and `None` is the null pointer.
This is what lets a nullable C string be an `Option` in Rust and a plain `char*` in C.

**5. Slices and vectors are rejected where the length has nowhere to go.** The expansion
in behaviour 2 only works directly in a struct field or a tagged union variant. Every other
position — a function signature, a type alias, behind a pointer, inside an array — would
emit the pointer alone, dropping the length and leaving C passing one word where Rust reads
two. Terraffi fails generation rather than emit that:

```
terraffi found 1 use of a slice or vector type where C cannot carry the length:

  function `sum_items`, parameter `items`
```

Use a pointer with a separate length instead:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn sum_items(items: CArrayPtr<Payload>, len: usize) -> u64 { 0 }
```

```c
uint64_t sum_items(const payload_t* items, size_t len);
```

### Ownership at a Glance

One rule governs all of these, and it is the thing to get right:

> **The owning types free with Rust's allocator.** Their `Drop` reconstructs a specific
> Rust allocation — a `CString`, a `Box<[T]>`, a `Vec<T>` — and releases it. Memory that
> came from `malloc` must never be wrapped in one, and memory handed to C must never be
> released with `free()`.

| Type | Emitted C | Owns its memory |
|------|-----------|-----------------|
| [`CStringPtr`](#cstringptr) | `const char*` | yes |
| [`CStringPtrMut`](#cstringptrmut) | `char*` | yes |
| [`CStringPtrRef<'a>`](#cstringptrrefa) | `const char*` | no |
| [`CStringPtrMutRef<'a>`](#cstringptrmutrefa) | `char*` | no |
| [`CStringBuffer`](#cstringbuffer) | `c_string_buffer_t` | yes |
| [`CStringBufferRef<'a>`](#cstringbufferrefa) | `c_string_buffer_t` | no |
| [`CSlice<T>`](#cslicet) | `T*` + `_len` | yes |
| [`CSliceRef<'a, T>`](#cslicerefa-t) | `const T*` + `_len` | no |
| [`CSliceMutRef<'a, T>`](#cslicemutrefa-t) | `T*` + `_len` | no |
| [`CVec<T>`](#cvect) | `T*` + `_len` + `_capacity` | yes |
| [`CVecRef<'a, T>`](#cvecrefa-t) | `const T*` + `_len` + `_capacity` | no |
| [`CVecMutRef<'a, T>`](#cvecmutrefa-t) | `T*` + `_len` + `_capacity` | no |
| [`CArrayPtr<T>`](#carrayptrt) | `const T*` | no |
| [`CArrayPtrMut<T>`](#carrayptrmutt) | `T*` | no |
| [`CArrayPtrRef<'a, T>`](#carrayptrrefa-t) | `const T*` | no |
| [`CArrayPtrMutRef<'a, T>`](#carrayptrmutrefa-t) | `T*` | no |
| [`CHandle<T>`](#chandlet) | `T**` | see below |
| [`BoxPtr<T>`](#boxptrt) | `T*` | yes |
| [`RefPtr<'a, T>`](#refptra-t) | `const T*` | no |
| [`MutRefPtr<'a, T>`](#mutrefptra-t) | `T*` | no |

### Strings

#### `CStringPtr`

Emitted as `const char*`. An owned, null-terminated UTF-8 string with no interior nulls.

Use it when Rust allocates a string and C only reads it. `Option<CStringPtr>` is the same
single pointer, with `None` as null, so use that when the string may be absent.

Owns a `CString`. `Drop` frees it with Rust's allocator, so a pointer that came from C must
never be wrapped in this type.

#### `CStringPtrMut`

Emitted as `char*`. As `CStringPtr`, but C may modify the bytes in place.

Length is fixed: C can change the contents, not grow the string. Owns a `CString`; `Drop`
frees it.

#### `CStringPtrRef<'a>`

Emitted as `const char*`. A borrowed, null-terminated string.

Use it when **C owns** the string and Rust reads it — a string from `malloc`, a string
literal, or a buffer C keeps alive for the duration of the call. The lifetime ties it to
whatever it borrows from.

Never frees anything.

#### `CStringPtrMutRef<'a>`

Emitted as `char*`. A borrowed string Rust may modify in place.

Use it for a C-owned buffer Rust writes into. Never frees anything.

#### `CStringBuffer`

Emitted as a struct:

```c
typedef struct c_string_buffer {
    /** Pointer to a null-terminated UTF-8 string, or NULL if absent. */
    const char* ptr;
    /** Length of the string in bytes, including the null terminator. */
    size_t len;
} c_string_buffer_t;
```

The string type to reach for when the value may be absent, may contain interior nulls, or
when C should not have to call `strlen`. A null `ptr` means `None`. It is also the only
string type that survives a by-value function signature, because it is a real struct in C.

Owns a `Box<[u8]>` of exactly `len` bytes. `Drop` frees it, and `leak`/`free` hand that
responsibility across the boundary — see [Handing Memory to C](#handing-memory-to-c).

#### `CStringBufferRef<'a>`

Emitted as `c_string_buffer_t` — the same struct as `CStringBuffer`, because the two share
a layout. The borrowed/owned distinction is a Rust-side concern, exactly as it is for
`CStringPtr` and `CStringPtrRef`.

Use it when **C owns** the buffer and Rust reads it. Unlike `CStringBuffer`, the pointer and
length are non-null and non-zero, so an absent value is `Option<CStringBufferRef>` — still
the same two words, thanks to the niche.

Never frees anything.

### Slices and Vectors

Remember behaviour 5: these are only usable directly in a struct field or a tagged union
variant.

`CSlice<T>` and `CVec<T>` deliberately mirror the interfaces of `Box<[T]>` and `Vec<T>`, so
code either side of the boundary reads the same way. You build them from the Rust types or
by collecting, and inspect them with the method names you already know:

```rust
// Construction, as for Vec.
let mut buffer: CVec<i32> = CVec::with_capacity(16);
buffer.reserve(8);

// From the Rust equivalents, or straight from an iterator.
let from_vec: CVec<i32> = vec![1, 2, 3].into();
let collected: CSlice<i32> = (0..3).collect();

// Inspection, as for a slice.
let total: i32 = from_vec.iter().sum();
let first = collected.as_slice().first();
```

Both carry `len`, `is_empty`, `iter`, `iter_mut`, `as_slice` and `as_mut_slice`, both
implement `From<Vec<T>>`, `From<Box<[T]>>` and `FromIterator<T>`, and both derive `Default`,
`Clone`, `PartialEq`, `Eq`, `Debug` and `Hash` (plus `Serialize` and `Deserialize` under the
`serde` feature).

`CVec<T>` goes further and is close to a drop-in replacement for `Vec<T>`. It implements
`Deref<Target = [T]>` and `DerefMut`, so the whole slice API and indexing come with it, and
it implements `IntoIterator` in all three forms:

```rust
let mut buffer: CVec<i32> = vec![3, 1, 2].into();

// Element-wise mutation.
buffer.push(4);
let last = buffer.pop();
buffer.insert(0, 0);
let first = buffer.remove(0);

// Indexing and the slice API, through Deref.
buffer.sort();
let smallest = buffer.first();
let tail = &buffer[1..];

// Iteration, by reference, by mutable reference, or by value.
for x in &buffer {}
for x in &mut buffer { *x += 1; }
let total: i32 = buffer.into_iter().sum();
```

Alongside those it has `capacity`, `reserve`, `reserve_exact`, `shrink_to_fit`, `clear`,
`truncate`, `swap_remove` and `Extend<T>`. Every mutating method keeps the pointer, length
and capacity consistent, including across a reallocation that moves the buffer, so the C
side always sees a valid triple.

`CSlice<T>` deliberately stops short of that. It models a fixed-length buffer, so it has no
element-wise mutation and no `Deref` — reach for `as_slice()` or `as_mut_slice()` and use
the slice API there. Convert to `CVec<T>` if you need to grow it.

Growing a `CVec<T>` behaves like growing a C++ `std::vector`: if the buffer has to move,
raw pointers into the old one are invalidated. Every mutating method writes the new pointer,
length and capacity back into the struct's C-visible members, so C code that reads them each
time is always correct — the only thing to avoid is caching the bare pointer across a call
that might grow the buffer.

#### `CSlice<T>`

Emitted as a pointer plus a `_len` member.

The default choice for an array Rust owns whose length is fixed once built.

Owns a `Box<[T]>`. `Drop` frees it.

#### `CSliceRef<'a, T>`

Emitted as a `const` pointer plus a `_len` member.

Use it when **C owns** the array and Rust reads it. Never frees anything.

#### `CSliceMutRef<'a, T>`

Emitted as a pointer plus a `_len` member.

Use it when C owns the array and Rust writes into it. The length cannot change. Never frees
anything.

#### `CVec<T>`

Emitted as a pointer plus `_len` and `_capacity` members.

Use it only when the capacity genuinely matters — when Rust may still grow the array while
C holds it. It costs C a third member, so prefer `CSlice<T>` otherwise.

Owns a `Vec<T>`. `Drop` frees the full capacity, not just the initialised length.

#### `CVecRef<'a, T>`

Emitted as a `const` pointer plus `_len` and `_capacity` members.

Reading a `CVec` that C owns. Never frees anything.

#### `CVecMutRef<'a, T>`

Emitted as a pointer plus `_len` and `_capacity` members.

Writing into a `CVec` that C owns. Never frees anything.

### Array Pointers

These carry no length, which is exactly why they work in a function signature where the
length travels as a separate parameter.

#### `CArrayPtr<T>`

Emitted as `const T*`. A non-null read-only array pointer.

Use it when the length is already known to both sides, or is passed alongside. Wrap in
`Option` where the pointer may be null; that is still one pointer.

Never owns, never frees.

#### `CArrayPtrMut<T>`

Emitted as `T*`. As `CArrayPtr<T>`, writable. Never owns, never frees.

#### `CArrayPtrRef<'a, T>`

Emitted as `const T*`. A read-only array pointer with a lifetime tying it to what it
borrows from — the same C shape as `CArrayPtr<T>`, with the borrow checked on the Rust
side. Never owns, never frees.

#### `CArrayPtrMutRef<'a, T>`

Emitted as `T*`. The writable, lifetime-bound form. Never owns, never frees.

### Single Pointers and Handles

#### `CHandle<T>`

Emitted as `T**`. An opaque handle C holds across calls: the outer pointer says whether the
handle is valid, the inner one whether a value is currently held.

Use it to give C a token for Rust state it must not inspect. C declares a `payload_t*`
variable and passes its address; Rust fills it with `alloc` and empties it with `take`.

**This is the one type that does not free itself.** It deliberately implements no `Drop`,
because C may still hold a copy of the pointer. Letting one fall out of scope in Rust leaks
both the handle slot and any value it holds. Release it explicitly, normally from a `*_free`
function that calls `CHandle::take` and discards the result.

#### `BoxPtr<T>`

Emitted as `T*`. An `Option<Box<T>>` — a nullable owning pointer to one heap value.

Use it to move a single value across the boundary. Owns the `Box`; `Drop` frees it.

#### `RefPtr<'a, T>`

Emitted as `const T*`. An `Option<&'a T>` — a nullable read-only pointer to one value,
with `None` as null.

Never owns, never frees.

#### `MutRefPtr<'a, T>`

Emitted as `T*`. An `Option<&'a mut T>`. The writable form. Never owns, never frees.

### Handing Memory to C

When ownership really does cross the boundary, `Drop` must not run on the Rust side. Leak
on the way out, and provide a function C calls to give it back:

```rust
/// Creates an owned string. The caller must release it with `string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn string_new() -> ManuallyDrop<CStringBuffer> {
    unsafe { CStringBuffer::new("hello").leak() }
}

/// Frees a string produced by `string_new`.
#[unsafe(no_mangle)]
pub extern "C" fn string_free(str: Option<&mut CStringBuffer>) {
    if let Some(str) = str {
        unsafe { str.free() };
    }
}
```

```c
c_string_buffer_t string_new(void);
void string_free(c_string_buffer_t* str);
```

`ManuallyDrop` is transparent to the generator, so the C signature is unchanged by it.
`CSlice` and `CVec` have the same shape through `into_raw_parts` and `from_raw_parts`.

The rule this enforces is the one at the top of this section: C must call `string_free`,
never `free()`. The buffer is a Rust `Box<[u8]>`, and releasing it with the C allocator is
undefined behaviour.

## Minimum Supported Rust Version

Rust 1.93. Raising it is a breaking change and will come with a minor version bump while
terraffi is pre-1.0.

## License

Licensed under the [MIT License](LICENSE).
