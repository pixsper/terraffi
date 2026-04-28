#ifndef TERRAFFI_TESTLIB_H
#define TERRAFFI_TESTLIB_H

#include <stdint.h>
#include <stdbool.h>

/** A simple struct defined in a dependency crate, used to verify that types from local dependency crates are included
 *   in the generated header.
 */
typedef struct example_dependency_struct_t {
    int32_t foo;
} example_dependency_struct_t;

/** An opaque struct */
typedef struct opaque_struct_t opaque_struct_t;

typedef uint8_t example_slice_type_t[16];

/** A simple C-compatible enum with unit variants and no explicit discriminant values. */
typedef enum example_enum_e {
    /** Represents the absence of a value. */
    EXAMPLE_ENUM_NONE,
    /** First variant. */
    EXAMPLE_ENUM_FOO,
    /** Second variant. */
    EXAMPLE_ENUM_BAR
} example_enum_e;

/** A structure demonstrating various field types supported by terraffi, including primitives, enums, nullable strings,
 *   slices, and types from dependency crates.
 */
typedef struct example_structure_t {
    /** A single-precision floating point value. */
    float float_member;
    /** A 32-bit signed integer value. */
    int32_t int_member;
    /** An enum member demonstrating enum field support. */
    example_enum_e enum_member;
    /** An optional owned C string pointer, nullable in the generated header. */
    const char* string_pointer_member;
    /** A slice of integers, expanded to a length + pointer pair in C. */
    int32_t* array_member;
    /** Number of elements in array_member. */
    size_t array_member_len;
    /** A struct from a dependency crate. */
    example_dependency_struct_t struct_member;
    /** A slice member */
    uint8_t slice_member[16];
    /** A box ptr member */
    int32_t* box_ptr_member;
    /** A option box member */
    int32_t* option_box_member;
} example_structure_t;

typedef struct example_ref_structure_t {
    const example_structure_t* option_ref;
    const example_structure_t* ref_ptr;
    example_structure_t* mut_option_ref;
    example_structure_t* mut_ref_ptr;
} example_ref_structure_t;

/** A C-compatible enum with explicitly assigned discriminant values, including gaps in the numbering. */
typedef enum example_enum_with_values_e {
    /** Default variant with value 0. */
    EXAMPLE_ENUM_WITH_VALUES_NONE = 0,
    /** Variant with an explicit value of 10. */
    EXAMPLE_ENUM_WITH_VALUES_FOO = 10,
    /** Variant with an auto-incremented value. */
    EXAMPLE_ENUM_WITH_VALUES_BAR,
    /** Variant with a large explicit value. */
    EXAMPLE_ENUM_WITH_VALUES_BAZ = 2544
} example_enum_with_values_e;

/** A struct only used inside a discriminated union variant, not referenced by any function. */
typedef struct union_only_struct_t {
    /** An x coordinate. */
    float x;
    /** A y coordinate. */
    float y;
} union_only_struct_t;

/** A discriminated (tagged) union demonstrating variants that carry associated data.
 *
 * The `DiscriminantEnum` derive generates a companion `ExampleDiscriminatedEnumKind` enum containing only the
 *   discriminant tags.
 */
typedef enum example_discriminated_enum_kind_e {
    /** Empty variant with no associated data. */
    EXAMPLE_DISCRIMINATED_ENUM_KIND_NONE,
    /** Variant carrying a single unsigned 32-bit integer. */
    EXAMPLE_DISCRIMINATED_ENUM_KIND_FOO,
    /** Variant carrying an enum value. */
    EXAMPLE_DISCRIMINATED_ENUM_KIND_BAR,
    /** Variant carrying a full structure. */
    EXAMPLE_DISCRIMINATED_ENUM_KIND_BAZ,
    /** Variant carrying a struct only used in this union. */
    EXAMPLE_DISCRIMINATED_ENUM_KIND_QUX
} example_discriminated_enum_kind_e;

/** A discriminated (tagged) union demonstrating variants that carry associated data.
 *
 * The `DiscriminantEnum` derive generates a companion `ExampleDiscriminatedEnumKind` enum containing only the
 *   discriminant tags.
 */
typedef struct example_discriminated_enum_t {
    example_discriminated_enum_kind_e kind;
    union {
        uint32_t foo;
        example_enum_e bar;
        example_structure_t baz;
        union_only_struct_t qux;
    }
} example_discriminated_enum_t;

/** A discriminated union with explicit discriminant values and a fixed underlying representation of `u32`. Demonstrates
 *   that variant values are preserved in the generated C kind enum.
 */
typedef struct example_discriminated_enum_with_values_t {
    example_discriminated_enum_kind_e kind;
    union {
        uint32_t foo;
        example_enum_e bar;
        example_structure_t baz;
    }
} example_discriminated_enum_with_values_t;

/** A set of bitflags demonstrating `bitflags!` macro support in terraffi.
 *
 * Emitted as a typedef with `#define` constants in the generated C header.
 */
typedef uint32_t example_flags_t;
/** First flag (bit 0). */
#define EXAMPLE_FLAGS_FOO ((example_flags_t)0x1)
/** Second flag (bit 1). */
#define EXAMPLE_FLAGS_BAR ((example_flags_t)0x2)
/** Third flag (bit 2). */
#define EXAMPLE_FLAGS_BAZ ((example_flags_t)0x4)

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/** Accepts an enum by value.
 *
 * @param v The enum value to process.
 */
void param_enum(example_enum_with_values_e v);

/** Accepts a const pointer to a structure.
 *
 * @param p A non-null const pointer to an @ref ExampleStructure.
 */
void param_pointer(const example_structure_t* p);

/** Accepts a mutable pointer to a structure.
 *
 * @param p A non-null mutable pointer to an @ref ExampleStructure.
 */
void param_pointer_mut(example_structure_t* p);

/** Accepts an optional immutable reference, emitted as a nullable const pointer in C.
 *
 * @param p An optional reference to an @ref ExampleStructure, or `None` for null.
 */
void param_option(const example_structure_t* p);

/** Accepts an optional mutable reference, emitted as a nullable pointer in C.
 *
 * @param p An optional mutable reference to an @ref ExampleStructure, or `None` for null.
 */
void param_option_mut(example_structure_t* p);

/** Accepts an optional owned C string, emitted as a nullable `char*` in C.
 *
 * @param p An optional owned C string pointer, or `None` for null.
 */
void param_string(const char* p);

/** Accepts a const pointer to a tagged union.
 *
 * @param p A non-null const pointer to an @ref ExampleDiscriminatedEnumWithValues.
 *
 * @return A 32-bit integer status code. Returns `0` on success.
 */
int32_t param_tagged_union(const example_discriminated_enum_with_values_t* p);

/** Accepts a bitflags value by copy.
 *
 * @param f A set of @ref ExampleFlags bitflags.
 */
void param_flags(example_flags_t f);

/** Accepts an opaque struct by pointer
 */
void param_opaque_struct_pointer(const opaque_struct_t* f);


#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif
