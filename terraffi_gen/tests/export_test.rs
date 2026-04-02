use terraffi_gen::parse_exports_from_crate;

#[test]
fn can_export_functions() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let testlib_src = format!("{}/../terraffi_testlib/src", crate_dir);
    let exports = parse_exports_from_crate(&testlib_src).expect("Failed to parse exports");

    assert!(exports.len() > 0);
}
