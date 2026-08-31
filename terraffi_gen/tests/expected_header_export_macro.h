#ifndef TERRAFFI_TESTLIB_H
#define TERRAFFI_TESTLIB_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#if defined _WIN32 || defined __CYGWIN__
    #define DLL_API __declspec(dllimport)
#elif __GNUC__ >= 4
    #define DLL_API __attribute__ ((visibility ("default")))
#else
    #define DLL_API
#endif

/** A simple struct defined in a dependency crate, used to verify that types from local dependency crates are included
 *   in the generated header.
 */
typedef struct example_dependency_struct {
    int32_t foo;
} example_dependency_struct_t;

/** An owned null-terminated UTF-8 string buffer.
 *
 * A null pointer represents an absent value (equivalent to None). The buffer must end with a null byte for C
 *   compatibility.
 */
typedef struct c_string_buffer {
    /** Pointer to a null-terminated UTF-8 string, or NULL if absent. */
    const char* ptr;
    /** Length of the string in bytes, including the null terminator. */
    size_t len;
} c_string_buffer_t;

/** An opaque struct */
typedef struct opaque_struct opaque_struct_t;

/** Uses the borrowed string buffer, which shares `CStringBuffer`'s layout and so shares its C struct. */
typedef struct example_borrowed_buffer {
    /** A borrowed null-terminated buffer. */
    c_string_buffer_t borrowed;
    /** The same, optional. */
    c_string_buffer_t maybe;
} example_borrowed_buffer_t;

/** A self-referential structure demonstrating a node that contains a pointer to another instance of its own type, as in
 *   a singly-linked list.
 */
typedef struct example_self_referential_structure {
    /** The value held by this node. */
    int32_t value;
    /** A pointer to the next node, or null if this is the last node. */
    struct example_self_referential_structure* next;
} example_self_referential_structure_t;

typedef uint8_t example_slice_type_t[16];

/** A simple C-compatible enum with unit variants and no explicit discriminant values. */
typedef enum example_enum {
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
typedef struct example_structure {
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

typedef struct example_ref_structure {
    const example_structure_t* option_ref;
    const example_structure_t* ref_ptr;
    example_structure_t* mut_option_ref;
    example_structure_t* mut_ref_ptr;
} example_ref_structure_t;

/** A C-compatible enum with explicitly assigned discriminant values, including gaps in the numbering. */
typedef enum example_enum_with_values {
    /** Default variant with value 0. */
    EXAMPLE_ENUM_WITH_VALUES_NONE = 0,
    /** Variant with an explicit value of 10. */
    EXAMPLE_ENUM_WITH_VALUES_FOO = 10,
    /** Variant with an auto-incremented value. */
    EXAMPLE_ENUM_WITH_VALUES_BAR,
    /** Variant with a large explicit value. */
    EXAMPLE_ENUM_WITH_VALUES_BAZ = 2544
} example_enum_with_values_e;

/** A struct only used inside a data-carrying enum variant, not referenced by any function. */
typedef struct union_only_struct {
    /** An x coordinate. */
    float x;
    /** A y coordinate. */
    float y;
} union_only_struct_t;

/** An enum with data-carrying variants, emitted as a tagged union in C.
 *
 * The `DiscriminantEnum` derive generates a companion `ExampleDataEnumKind` enum containing only the discriminant tags.
 */
typedef enum example_data_enum_kind {
    /** Empty variant with no associated data. */
    EXAMPLE_DATA_ENUM_KIND_NONE,
    /** Variant carrying a single unsigned 32-bit integer. */
    EXAMPLE_DATA_ENUM_KIND_FOO,
    /** Variant carrying an enum value. */
    EXAMPLE_DATA_ENUM_KIND_BAR,
    /** Variant carrying a full structure. */
    EXAMPLE_DATA_ENUM_KIND_BAZ,
    /** Variant carrying a struct only used in this enum. */
    EXAMPLE_DATA_ENUM_KIND_QUX
} example_data_enum_kind_e;

/** An enum with data-carrying variants, emitted as a tagged union in C.
 *
 * The `DiscriminantEnum` derive generates a companion `ExampleDataEnumKind` enum containing only the discriminant tags.
 */
typedef struct example_data_enum {
    example_data_enum_kind_e kind;
    union {
        uint32_t foo;
        example_enum_e bar;
        example_structure_t baz;
        union_only_struct_t qux;
    };
} example_data_enum_t;

/** An enum with data-carrying variants, explicit discriminant values, and a fixed `u32` tag width. Demonstrates that
 *   variant values are preserved in the generated C discriminant enum.
 */
typedef struct example_data_enum_with_values {
    example_data_enum_kind_e kind;
    union {
        uint32_t foo;
        example_enum_e bar;
        example_structure_t baz;
    };
} example_data_enum_with_values_t;

/** Sentinel meaning "no maximum" — non-ASCII text like § must survive generation intact, and the value must emit as
 *   a C `<stdint.h>` macro.
 */
#define EXAMPLE_NO_MAX ((uint64_t)UINT64_MAX)

/** Sentinel meaning "no minimum". */
#define EXAMPLE_NO_MIN ((int32_t)INT32_MIN)

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

/** One half of a mutually referential pair. Neither type's typedef can be complete before the other is named, so the
 *   first one emitted refers to its peer through the struct tag.
 */
typedef struct example_mutual_a {
    /** A value held by this node. */
    int32_t value;
    /** A pointer to the peer, or null. */
    struct example_mutual_b* peer;
} example_mutual_a_t;

/** The other half of the mutually referential pair. By the time this is emitted its peer is declared, so it uses the
 *   plain typedef name.
 */
typedef struct example_mutual_b {
    /** A value held by this node. */
    int32_t value;
    /** A pointer to the peer, or null. */
    example_mutual_a_t* peer;
} example_mutual_b_t;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/** Accepts an enum by value.
 *
 * @param v The enum value to process.
 */
DLL_API void param_enum(example_enum_with_values_e v);

/** Accepts a const pointer to a structure.
 *
 * @param p A non-null const pointer to an @ref ExampleStructure.
 */
DLL_API void param_pointer(const example_structure_t* p);

/** Accepts a mutable pointer to a structure.
 *
 * @param p A non-null mutable pointer to an @ref ExampleStructure.
 */
DLL_API void param_pointer_mut(example_structure_t* p);

/** Accepts an optional immutable reference, emitted as a nullable const pointer in C.
 *
 * @param p An optional reference to an @ref ExampleStructure, or `None` for null.
 */
DLL_API void param_option(const example_structure_t* p);

/** Accepts an optional mutable reference, emitted as a nullable pointer in C.
 *
 * @param p An optional mutable reference to an @ref ExampleStructure, or `None` for null.
 */
DLL_API void param_option_mut(example_structure_t* p);

/** Accepts an optional owned C string, emitted as a nullable `char*` in C.
 *
 * @param p An optional owned C string pointer, or `None` for null.
 */
DLL_API void param_string(const char* p);

/** Accepts a const pointer to a tagged union.
 *
 * @param p A non-null const pointer to an @ref ExampleDataEnumWithValues.
 *
 * @return A 32-bit integer status code. Returns `0` on success.
 */
DLL_API int32_t param_tagged_union(const example_data_enum_with_values_t* p);

/** Accepts a bitflags value by copy.
 *
 * @param f A set of @ref ExampleFlags bitflags.
 */
DLL_API void param_flags(example_flags_t f);

/** Accepts an opaque struct by pointer
 */
DLL_API void param_opaque_struct_pointer(const opaque_struct_t* f);

/** Creates an owned string
 */
DLL_API c_string_buffer_t string_new(void);

/** Frees an owned string
 */
DLL_API void string_free(c_string_buffer_t* str);


#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif
