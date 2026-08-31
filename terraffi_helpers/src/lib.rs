//! Attribute inspection shared between `terraffi_macro` and `terraffi_gen`.
//!
//! Both crates need to agree on what makes an item exportable, so the predicates
//! live here rather than being written twice. This is an implementation detail of
//! terraffi and carries no stability guarantee; depend on it directly at your own
//! risk.

#![warn(missing_docs)]
/// Returns true if any attribute in the list is `#[terraffi_export]`.
pub fn has_terraffi_export(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr.path().is_ident("terraffi_export"))
}

/// Returns true if any attribute in the list is `#[terraffi_ignore]`.
pub fn has_terraffi_ignore(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr.path().is_ident("terraffi_ignore"))
}

/// Returns true if the attribute list contains `#[repr(C)]`,
/// including combined forms like `#[repr(C, u32)]`.
pub fn is_repr_c(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("repr")
            && let Ok(args) = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )
        {
            return args.iter().any(|meta| meta.path().is_ident("C"));
        }
        false
    })
}

/// Returns true if the attribute list contains `#[repr(transparent)]`.
pub fn is_repr_transparent(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("repr")
            && let Ok(args) = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )
        {
            return args.iter().any(|meta| meta.path().is_ident("transparent"));
        }
        false
    })
}

/// Returns true if the attribute list contains `#[no_mangle]` or `#[unsafe(no_mangle)]`.
pub fn has_no_mangle(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        // #[no_mangle]
        if attr.path().is_ident("no_mangle") {
            return true;
        }
        // #[unsafe(no_mangle)]
        if attr.path().is_ident("unsafe")
            && let syn::Meta::List(meta_list) = &attr.meta
            && let Ok(inner) = meta_list.parse_args::<syn::Meta>()
        {
            return inner.path().is_ident("no_mangle");
        }
        false
    })
}

/// Returns true if the given path refers to `Option` from `std` or `core`.
///
/// Matches: `Option`, `std::Option`, `core::Option`,
/// `std::option::Option`, `core::option::Option`.
pub fn is_std_option(path: &syn::Path) -> bool {
    let segments: Vec<_> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    match segments.as_slice() {
        [only] => only == "Option",
        [a, b] if b == "Option" => a == "std" || a == "core",
        [a, b, c] if c == "Option" => (a == "std" || a == "core") && b == "option",
        _ => false,
    }
}

/// Converts a Rust integer literal string to its C equivalent.
///
/// - Strips type suffixes (e.g. `0x1u32` -> `0x1`)
/// - Converts binary literals to hex (e.g. `0b0001` -> `0x1`)
/// - Strips underscores from decimal and hex literals
pub fn rust_literal_to_c(s: &str) -> String {
    // Strip type suffix (e.g. 0x1u32 -> 0x1)
    let stripped = s.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '_');
    let stripped = if stripped.is_empty() { s } else { stripped };

    if let Some(bin) = stripped.strip_prefix("0b") {
        let digits: String = bin.chars().filter(|c| *c != '_').collect();
        if let Ok(v) = u64::from_str_radix(&digits, 2) {
            return format!("0x{v:X}");
        }
    }
    // Rust hex (0x) and decimal are valid C, just strip underscores
    stripped.replace('_', "")
}

/// Maps a Rust integer `{type}::MAX` / `{type}::MIN` associated constant to the
/// equivalent C `<stdint.h>` macro (e.g. `u64::MAX` -> `UINT64_MAX`), or returns
/// `None` when the pair is not one of them.
fn rust_int_limit_to_c(ty: &str, limit: &str) -> Option<&'static str> {
    Some(match (ty, limit) {
        ("u8", "MAX") => "UINT8_MAX",
        ("u16", "MAX") => "UINT16_MAX",
        ("u32", "MAX") => "UINT32_MAX",
        ("u64", "MAX") => "UINT64_MAX",
        ("usize", "MAX") => "SIZE_MAX",
        ("u8" | "u16" | "u32" | "u64" | "usize", "MIN") => "0",
        ("i8", "MAX") => "INT8_MAX",
        ("i16", "MAX") => "INT16_MAX",
        ("i32", "MAX") => "INT32_MAX",
        ("i64", "MAX") => "INT64_MAX",
        ("isize", "MAX") => "INTPTR_MAX",
        ("i8", "MIN") => "INT8_MIN",
        ("i16", "MIN") => "INT16_MIN",
        ("i32", "MIN") => "INT32_MIN",
        ("i64", "MIN") => "INT64_MIN",
        ("isize", "MIN") => "INTPTR_MIN",
        _ => return None,
    })
}

/// Converts a vector of `proc_macro2` token trees representing a Rust literal
/// expression into a C literal string.
pub fn rust_literal_tokens_to_c(tokens: Vec<proc_macro2::TokenTree>) -> String {
    use proc_macro2::TokenTree;
    // If it's a single literal, convert it
    if tokens.len() == 1
        && let TokenTree::Literal(lit) = &tokens[0]
    {
        return rust_literal_to_c(&lit.to_string());
    }
    // Otherwise stringify as-is, converting literals and integer limit
    // associated constants (`u64::MAX` and friends, which mean nothing in C)
    let mut result = String::new();
    let mut i = 0;
    while i < tokens.len() {
        // `u64::MAX` arrives as [Ident, Punct(':'), Punct(':'), Ident]
        if let Some(
            [
                TokenTree::Ident(ty),
                TokenTree::Punct(p1),
                TokenTree::Punct(p2),
                TokenTree::Ident(limit),
            ],
        ) = tokens.get(i..i + 4)
            && p1.as_char() == ':'
            && p2.as_char() == ':'
            && let Some(c_limit) = rust_int_limit_to_c(&ty.to_string(), &limit.to_string())
        {
            result.push_str(c_limit);
            i += 4;
            continue;
        }
        match &tokens[i] {
            TokenTree::Literal(lit) => result.push_str(&rust_literal_to_c(&lit.to_string())),
            other => result.push_str(&other.to_string()),
        }
        i += 1;
    }
    result
}

/// Returns true if the item has `#[derive(DiscriminantEnum)]`.
pub fn has_discriminant_enum_derive(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("derive")
            && let syn::Meta::List(meta_list) = &attr.meta
        {
            let tokens: Vec<proc_macro2::TokenTree> =
                meta_list.tokens.clone().into_iter().collect();
            return tokens.iter().any(|t| {
                if let proc_macro2::TokenTree::Ident(ident) = t {
                    ident == "DiscriminantEnum"
                } else {
                    false
                }
            });
        }
        false
    })
}

/// Extracts the kind name from `#[terraffi(discriminant_enum_name = "CustomName")]` helper attribute,
/// or returns `{enum_name}Kind` if no custom name is specified but the derive is present.
pub fn get_terraffi_discriminant_enum_name(attrs: &[syn::Attribute]) -> Option<String> {
    // Check for #[terraffi(discriminant_enum_name = "...")] helper attribute
    for attr in attrs {
        if attr.path().is_ident("terraffi")
            && let syn::Meta::List(meta_list) = &attr.meta
            && let Ok(nested) = meta_list.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )
        {
            for meta in &nested {
                if let syn::Meta::NameValue(nv) = meta
                    && nv.path.is_ident("discriminant_enum_name")
                    && let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                {
                    return Some(s.value());
                }
            }
        }
    }

    None
}
