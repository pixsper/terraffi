//! The naming conventions offered for generated identifiers.

use terraffi_gen::{Case, TerraffiGeneratorBuilder};

fn testlib_path() -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "{}/../terraffi_testlib",
        env!("CARGO_MANIFEST_DIR")
    ))
}

/// How `ExampleEnum` from the test library renders under each case, with the
/// default `_e` enum suffix applied.
const RENDERINGS: &[(Case, &str)] = &[
    (Case::Snake, "example_enum_e"),
    (Case::UpperSnake, "EXAMPLE_ENUM_E"),
    (Case::Pascal, "ExampleEnumE"),
    (Case::Camel, "exampleEnumE"),
    (Case::Ada, "Example_Enum_E"),
    (Case::Flat, "exampleenume"),
    (Case::UpperFlat, "EXAMPLEENUME"),
];

/// Locks the mapping onto the underlying conversion crate, so a change there
/// cannot silently alter generated identifiers.
#[test]
fn each_case_renders_as_documented() {
    for (case, expected) in RENDERINGS {
        let header = TerraffiGeneratorBuilder::new()
            .typename_case(*case)
            .build(testlib_path())
            .generate()
            .expect("test library should generate");

        assert!(
            header.contains(expected),
            "{case:?} should render `ExampleEnum` as `{expected}`"
        );
    }
}

/// Every offered case must yield something C will accept as an identifier.
///
/// The renderings are checked directly rather than by scanning the generated
/// header, because whitespace-separated tokens each look valid on their own:
/// `typedef enum Example Enum E {` would slip past such a scan.
#[test]
fn every_case_is_a_valid_c_identifier() {
    for (case, rendered) in RENDERINGS {
        assert!(
            !rendered.is_empty()
                && !rendered.starts_with(|c: char| c.is_ascii_digit())
                && rendered
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "{case:?} renders `{rendered}`, which is not a valid C identifier"
        );
    }
}

/// Every variant of the public enum must appear above, so a newly added case
/// cannot skip these checks.
#[test]
fn all_variants_are_covered() {
    let case = Case::Snake;
    // Exhaustive match: adding a variant breaks compilation here until it is
    // added to RENDERINGS as well.
    let _name = match case {
        Case::Snake => "example_enum_e",
        Case::UpperSnake => "EXAMPLE_ENUM_E",
        Case::Pascal => "ExampleEnumE",
        Case::Camel => "exampleEnumE",
        Case::Ada => "Example_Enum_E",
        Case::Flat => "exampleenume",
        Case::UpperFlat => "EXAMPLEENUME",
        _ => unreachable!("non_exhaustive requires a wildcard from outside the crate"),
    };
    assert_eq!(RENDERINGS.len(), 7, "RENDERINGS is missing a variant");
}
