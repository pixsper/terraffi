//! Validation of the items terraffi is about to emit.
//!
//! The header generator names any type it does not recognise verbatim, so a Rust
//! type with no C equivalent would become an undefined C typename and the header
//! would fail only when the consumer compiles their C, several steps removed from
//! the cause. This module catches that at generation time instead.

use crate::items::{CItem, CType};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;

/// Rust types that can never cross the FFI boundary, mapped to what to use instead.
///
/// These are matched on the outermost path segment, so `Vec<u8>` and
/// `std::vec::Vec<u8>` both resolve to `Vec`.
fn suggestion_for(rust_type: &str) -> Option<&'static str> {
    Some(match rust_type {
        "String" | "CString" | "OsString" | "PathBuf" => {
            "use `CStringBuffer` for an owned string, or `CStringPtr` for a borrowed one"
        }
        "str" | "CStr" | "OsStr" | "Path" => {
            "use `CStringPtr` or `CStringPtrRef` for a borrowed string"
        }
        "Vec" => {
            "use `CVec<T>` for an owned buffer, or `CSlice<T>` / `CSliceRef<T>` for a borrowed one"
        }
        "VecDeque" | "HashMap" | "BTreeMap" | "HashSet" | "BTreeSet" | "LinkedList"
        | "BinaryHeap" => {
            "Rust collections have no stable C layout; expose the contents as a `CSlice<T>` \
             or a `#[repr(C)]` type"
        }
        "Box" => "use `BoxPtr<T>` for an owned pointer, or `CHandle<T>` for an opaque handle",
        "Rc" | "Arc" => "use `CHandle<T>`, or expose the value as an opaque pointer",
        "Cell" | "RefCell" | "Mutex" | "RwLock" => {
            "Rust cell and lock types have no stable C layout; expose the guarded value instead"
        }
        "Cow" => "use `CStringBuffer` / `CStringPtr`, or a `#[repr(C)]` type",
        "Result" => "return a `#[repr(C)]` status enum and pass the value back by out-parameter",
        "Range" | "RangeInclusive" | "Duration" | "Instant" | "SystemTime" => {
            "these types have no stable C layout; expose their parts as a `#[repr(C)]` struct"
        }
        _ => return None,
    })
}

/// A single place where a type could not be resolved to a C declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedType {
    /// The Rust type name that could not be resolved.
    pub type_name: String,
    /// Human-readable description of where it was referenced.
    pub location: String,
}

impl UnresolvedType {
    fn describe(&self, out: &mut String) {
        use fmt::Write;

        let _ = write!(out, "  `{}` in {}\n      ", self.type_name, self.location);
        match suggestion_for(&self.type_name) {
            Some(suggestion) => {
                let _ = writeln!(out, "`{}` is not FFI-safe: {suggestion}.", self.type_name);
            }
            None => {
                let _ = writeln!(
                    out,
                    "no C declaration for `{}` was found. Check the spelling, that the type is \
                     `#[repr(C)]` or `#[repr(transparent)]`, and that its crate is being scanned.",
                    self.type_name
                );
            }
        }
    }
}

/// Error returned when the generated header would reference undeclared C types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedTypesError {
    /// Every unresolved reference found, in a stable order.
    pub unresolved: Vec<UnresolvedType>,
}

impl fmt::Display for UnresolvedTypesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.unresolved.len();
        let plural = if count == 1 { "type" } else { "types" };
        writeln!(
            f,
            "terraffi could not resolve {count} {plural} to a C declaration:\n"
        )?;

        let mut body = String::new();
        for item in &self.unresolved {
            item.describe(&mut body);
        }
        f.write_str(&body)?;

        f.write_str(
            "\nEmitting these would produce a header that does not compile. If a type is \
             declared in a header you pull in with `add_include`, list it with \
             `assume_declared(..)`. To downgrade this to a warning, use \
             `allow_unresolved_types()`.\n",
        )
    }
}

impl Error for UnresolvedTypesError {}

/// Collects every type referenced by `items` that `items` does not also declare.
///
/// `assumed_declared` names types the caller has promised are declared elsewhere,
/// typically via an `add_include` header.
pub fn find_unresolved(items: &[CItem], assumed_declared: &HashSet<String>) -> Vec<UnresolvedType> {
    // Every type this header will declare. Functions declare no type.
    let declared: HashSet<&str> = items
        .iter()
        .filter(|i| !matches!(i, CItem::Function(_)))
        .map(|i| i.name())
        .collect();

    let mut found = Vec::new();
    let mut seen = HashSet::new();

    let mut check = |ty: &CType, location: &str, found: &mut Vec<UnresolvedType>| {
        let mut names = HashSet::new();
        ty.collect_referenced_type_names(&mut names);

        // Sort so the report is deterministic regardless of hash iteration order.
        let mut names: Vec<&String> = names.iter().collect();
        names.sort();

        for name in names {
            if declared.contains(name.as_str()) || assumed_declared.contains(name) {
                continue;
            }
            if seen.insert((name.clone(), location.to_string())) {
                found.push(UnresolvedType {
                    type_name: name.clone(),
                    location: location.to_string(),
                });
            }
        }
    };

    for item in items {
        match item {
            CItem::Struct(s) => {
                for field in &s.fields {
                    check(
                        &field.ty,
                        &format!("struct `{}`, field `{}`", s.name, field.name),
                        &mut found,
                    );
                }
            }
            CItem::TaggedUnion(u) => {
                for variant in &u.variants {
                    for field in &variant.fields {
                        check(
                            &field.ty,
                            &format!(
                                "union `{}`, variant `{}`, field `{}`",
                                u.name, variant.name, field.name
                            ),
                            &mut found,
                        );
                    }
                }
            }
            CItem::Function(func) => {
                for param in &func.params {
                    check(
                        &param.ty,
                        &format!("function `{}`, parameter `{}`", func.name, param.name),
                        &mut found,
                    );
                }
                check(
                    &func.return_type,
                    &format!("function `{}`, return type", func.name),
                    &mut found,
                );
            }
            CItem::Typedef(t) => {
                if let Some(inner) = &t.inner_type {
                    check(inner, &format!("typedef `{}`", t.name), &mut found);
                }
            }
            CItem::Bitflags(b) => {
                check(
                    &b.underlying_type,
                    &format!("bitflags `{}`", b.name),
                    &mut found,
                );
            }
            CItem::Const(c) => {
                check(&c.ty, &format!("constant `{}`", c.name), &mut found);
            }
            // Enums carry only discriminants, and macros are emitted verbatim.
            CItem::Enum(_) | CItem::Macro(_) => {}
        }
    }

    found
}

/// Why a type cannot be emitted in the position it was used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnrepresentableReason {
    /// A slice or vector where the length has nowhere to go.
    SliceOrVector,
    /// A unit or tuple type, which becomes `void` and is only valid as a return type.
    Void,
}

/// A type used somewhere C cannot represent it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrepresentableType {
    /// Human-readable description of where it was used.
    pub location: String,
    /// Why it cannot be emitted there.
    pub reason: UnrepresentableReason,
}

/// Error returned when an item uses a type C has no valid spelling for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrepresentableTypesError {
    /// Every offending use, in a stable order.
    pub unrepresentable: Vec<UnrepresentableType>,
}

const SLICE_EXPLANATION: &str = "\n\
    `CSlice`, `CSliceRef`, `CSliceMutRef`, `CVec`, `CVecRef` and `CVecMutRef` are a pointer \
    and a length (and a capacity, for vectors). Terraffi represents them by expanding into \
    adjacent members, which it can only do directly in a struct field or a tagged union \
    variant. In any other position - a function signature, a type alias, behind a pointer, \
    or inside an array - it would emit the pointer alone, dropping the length and \
    mismatching the ABI.\n\n\
    Use `CArrayPtr<T>` or `CArrayPtrMut<T>` with a separate `usize` length, or move the \
    slice into a `#[repr(C)]` struct and use a pointer to that.\n";

const VOID_EXPLANATION: &str = "\n\
    Rust's unit type `()` and tuples have no C representation, so terraffi emits them as \
    `void`. That is only valid as a function's return type: C rejects a `void` field and a \
    `void` parameter as incomplete types.\n\n\
    Remove the field, or give it a concrete `#[repr(C)]` type. A pointer to void is \
    unaffected - `*const c_void` remains `const void*`.\n";

impl fmt::Display for UnrepresentableTypesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.unrepresentable.len();
        let plural = if count == 1 { "use" } else { "uses" };
        writeln!(
            f,
            "terraffi found {count} {plural} of a type C cannot represent where it was used:\n"
        )?;
        for item in &self.unrepresentable {
            writeln!(f, "  {}", item.location)?;
        }

        // Only explain the reasons that actually occurred.
        let has = |r| self.unrepresentable.iter().any(|u| u.reason == r);
        if has(UnrepresentableReason::SliceOrVector) {
            f.write_str(SLICE_EXPLANATION)?;
        }
        if has(UnrepresentableReason::Void) {
            f.write_str(VOID_EXPLANATION)?;
        }
        Ok(())
    }
}

impl Error for UnrepresentableTypesError {}

/// Returns true if `ty` contains a slice or vector at any depth.
fn contains_slice_or_vec(ty: &CType) -> bool {
    match ty {
        CType::Slice { .. } | CType::Vec { .. } => true,
        CType::Pointer { inner, .. } | CType::Array { inner, .. } => contains_slice_or_vec(inner),
        CType::FnPointer {
            return_type,
            params,
        } => contains_slice_or_vec(return_type) || params.iter().any(contains_slice_or_vec),
        _ => false,
    }
}

/// Returns true if `ty` is `void`, or an array of it.
///
/// Rust's unit type and tuples convert to `void`, which C accepts only as a
/// function's return type. A pointer to void is fine at any depth, so this
/// deliberately does not recurse through `CType::Pointer`.
fn is_bare_void(ty: &CType) -> bool {
    match ty {
        CType::Void => true,
        CType::Array { inner, .. } => is_bare_void(inner),
        _ => false,
    }
}

/// Returns true if a field of type `ty` cannot be emitted correctly.
///
/// A slice or vector directly in field position is fine: terraffi expands it into
/// adjacent members. Nested anywhere else - behind a pointer, inside an array, or
/// as the element of another slice - there is nowhere to put the length.
fn field_is_unrepresentable(ty: &CType) -> bool {
    match ty {
        CType::Slice { inner, .. } | CType::Vec { inner, .. } => contains_slice_or_vec(inner),
        other => contains_slice_or_vec(other),
    }
}

/// Finds slice and vector types used where C has no way to carry the length.
///
/// Terraffi represents them by expanding into adjacent members, which it can only
/// do directly in a struct field or a tagged union variant. Every other position -
/// a function signature, a type alias, behind a pointer, inside an array - would
/// emit the pointer alone, dropping the length and mismatching the ABI.
pub fn find_unrepresentable(items: &[CItem]) -> Vec<UnrepresentableType> {
    let mut found = Vec::new();
    let report = |cond: bool,
                  location: String,
                  reason: UnrepresentableReason,
                  found: &mut Vec<UnrepresentableType>| {
        if cond {
            found.push(UnrepresentableType { location, reason });
        }
    };

    for item in items {
        match item {
            CItem::Function(func) => {
                for param in &func.params {
                    let where_ = format!("function `{}`, parameter `{}`", func.name, param.name);
                    report(
                        contains_slice_or_vec(&param.ty),
                        where_.clone(),
                        UnrepresentableReason::SliceOrVector,
                        &mut found,
                    );
                    report(
                        is_bare_void(&param.ty),
                        where_,
                        UnrepresentableReason::Void,
                        &mut found,
                    );
                }
                // `void` is legal as a return type, so only the slice rule applies here.
                report(
                    contains_slice_or_vec(&func.return_type),
                    format!("function `{}`, return type", func.name),
                    UnrepresentableReason::SliceOrVector,
                    &mut found,
                );
            }
            CItem::Typedef(t) => {
                if let Some(inner) = &t.inner_type {
                    let where_ = format!("type alias `{}`", t.name);
                    report(
                        contains_slice_or_vec(inner),
                        where_.clone(),
                        UnrepresentableReason::SliceOrVector,
                        &mut found,
                    );
                    report(
                        is_bare_void(inner),
                        where_,
                        UnrepresentableReason::Void,
                        &mut found,
                    );
                }
            }
            CItem::Struct(st) => {
                for field in &st.fields {
                    let where_ = format!("struct `{}`, field `{}`", st.name, field.name);
                    report(
                        field_is_unrepresentable(&field.ty),
                        where_.clone(),
                        UnrepresentableReason::SliceOrVector,
                        &mut found,
                    );
                    report(
                        is_bare_void(&field.ty),
                        where_,
                        UnrepresentableReason::Void,
                        &mut found,
                    );
                }
            }
            CItem::TaggedUnion(u) => {
                for variant in &u.variants {
                    for field in &variant.fields {
                        let where_ = format!(
                            "union `{}`, variant `{}`, field `{}`",
                            u.name, variant.name, field.name
                        );
                        report(
                            field_is_unrepresentable(&field.ty),
                            where_.clone(),
                            UnrepresentableReason::SliceOrVector,
                            &mut found,
                        );
                        report(
                            is_bare_void(&field.ty),
                            where_,
                            UnrepresentableReason::Void,
                            &mut found,
                        );
                    }
                }
            }
            CItem::Const(c) => {
                let where_ = format!("constant `{}`", c.name);
                report(
                    contains_slice_or_vec(&c.ty),
                    where_.clone(),
                    UnrepresentableReason::SliceOrVector,
                    &mut found,
                );
                report(
                    is_bare_void(&c.ty),
                    where_,
                    UnrepresentableReason::Void,
                    &mut found,
                );
            }
            CItem::Enum(_) | CItem::Bitflags(_) | CItem::Macro(_) => {}
        }
    }
    found
}
