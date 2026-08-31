//! Slice and vector types used where C cannot carry the length.

use terraffi_gen::{TerraffiError, TerraffiGeneratorBuilder};

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "{}/tests/fixtures/slice_in_signature",
        env!("CARGO_MANIFEST_DIR")
    ))
}

fn builder() -> TerraffiGeneratorBuilder {
    TerraffiGeneratorBuilder::new().disable_scan_dependencies()
}

/// A slice or vector is a pointer plus a length. Emitting only the pointer would
/// drop the length and leave the C caller passing one word where Rust reads two,
/// so generation fails instead.
#[test]
fn slices_in_signatures_are_rejected() {
    let err = builder()
        .build(fixture_path())
        .generate()
        .expect_err("a slice in a signature should be rejected");

    let msg = err.to_string();
    assert!(
        matches!(err, TerraffiError::UnrepresentableTypes(_)),
        "expected UnrepresentableTypes, got: {msg}"
    );
    assert!(
        msg.contains("function `takes_slice_ref`, parameter `s`"),
        "missing the CSliceRef parameter: {msg}"
    );
    assert!(
        msg.contains("function `takes_vec`, parameter `v`"),
        "missing the CVec parameter: {msg}"
    );
    assert!(
        msg.contains("function `returns_slice`, return type"),
        "missing the return type: {msg}"
    );
}

/// The error should name what to do instead, not just what is wrong.
#[test]
fn the_error_suggests_a_replacement() {
    let msg = builder()
        .build(fixture_path())
        .generate()
        .expect_err("should fail")
        .to_string();

    assert!(msg.contains("CArrayPtr"), "no suggested replacement: {msg}");
}

/// Pointer-sized types, pointers to structs holding a slice, and an explicit
/// pointer/length pair all remain legal.
#[test]
fn legal_signature_forms_are_not_reported() {
    let msg = builder()
        .build(fixture_path())
        .generate()
        .expect_err("should fail")
        .to_string();

    for legal in ["takes_ptr_and_len", "takes_string", "takes_holder"] {
        assert!(
            !msg.contains(legal),
            "`{legal}` should be legal, got: {msg}"
        );
    }
}

/// The rule applies to signatures only; a slice as a struct field still works.
#[test]
fn slices_remain_legal_as_struct_fields() {
    let header = TerraffiGeneratorBuilder::new()
        .build(format!(
            "{}/../terraffi_testlib",
            env!("CARGO_MANIFEST_DIR")
        ))
        .generate()
        .expect("the test library uses slices as fields and should generate");

    assert!(header.contains("size_t array_member_len;"));
}

/// A C typedef has no way to expand into a pointer and a length.
#[test]
fn slice_type_aliases_are_rejected() {
    let msg = builder()
        .build(fixture_path())
        .generate()
        .expect_err("should fail")
        .to_string();

    assert!(
        msg.contains("type alias `AliasedSlice`"),
        "alias not reported: {msg}"
    );
}

/// Nested inside a field there is nowhere to put the length, even though a slice
/// directly in field position is fine.
#[test]
fn slices_nested_inside_fields_are_rejected() {
    let msg = builder()
        .build(fixture_path())
        .generate()
        .expect_err("should fail")
        .to_string();

    assert!(
        msg.contains("struct `BehindPointer`, field `items`"),
        "pointer-to-slice not reported: {msg}"
    );
    assert!(
        msg.contains("struct `InsideArray`, field `items`"),
        "array-of-slices not reported: {msg}"
    );
}

/// A tagged union variant expands exactly like a struct field, so it stays legal.
#[test]
fn slices_remain_legal_in_tagged_union_variants() {
    let msg = builder()
        .build(fixture_path())
        .generate()
        .expect_err("should fail")
        .to_string();

    assert!(
        !msg.contains("Tagged"),
        "a slice in a union variant should be legal: {msg}"
    );
}

/// The legal positions must still emit the expanded members.
#[test]
fn legal_positions_expand_into_adjacent_members() {
    let header = TerraffiGeneratorBuilder::new()
        .build(format!(
            "{}/../terraffi_testlib",
            env!("CARGO_MANIFEST_DIR")
        ))
        .generate()
        .expect("the test library should generate");

    assert!(header.contains("    int32_t* array_member;"));
    assert!(header.contains("    size_t array_member_len;"));
}

/// Rust's unit and tuple types become `void`, which C accepts only as a return
/// type. A `void` field or parameter is an incomplete type and will not compile.
#[test]
fn bare_void_is_rejected_outside_return_position() {
    let msg = builder()
        .build(fixture_path())
        .generate()
        .expect_err("should fail")
        .to_string();

    assert!(
        msg.contains("struct `HasUnitField`, field `nothing`"),
        "unit field not reported: {msg}"
    );
    assert!(
        msg.contains("function `takes_a_tuple`, parameter `pair`"),
        "tuple parameter not reported: {msg}"
    );
    assert!(
        msg.contains("struct `HasVoidArray`, field `items`"),
        "array of unit not reported: {msg}"
    );
}

/// `void` as a return type and `void*` anywhere must stay legal.
#[test]
fn legal_void_positions_are_not_reported() {
    let msg = builder()
        .build(fixture_path())
        .generate()
        .expect_err("should fail")
        .to_string();

    assert!(
        !msg.contains("returns_nothing"),
        "a unit return type should be legal: {msg}"
    );
    assert!(
        !msg.contains("takes_void_pointer"),
        "a pointer to void should be legal: {msg}"
    );
}

/// Each failure carries why it failed, so a caller can tell them apart.
#[test]
fn failures_carry_their_reason() {
    use terraffi_gen::UnrepresentableReason;

    let err = builder()
        .build(fixture_path())
        .generate()
        .expect_err("should fail");

    let TerraffiError::UnrepresentableTypes(e) = err else {
        panic!("expected UnrepresentableTypes");
    };
    assert!(
        e.unrepresentable
            .iter()
            .any(|u| u.reason == UnrepresentableReason::Void),
        "no void reason recorded"
    );
    assert!(
        e.unrepresentable
            .iter()
            .any(|u| u.reason == UnrepresentableReason::SliceOrVector),
        "no slice reason recorded"
    );
}
