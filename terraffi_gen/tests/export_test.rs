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
