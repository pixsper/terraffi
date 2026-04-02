use std::fs;
use std::path::{Path, PathBuf};
use syn::{Attribute, Item};

/// Parses Rust source code in the specified folder and returns all items
/// annotated with #[terraffi_export].
pub fn parse_exports_from_crate(
    crate_dir_path: impl AsRef<Path>,
) -> Result<Vec<Item>, Box<dyn std::error::Error>> {
    todo!()
}

fn collect_rs_files(
    path: impl AsRef<Path>,
    rs_file_paths: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.as_ref();
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            collect_rs_files(entry?.path(), rs_file_paths)?;
        }
    } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
        rs_file_paths.push(path.to_owned());
    }
    Ok(())
}

fn collect_syntax_items(
    file_path: impl AsRef<Path>,
    items: &mut Vec<Item>,
) -> Result<(), Box<dyn std::error::Error>> {
    let src = fs::read_to_string(file_path)?;
    let syntax = syn::parse_file(&src)?;

    for item in syntax.items {
        match item {
            Item::Fn(_) => items.push(item),
            Item::Const(_) => items.push(item),
            Item::Enum(_) => items.push(item),
            Item::Struct(_) => items.push(item),
            Item::Union(_) => items.push(item),
            _ => {}
        }
    }

    Ok(())
}

/// Check if the item has #[terraffi_export] attached
fn has_terraffi_export(item: &Item) -> bool {
    let attrs = match item {
        Item::Fn(f) => &f.attrs,
        Item::Struct(s) => &s.attrs,
        Item::Enum(e) => &e.attrs,
        _ => return false,
    };

    attrs.iter().any(|a| is_terraffi_export_attr(a))
}

/// Detect if an attribute is #[terraffi_export]
fn is_terraffi_export_attr(attr: &Attribute) -> bool {
    attr.path().is_ident("terraffi_export")
}
