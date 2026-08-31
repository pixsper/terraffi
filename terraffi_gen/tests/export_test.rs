use terraffi_gen::{Case, TerraffiGeneratorBuilder};

fn testlib_path() -> std::path::PathBuf {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    std::path::PathBuf::from(format!("{}/../terraffi_testlib", crate_dir))
}

#[test]
fn can_generate_header() {
    let header = TerraffiGeneratorBuilder::new()
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");

    let expected = include_str!("expected_header.h");
    assert_eq!(header, expected);
}

#[test]
fn can_generate_header_with_prefix() {
    let header = TerraffiGeneratorBuilder::new()
        .typename_prefix("test_")
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");

    let expected = include_str!("expected_header_prefixed.h");
    assert_eq!(header, expected);
}

#[test]
fn can_generate_header_with_export_macro() {
    let header = TerraffiGeneratorBuilder::new()
        .export_macro("DLL_API")
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");

    let expected = include_str!("expected_header_export_macro.h");
    assert_eq!(header, expected);
}

#[test]
fn can_generate_header_with_comment() {
    let header = TerraffiGeneratorBuilder::new()
        .header_comment("This file is auto-generated.\nDo not edit manually.")
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");

    let expected = include_str!("expected_header_comment.h");
    assert_eq!(header, expected);
}

#[test]
fn can_generate_header_with_pascal_case() {
    let header = TerraffiGeneratorBuilder::new()
        .typename_case(Case::Pascal)
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");

    let expected = include_str!("expected_header_pascal.h");
    assert_eq!(header, expected);
}

/// `ExampleDependencyStruct` is defined in `terraffi_testdeplib`, a dependency of
/// the test library. Scanning is on by default, so this is paired with the test
/// below: checking only the disabled case would pass even if the setter did
/// nothing at all.
#[test]
fn dependency_scanning_is_on_by_default() {
    let header = TerraffiGeneratorBuilder::new()
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");

    assert!(
        header.contains("typedef struct example_dependency_struct {"),
        "expected the dependency type to be defined when scanning is enabled"
    );
}

/// Dropping the dependency scan leaves `ExampleDependencyStruct` referenced but
/// undeclared, which is exactly what unresolved-type validation exists to catch.
#[test]
fn disable_scan_dependencies_alone_is_rejected() {
    let err = TerraffiGeneratorBuilder::new()
        .disable_scan_dependencies()
        .build(testlib_path())
        .generate()
        .expect_err("expected the undeclared dependency type to be rejected");

    let msg = err.to_string();
    assert!(
        msg.contains("ExampleDependencyStruct"),
        "error should name the undeclared type, got: {msg}"
    );
}

/// The supported workflow: skip the dependency scan and promise the type is
/// declared by a header supplied through `add_include`.
#[test]
fn disable_scan_dependencies_omits_dependency_types() {
    let header = TerraffiGeneratorBuilder::new()
        .disable_scan_dependencies()
        .assume_declared("ExampleDependencyStruct")
        .add_include("terraffi_testdeplib.h")
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");

    assert!(
        !header.contains("typedef struct example_dependency_struct {"),
        "dependency type was still defined after disable_scan_dependencies()"
    );
    assert!(
        header.contains("example_dependency_struct_t struct_member;"),
        "the field referencing the dependency type should still be emitted"
    );
}

/// `Default` must agree with `new()`, down to the generated header. A divergence
/// between them is silent: both produce a header, just not the same one.
#[test]
fn default_builder_matches_new() {
    assert_eq!(
        TerraffiGeneratorBuilder::default(),
        TerraffiGeneratorBuilder::new()
    );

    let from_default = TerraffiGeneratorBuilder::default()
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");
    let from_new = TerraffiGeneratorBuilder::new()
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");

    assert_eq!(from_default, from_new);
    // Guards against both paths being equally broken.
    assert_eq!(from_new, include_str!("expected_header.h"));
}

/// Setting an option to its default value must behave the same as leaving it unset.
#[test]
fn explicitly_setting_a_default_is_a_no_op() {
    let implicit = TerraffiGeneratorBuilder::new()
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");
    let explicit = TerraffiGeneratorBuilder::new()
        .add_std_includes(true)
        .export_public_functions()
        .export_only_annotated_types()
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");

    assert_eq!(implicit, explicit);
}

/// The three ways one struct can appear inside another, and the naming each
/// requires. A `struct` tag is only needed where the typedef is not available
/// yet; everywhere else the plain typedef name is correct.
#[test]
fn struct_members_use_the_right_name_form() {
    let header = TerraffiGeneratorBuilder::new()
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");

    // By value: the typedef is complete, so no `struct` prefix.
    assert!(
        header.contains("    example_dependency_struct_t struct_member;"),
        "a struct member held by value should use the typedef name"
    );

    // Self-referential: the typedef does not exist until the closing brace.
    assert!(
        header.contains("    struct example_self_referential_structure* next;"),
        "a self-referential pointer should use the struct tag"
    );

    // Pointer to an already-declared type: the typedef name is available.
    assert!(
        header.contains("    const example_structure_t* option_ref;"),
        "a pointer to an already-declared struct should use the typedef name"
    );
}

/// Mutually referential structs form a cycle, so no ordering declares both
/// typedefs first. Whichever is emitted first must reach its peer through the
/// struct tag, which doubles as a forward declaration.
#[test]
fn mutually_recursive_structs_forward_reference_by_tag() {
    let header = TerraffiGeneratorBuilder::new()
        .build(testlib_path())
        .generate()
        .expect("Failed to generate header");

    let a = header
        .find("typedef struct example_mutual_a {")
        .expect("ExampleMutualA should be emitted");
    let b = header
        .find("typedef struct example_mutual_b {")
        .expect("ExampleMutualB should be emitted");
    assert!(a < b, "this test assumes A is emitted first");

    assert!(
        header.contains("    struct example_mutual_b* peer;"),
        "the forward reference should use the struct tag, got:\n{header}"
    );
    assert!(
        header.contains("    example_mutual_a_t* peer;"),
        "the backward reference should use the typedef name"
    );
}

/// `CStringBufferRef` shares `CStringBuffer`'s layout, so it resolves to the same
/// emitted struct rather than being reported as an unresolved type. The layout
/// invariant this relies on is asserted in `terraffi_ctypes`.
#[test]
fn borrowed_string_buffer_resolves_to_the_owned_struct() {
    let header = TerraffiGeneratorBuilder::new()
        .build(testlib_path())
        .generate()
        .expect("a borrowed string buffer should resolve");

    assert!(
        header.contains("    c_string_buffer_t borrowed;"),
        "CStringBufferRef should emit the shared struct"
    );
    assert!(
        header.contains("    c_string_buffer_t maybe;"),
        "Option<CStringBufferRef> should emit the shared struct"
    );
    // Only one definition, not a near-duplicate for the borrowed form.
    assert_eq!(
        header.matches("typedef struct c_string_buffer {").count(),
        1,
        "the struct should be defined exactly once"
    );
    assert!(
        !header.contains("c_string_buffer_ref"),
        "no separate borrowed struct should be emitted"
    );
}
