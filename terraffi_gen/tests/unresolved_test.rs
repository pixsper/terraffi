//! Validation of types the generated header would reference but never declare.

use terraffi_gen::TerraffiGeneratorBuilder;

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "{}/tests/fixtures/unresolved",
        env!("CARGO_MANIFEST_DIR")
    ))
}

/// The fixture is not a real package, so dependency scanning (which shells out to
/// cargo metadata) is switched off for every test here.
fn builder() -> TerraffiGeneratorBuilder {
    TerraffiGeneratorBuilder::new()
        .disable_scan_dependencies()
        .export_public_types()
}

fn generate_err() -> String {
    builder()
        .build(fixture_path())
        .generate()
        .expect_err("expected unresolved types to be rejected")
        .to_string()
}

#[test]
fn unsupported_std_types_are_rejected_with_a_suggestion() {
    let msg = generate_err();

    assert!(msg.contains("`String`"), "String not reported: {msg}");
    assert!(
        msg.contains("CStringBuffer"),
        "String suggestion missing: {msg}"
    );
    assert!(msg.contains("`Vec`"), "Vec not reported: {msg}");
    assert!(msg.contains("CVec<T>"), "Vec suggestion missing: {msg}");
    assert!(msg.contains("`HashMap`"), "HashMap not reported: {msg}");
    assert!(msg.contains("`str`"), "str not reported: {msg}");
}

#[test]
fn error_names_the_field_or_parameter() {
    let msg = generate_err();

    assert!(
        msg.contains("struct `BadFields`, field `owned`"),
        "field location missing: {msg}"
    );
    assert!(
        msg.contains("function `takes_string`, parameter `s`"),
        "parameter location missing: {msg}"
    );
}

#[test]
fn unknown_type_gets_the_generic_message() {
    let msg = generate_err();

    assert!(msg.contains("`NoSuchType`"), "typo not reported: {msg}");
    assert!(
        msg.contains("Check the spelling"),
        "generic guidance missing: {msg}"
    );
}

/// A non-`repr(C)` type used behind a pointer is emitted as an opaque forward
/// declaration, so it is declared and must never be reported.
#[test]
fn opaque_handles_are_not_reported() {
    let msg = generate_err();

    assert!(
        !msg.contains("OpaqueHandle"),
        "opaque handle wrongly reported: {msg}"
    );
}

#[test]
fn assume_declared_suppresses_only_the_named_type() {
    let msg = builder()
        .assume_declared("NoSuchType")
        .build(fixture_path())
        .generate()
        .expect_err("other unresolved types should still fail")
        .to_string();

    assert!(
        !msg.contains("NoSuchType"),
        "assume_declared did not suppress the type: {msg}"
    );
    assert!(msg.contains("`String`"), "other types should remain: {msg}");
}

#[test]
fn assume_declared_for_every_type_allows_generation() {
    let header = builder()
        .assume_declared("String")
        .assume_declared("Vec")
        .assume_declared("HashMap")
        .assume_declared("str")
        .assume_declared("NoSuchType")
        .build(fixture_path())
        .generate()
        .expect("all unresolved types were assumed declared");

    assert!(header.contains("takes_string"));
}

#[test]
fn allow_unresolved_types_downgrades_to_a_warning() {
    let mut generator = builder().allow_unresolved_types().build(fixture_path());
    let header = generator
        .generate()
        .expect("generation should succeed when unresolved types are allowed");

    assert!(!header.is_empty());

    let reported = generator.unresolved_types();
    assert!(!reported.is_empty(), "expected reported unresolved types");
    assert!(
        reported.iter().any(|u| u.type_name == "String"),
        "String missing from reported types"
    );
    assert!(
        reported.iter().any(|u| u.location.contains("takes_string")),
        "location missing from reported types"
    );
}

/// A clean crate must not trip the check.
#[test]
fn valid_crate_reports_nothing() {
    let mut generator = TerraffiGeneratorBuilder::new()
        .allow_unresolved_types()
        .build(format!(
            "{}/../terraffi_testlib",
            env!("CARGO_MANIFEST_DIR")
        ));

    generator.generate().expect("test library should generate");
    assert!(
        generator.unresolved_types().is_empty(),
        "unexpected unresolved types: {:?}",
        generator.unresolved_types()
    );
}
