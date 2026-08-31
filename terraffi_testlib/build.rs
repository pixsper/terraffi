fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let crate_name = std::path::Path::new(&crate_dir)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_filename = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap()
        .join(format!("{}.h", crate_name.to_ascii_lowercase()));

    let header = terraffi_gen::TerraffiGeneratorBuilder::new()
        .build(crate_dir)
        .generate()
        .unwrap();

    std::fs::write(out_filename, header).unwrap();
}
