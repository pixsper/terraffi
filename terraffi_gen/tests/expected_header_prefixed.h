#ifndef TERRAFFI_TESTLIB_H
#define TERRAFFI_TESTLIB_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/** A simple struct defined in a dependency crate, used to verify that types from local dependency crates are included
 *   in the generated header.
 */
typedef struct test_example_dependency_struct_t {
    int32_t foo;
} test_example_dependency_struct_t;

/** An opaque struct */
typedef struct test_opaque_struct_t test_opaque_struct_t;

typedef uint8_t test_example_slice_type_t[16];

/** A simple C-compatible enum with unit variants and no explicit discriminant values. */
typedef enum test_example_enum_e {
    /** Represents the absence of a value. */
    TEST_EXAMPLE_ENUM_NONE,
    /** First variant. */
    TEST_EXAMPLE_ENUM_FOO,
    /** Second variant. */
    TEST_EXAMPLE_ENUM_BAR
} test_example_enum_e;

/** A structure demonstrating various field types supported by terraffi, including primitives, enums, nullable strings,
 *   slices, and types from dependency crates.
 */
typedef struct test_example_structure_t {
    /** A single-precision floating point value. */
    float float_member;
    /** A 32-bit signed integer value. */
    int32_t int_member;
    /** An enum member demonstrating enum field support. */
    test_example_enum_e enum_member;
    /** An optional owned C string pointer, nullable in the generated header. */
    const char* string_pointer_member;
    /** A slice of integers, expanded to a length + pointer pair in C. */
    int32_t* array_member;
    /** Number of elements in array_member. */
    size_t array_member_len;
    /** A struct from a dependency crate. */
    test_example_dependency_struct_t struct_member;
    /** A slice member */
    uint8_t slice_member[16];
    /** A box ptr member */
    int32_t* box_ptr_member;
    /** A option box member */
    int32_t* option_box_member;
} test_example_structure_t;

typedef struct test_example_ref_structure_t {
    const test_example_structure_t* option_ref;
    const test_example_structure_t* ref_ptr;
    test_example_structure_t* mut_option_ref;
    test_example_structure_t* mut_ref_ptr;
} test_example_ref_structure_t;

/** A C-compatible enum with explicitly assigned discriminant values, including gaps in the numbering. */
typedef enum test_example_enum_with_values_e {
    /** Default variant with value 0. */
    TEST_EXAMPLE_ENUM_WITH_VALUES_NONE = 0,
    /** Variant with an explicit value of 10. */
    TEST_EXAMPLE_ENUM_WITH_VALUES_FOO = 10,
    /** Variant with an auto-incremented value. */
    TEST_EXAMPLE_ENUM_WITH_VALUES_BAR,
    /** Variant with a large explicit value. */
    TEST_EXAMPLE_ENUM_WITH_VALUES_BAZ = 2544
} test_example_enum_with_values_e;

/** A struct only used inside a discriminated union variant, not referenced by any function. */
typedef struct test_union_only_struct_t {
    /** An x coordinate. */
    float x;
    /** A y coordinate. */
    float y;
} test_union_only_struct_t;

/** A discriminated (tagged) union demonstrating variants that carry associated data.
 *
 * The `DiscriminantEnum` derive generates a companion `ExampleDiscriminatedEnumKind` enum containing only the
 *   discriminant tags.
 */
typedef enum test_example_discriminated_enum_kind_e {
    /** Empty variant with no associated data. */
    TEST_EXAMPLE_DISCRIMINATED_ENUM_KIND_NONE,
    /** Variant carrying a single unsigned 32-bit integer. */
    TEST_EXAMPLE_DISCRIMINATED_ENUM_KIND_FOO,
    /** Variant carrying an enum value. */
    TEST_EXAMPLE_DISCRIMINATED_ENUM_KIND_BAR,
    /** Variant carrying a full structure. */
    TEST_EXAMPLE_DISCRIMINATED_ENUM_KIND_BAZ,
    /** Variant carrying a struct only used in this union. */
    TEST_EXAMPLE_DISCRIMINATED_ENUM_KIND_QUX
} test_example_discriminated_enum_kind_e;

/** A discriminated (tagged) union demonstrating variants that carry associated data.
 *
 * The `DiscriminantEnum` derive generates a companion `ExampleDiscriminatedEnumKind` enum containing only the
 *   discriminant tags.
 */
typedef struct test_example_discriminated_enum_t {
    test_example_discriminated_enum_kind_e kind;
    union {
        uint32_t foo;
        test_example_enum_e bar;
        test_example_structure_t baz;
        test_union_only_struct_t qux;
    };
} test_example_discriminated_enum_t;

/** A discriminated union with explicit discriminant values and a fixed underlying representation of `u32`. Demonstrates
 *   that variant values are preserved in the generated C kind enum.
 */
typedef struct test_example_discriminated_enum_with_values_t {
    test_example_discriminated_enum_kind_e kind;
    union {
        uint32_t foo;
        test_example_enum_e bar;
        test_example_structure_t baz;
    };
} test_example_discriminated_enum_with_values_t;

/** A set of bitflags demonstrating `bitflags!` macro support in terraffi.
 *
 * Emitted as a typedef with `#define` constants in the generated C header.
 */
typedef uint32_t test_example_flags_t;
/** First flag (bit 0). */
#define TEST_EXAMPLE_FLAGS_FOO ((test_example_flags_t)0x1)
/** Second flag (bit 1). */
#define TEST_EXAMPLE_FLAGS_BAR ((test_example_flags_t)0x2)
/** Third flag (bit 2). */
#define TEST_EXAMPLE_FLAGS_BAZ ((test_example_flags_t)0x4)

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/** Accepts an enum by value.
 *
 * @param v The enum value to process.
 */
void param_enum(test_example_enum_with_values_e v);

/** Accepts a const pointer to a structure.
 *
 * @param p A non-null const pointer to an @ref ExampleStructure.
 */
void param_pointer(const test_example_structure_t* p);

/** Accepts a mutable pointer to a structure.
 *
 * @param p A non-null mutable pointer to an @ref ExampleStructure.
 */
void param_pointer_mut(test_example_structure_t* p);

/** Accepts an optional immutable reference, emitted as a nullable const pointer in C.
 *
 * @param p An optional reference to an @ref ExampleStructure, or `None` for null.
 */
void param_option(const test_example_structure_t* p);

/** Accepts an optional mutable reference, emitted as a nullable pointer in C.
 *
 * @param p An optional mutable reference to an @ref ExampleStructure, or `None` for null.
 */
void param_option_mut(test_example_structure_t* p);

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
int32_t param_tagged_union(const test_example_discriminated_enum_with_values_t* p);

/** Accepts a bitflags value by copy.
 *
 * @param f A set of @ref ExampleFlags bitflags.
 */
void param_flags(test_example_flags_t f);

/** Accepts an opaque struct by pointer
 */
void param_opaque_struct_pointer(const test_opaque_struct_t* f);


#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif
