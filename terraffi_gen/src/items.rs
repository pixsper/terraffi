use crate::TerraffiConfig;
use crate::convert_case;
use crate::doc::{CDoc, CFunctionDoc, CParamDoc};
use proc_macro2::{Delimiter, TokenTree};
use quote::{ToTokens, quote};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::convert::Infallible;
use std::fmt;
use std::fmt::{Debug, Write};
use std::str::FromStr;
use syn::{Fields, FnArg, ItemEnum, ItemFn, ItemMacro, Type, Visibility};
use terraffi_helpers::{
    get_terraffi_discriminant_enum_name, has_discriminant_enum_derive, has_no_mangle,
    has_terraffi_export, has_terraffi_ignore, is_repr_c, is_repr_transparent, is_std_option,
    rust_literal_tokens_to_c,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TerraffiExportStatus {
    None,
    Public,
    ExplicitlyExported,
    ExplicitlyIgnored,
}

fn export_status_from_attrs(attrs: &[syn::Attribute], vis: &Visibility) -> TerraffiExportStatus {
    if has_terraffi_ignore(attrs) {
        TerraffiExportStatus::ExplicitlyIgnored
    } else if has_terraffi_export(attrs) {
        TerraffiExportStatus::ExplicitlyExported
    } else if matches!(vis, Visibility::Public(_)) {
        TerraffiExportStatus::Public
    } else {
        TerraffiExportStatus::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CType {
    Void,
    Float,
    Double,
    Int8,
    Int16,
    Int32,
    Int64,
    ISize,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    USize,
    Bool,
    Char,
    Pointer {
        is_const: bool,
        inner: Box<CType>,
    },
    Slice {
        is_const: bool,
        inner: Box<CType>,
    },
    Vec {
        is_const: bool,
        inner: Box<CType>,
    },
    Array {
        size: String,
        inner: Box<CType>,
    },
    FnPointer {
        return_type: Box<CType>,
        params: Vec<CType>,
    },
    Named(String),
}

impl CType {
    pub fn format(&self, terraffi_config: &TerraffiConfig) -> Cow<'_, str> {
        match self {
            CType::Void => "void".into(),
            CType::Float => "float".into(),
            CType::Double => "double".into(),
            CType::Int8 => "int8_t".into(),
            CType::Int16 => "int16_t".into(),
            CType::Int32 => "int32_t".into(),
            CType::Int64 => "int64_t".into(),
            CType::ISize => "ptrdiff_t".into(),
            CType::UInt8 => "uint8_t".into(),
            CType::UInt16 => "uint16_t".into(),
            CType::UInt32 => "uint32_t".into(),
            CType::UInt64 => "uint64_t".into(),
            CType::USize => "size_t".into(),
            CType::Bool => "bool".into(),
            CType::Char => "char".into(),
            CType::Array { inner, .. } => {
                // In contexts where format() is used (e.g. function params),
                // arrays decay to pointers.
                format!("{}*", inner.format(terraffi_config)).into()
            }
            CType::Named(name) => terraffi_config.format_type_name(name).into(),
            CType::Pointer { is_const, inner } => {
                let base = inner.format(terraffi_config);
                if *is_const {
                    if matches!(inner.as_ref(), CType::Pointer { .. }) {
                        // East-const for pointer-to-const-pointer: T* const*
                        format!("{base} const*").into()
                    } else {
                        // West-const for pointer-to-const-data: const T*
                        format!("const {base}*").into()
                    }
                } else {
                    format!("{base}*").into()
                }
            }
            CType::Slice { is_const, inner } | CType::Vec { is_const, inner } => {
                let base = inner.format(terraffi_config);
                if *is_const {
                    format!("const {base}*").into()
                } else {
                    format!("{base}*").into()
                }
            }
            CType::FnPointer {
                return_type,
                params,
            } => {
                // Without a name: `ret (*)(params)`
                let ret = return_type.format(terraffi_config);
                let params_str = if params.is_empty() {
                    "void".to_string()
                } else {
                    params
                        .iter()
                        .map(|p| p.format(terraffi_config).into_owned())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                format!("{ret} (*)({params_str})").into()
            }
        }
    }

    /// Formats a type + name declaration. For most types this is `"type name"`,
    /// but for function pointers it produces `"ret (*name)(params)"`.
    pub fn format_named(&self, name: &str, config: &TerraffiConfig) -> String {
        if let CType::FnPointer {
            return_type,
            params,
        } = self
        {
            let ret = return_type.format(config);
            let params_str = if params.is_empty() {
                "void".to_string()
            } else {
                params
                    .iter()
                    .map(|p| p.format(config).into_owned())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!("{ret} (*{name})({params_str})")
        } else if let CType::Array { size, inner } = self {
            format!("{} {}[{}]", inner.format(config), name, size)
        } else {
            format!("{} {}", self.format(config), name)
        }
    }

    pub fn collect_referenced_type_names(&self, names: &mut HashSet<String>) {
        match self {
            CType::Named(name) => {
                names.insert(name.clone());
            }
            CType::Pointer { inner, .. }
            | CType::Slice { inner, .. }
            | CType::Vec { inner, .. }
            | CType::Array { inner, .. } => {
                inner.collect_referenced_type_names(names);
            }
            CType::FnPointer {
                return_type,
                params,
            } => {
                return_type.collect_referenced_type_names(names);
                for p in params {
                    p.collect_referenced_type_names(names);
                }
            }
            _ => {}
        }
    }
}

/// Returns the first `Type` argument from a generic argument list, skipping lifetimes.
fn first_type_arg(
    args: &syn::punctuated::Punctuated<syn::GenericArgument, syn::token::Comma>,
) -> Option<&syn::Type> {
    args.iter().find_map(|arg| match arg {
        syn::GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

impl TryFrom<&Type> for CType {
    type Error = String;

    fn try_from(value: &Type) -> Result<Self, Self::Error> {
        match value {
            Type::BareFn(bare_fn) => {
                let return_type = match &bare_fn.output {
                    syn::ReturnType::Default => CType::Void,
                    syn::ReturnType::Type(_, ty) => ty.as_ref().try_into()?,
                };
                let params = bare_fn
                    .inputs
                    .iter()
                    .map(|arg| (&arg.ty).try_into())
                    .collect::<Result<Vec<CType>, String>>()?;
                Ok(CType::FnPointer {
                    return_type: Box::new(return_type),
                    params,
                })
            }
            Type::Ptr(ptr) => {
                let is_const = ptr.const_token.is_some();
                let inner = ptr.elem.as_ref().try_into()?;
                Ok(CType::Pointer {
                    is_const,

                    inner: Box::new(inner),
                })
            }
            Type::Reference(reference) => {
                let is_const = reference.mutability.is_none();
                let inner = reference.elem.as_ref().try_into()?;
                Ok(CType::Pointer {
                    is_const,

                    inner: Box::new(inner),
                })
            }
            Type::Path(type_path) => {
                if let Some(segment) = type_path.path.segments.last() {
                    let ident = segment.ident.to_string();

                    // Handle Option<&T>, Option<&mut T> as nullable pointers,
                    // and Option<CStringPtr> as char*
                    if is_std_option(&type_path.path)
                        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                        && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
                    {
                        if let syn::Type::Reference(reference) = inner_ty {
                            let is_const = reference.mutability.is_none();
                            let inner = reference.elem.as_ref().try_into()?;
                            return Ok(CType::Pointer {
                                is_const,

                                inner: Box::new(inner),
                            });
                        }
                        // Option<CStringPtr> has the same layout as CStringPtr
                        return inner_ty.try_into();
                    }

                    if ident == "CHandle"
                        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                        && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
                    {
                        let inner = inner_ty.try_into()?;
                        return Ok(CType::Pointer {
                            is_const: false,
                            inner: Box::new(CType::Pointer {
                                is_const: false,
                                inner: Box::new(inner),
                            }),
                        });
                    }

                    if matches!(
                        ident.as_str(),
                        "CArrayPtr" | "CArrayPtrMut" | "CArrayPtrRef" | "CArrayPtrMutRef"
                    ) && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                        && let Some(inner_ty) = first_type_arg(&args.args)
                    {
                        let is_const = matches!(ident.as_str(), "CArrayPtr" | "CArrayPtrRef");
                        let inner = inner_ty.try_into()?;
                        return Ok(CType::Pointer {
                            is_const,

                            inner: Box::new(inner),
                        });
                    }

                    if matches!(ident.as_str(), "CSlice" | "CSliceRef" | "CSliceMutRef")
                        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                        && let Some(inner_ty) = first_type_arg(&args.args)
                    {
                        let is_const = ident != "CSlice" && ident != "CSliceMutRef";
                        let inner = inner_ty.try_into()?;
                        return Ok(CType::Slice {
                            is_const,
                            inner: Box::new(inner),
                        });
                    }

                    if matches!(ident.as_str(), "CVec" | "CVecRef" | "CVecMutRef")
                        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                        && let Some(inner_ty) = first_type_arg(&args.args)
                    {
                        let is_const = ident != "CVec" && ident != "CVecMutRef";
                        let inner = inner_ty.try_into()?;
                        return Ok(CType::Vec {
                            is_const,
                            inner: Box::new(inner),
                        });
                    }

                    Ok(CType::from_str(ident.as_str()).unwrap())
                } else {
                    Ok(CType::Void)
                }
            }
            Type::Array(array) => {
                let inner = array.elem.as_ref().try_into()?;
                let size = array.len.to_token_stream().to_string().replace(' ', "");
                Ok(CType::Array {
                    size,
                    inner: Box::new(inner),
                })
            }
            Type::Tuple(tuple) if tuple.elems.is_empty() => Ok(CType::Void),
            _ => Ok(CType::Void),
        }
    }
}

impl FromStr for CType {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "f32" => CType::Float,
            "f64" => CType::Double,
            "i8" => CType::Int8,
            "i16" => CType::Int16,
            "i32" => CType::Int32,
            "i64" => CType::Int64,
            "isize" => CType::ISize,
            "u8" => CType::UInt8,
            "u16" => CType::UInt16,
            "u32" => CType::UInt32,
            "u64" => CType::UInt64,
            "usize" => CType::USize,
            "bool" => CType::Bool,
            "c_void" => CType::Void,
            "c_char" => CType::Char,
            "CStringPtr" | "CStringPtrRef" => CType::Pointer {
                is_const: true,

                inner: Box::new(CType::Char),
            },
            "CStringPtrMut" | "CStringPtrMutRef" => CType::Pointer {
                is_const: false,

                inner: Box::new(CType::Char),
            },
            other => CType::Named(other.to_string()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CItem {
    Struct(CStruct),
    Enum(CEnum),
    DiscriminatedUnion(CDiscriminatedUnion),
    Bitflags(CBitflags),
    Typedef(CTypedef),
    Const(CConst),
    Function(CFunction),
    #[allow(dead_code)]
    Macro(CMacro),
}

impl CItem {
    pub fn from_item(
        item: &syn::Item,
        source_order: &mut usize,
        config: &TerraffiConfig,
    ) -> Result<Vec<Self>, String> {
        match item {
            syn::Item::Fn(func) => {
                let is_extern_c = func
                    .sig
                    .abi
                    .as_ref()
                    .is_some_and(|abi| abi.name.as_ref().is_some_and(|n| n.value() == "C"));
                if !is_extern_c
                    || !has_no_mangle(&func.attrs)
                    || !matches!(func.vis, Visibility::Public(_))
                {
                    return Ok(Vec::new());
                }
                match CFunction::from_item(func, *source_order) {
                    Ok(f) => Ok(vec![CItem::Function(f)]),
                    Err(e) => Err(e),
                }
            }
            syn::Item::Struct(s) => {
                if is_repr_c(&s.attrs) {
                    match CStruct::from_item(s, *source_order) {
                        Ok(cs) => Ok(vec![CItem::Struct(cs)]),
                        Err(e) => Err(e),
                    }
                } else if is_repr_transparent(&s.attrs) {
                    match CTypedef::from_item(s, *source_order) {
                        Ok(Some(td)) => Ok(vec![CItem::Typedef(td)]),
                        Ok(None) => Ok(Vec::new()),
                        Err(e) => Err(e),
                    }
                } else {
                    // Non-repr(C) struct: emit as an opaque forward declaration
                    // if it ends up being referenced by exported items.
                    Ok(vec![CItem::Typedef(CTypedef {
                        name: s.ident.to_string(),
                        doc: CDoc::from_attrs(&s.attrs),
                        export_status: export_status_from_attrs(&s.attrs, &s.vis),
                        source_order: *source_order,
                        inner_type: None,
                    })])
                }
            }
            syn::Item::Enum(e) => {
                if is_repr_c(&e.attrs) {
                    let has_data_variants =
                        e.variants.iter().any(|v| !matches!(v.fields, Fields::Unit));
                    if has_data_variants {
                        match CDiscriminatedUnion::from_item(e, source_order, config) {
                            Ok((e, du)) => Ok(vec![CItem::Enum(e), CItem::DiscriminatedUnion(du)]),
                            Err(e) => Err(e),
                        }
                    } else {
                        match CEnum::from_item(e, *source_order) {
                            Ok(ce) => Ok(vec![CItem::Enum(ce)]),
                            Err(e) => Err(e),
                        }
                    }
                } else {
                    // Non-repr(C) enum: emit as an opaque forward declaration
                    // if it ends up being referenced by exported items.
                    Ok(vec![CItem::Typedef(CTypedef {
                        name: e.ident.to_string(),
                        doc: CDoc::from_attrs(&e.attrs),
                        export_status: export_status_from_attrs(&e.attrs, &e.vis),
                        source_order: *source_order,
                        inner_type: None,
                    })])
                }
            }
            syn::Item::Type(t) => match CTypedef::from_type_alias(t, *source_order) {
                Ok(Some(td)) => Ok(vec![CItem::Typedef(td)]),
                Ok(None) => Ok(Vec::new()),
                Err(e) => Err(e),
            },
            syn::Item::Macro(item_macro) => {
                if item_macro.mac.path.is_ident("bitflags") {
                    match CBitflags::from_item(item_macro, *source_order) {
                        Ok(bf) => Ok(vec![CItem::Bitflags(bf)]),
                        Err(e) => Err(e),
                    }
                } else {
                    Ok(Vec::new())
                }
            }
            syn::Item::Const(c) => match CConst::from_item(c, *source_order) {
                Ok(Some(cc)) => Ok(vec![CItem::Const(cc)]),
                Ok(None) => Ok(Vec::new()),
                Err(e) => Err(e),
            },
            _ => Ok(Vec::new()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            CItem::Struct(s) => &s.name,
            CItem::Enum(e) => &e.name,
            CItem::DiscriminatedUnion(u) => &u.name,
            CItem::Bitflags(b) => &b.name,
            CItem::Typedef(t) => &t.name,
            CItem::Const(c) => &c.name,
            CItem::Macro(m) => &m.name,
            CItem::Function(f) => &f.name,
        }
    }

    pub fn export_status(&self) -> TerraffiExportStatus {
        match self {
            CItem::Struct(s) => s.export_status,
            CItem::Enum(e) => e.export_status,
            CItem::DiscriminatedUnion(u) => u.export_status,
            CItem::Bitflags(b) => b.export_status,
            CItem::Typedef(t) => t.export_status,
            CItem::Const(c) => c.export_status,
            CItem::Macro(m) => m.export_status,
            CItem::Function(f) => f.export_status,
        }
    }

    pub fn source_order(&self) -> usize {
        match self {
            CItem::Struct(s) => s.source_order,
            CItem::Enum(e) => e.source_order,
            CItem::DiscriminatedUnion(u) => u.source_order,
            CItem::Bitflags(b) => b.source_order,
            CItem::Typedef(t) => t.source_order,
            CItem::Const(c) => c.source_order,
            CItem::Macro(m) => m.source_order,
            CItem::Function(f) => f.source_order,
        }
    }

    pub fn collect_referenced_type_names(&self, names: &mut HashSet<String>) {
        match self {
            CItem::Struct(s) => s.collect_referenced_type_names(names),
            CItem::DiscriminatedUnion(u) => u.collect_referenced_type_names(names),
            CItem::Typedef(t) => {
                if let Some(ty) = &t.inner_type {
                    ty.collect_referenced_type_names(names);
                }
            }
            CItem::Function(f) => f.collect_referenced_type_names(names),
            _ => {}
        }
    }

    pub fn emit(&self, w: &mut impl Write, config: &mut TerraffiConfig) -> fmt::Result {
        match self {
            CItem::Struct(s) => s.emit(w, config),
            CItem::Enum(e) => e.emit(w, config),
            CItem::DiscriminatedUnion(u) => u.emit(w, config),
            CItem::Bitflags(b) => b.emit(w, config),
            CItem::Typedef(t) => t.emit(w, config),
            CItem::Const(c) => c.emit(w, config),
            CItem::Macro(m) => m.emit(w, config),
            CItem::Function(f) => f.emit(w, config),
        }
    }
}

impl PartialOrd for CItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.source_order().cmp(&other.source_order())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CTypedef {
    pub name: String,
    pub doc: CDoc,
    pub export_status: TerraffiExportStatus,
    pub source_order: usize,
    pub inner_type: Option<CType>,
}

impl CTypedef {
    /// Creates a typedef from a `repr(transparent)` struct with a single field
    /// whose type is a primitive or repr(C) type. Returns `Ok(None)` if the
    /// struct doesn't qualify.
    pub fn from_item(s: &syn::ItemStruct, source_order: usize) -> Result<Option<Self>, String> {
        // Must have exactly one field
        let field = match &s.fields {
            Fields::Named(named) if named.named.len() == 1 => named.named.first().unwrap(),
            Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                unnamed.unnamed.first().unwrap()
            }
            _ => return Ok(None),
        };

        // Try to convert the field type — if it resolves to a primitive or Named type, proceed
        let inner_type: CType = match (&field.ty).try_into() {
            Ok(ty) => ty,
            Err(_) => return Ok(None),
        };

        // Skip if the inner type is Void (unknown/unsupported)
        if inner_type == CType::Void {
            return Ok(None);
        }

        let name = s.ident.to_string();
        let doc = CDoc::from_attrs(&s.attrs);
        let export_status = if has_terraffi_ignore(&s.attrs) {
            TerraffiExportStatus::ExplicitlyIgnored
        } else if has_terraffi_export(&s.attrs) {
            TerraffiExportStatus::ExplicitlyExported
        } else if matches!(s.vis, Visibility::Public(_)) {
            TerraffiExportStatus::Public
        } else {
            TerraffiExportStatus::None
        };

        Ok(Some(CTypedef {
            name,
            doc,
            export_status,
            source_order,
            inner_type: Some(inner_type),
        }))
    }

    /// Creates a typedef from a `type Foo = Bar;` type alias.
    /// Returns `Ok(None)` if the aliased type can't be resolved.
    pub fn from_type_alias(t: &syn::ItemType, source_order: usize) -> Result<Option<Self>, String> {
        let inner_type: CType = match (&*t.ty).try_into() {
            Ok(ty) => ty,
            Err(_) => return Ok(None),
        };

        if inner_type == CType::Void {
            return Ok(None);
        }

        let name = t.ident.to_string();
        let doc = CDoc::from_attrs(&t.attrs);
        let export_status = if has_terraffi_ignore(&t.attrs) {
            TerraffiExportStatus::ExplicitlyIgnored
        } else if has_terraffi_export(&t.attrs) {
            TerraffiExportStatus::ExplicitlyExported
        } else if matches!(t.vis, Visibility::Public(_)) {
            TerraffiExportStatus::Public
        } else {
            TerraffiExportStatus::None
        };

        Ok(Some(CTypedef {
            name,
            doc,
            export_status,
            source_order,
            inner_type: Some(inner_type),
        }))
    }

    pub fn emit(&self, w: &mut impl Write, config: &TerraffiConfig) -> fmt::Result {
        let type_name = config.format_type_name(&self.name);
        self.doc.emit_doxygen(w, "")?;
        match &self.inner_type {
            None => writeln!(w, "typedef struct {type_name} {type_name};"),
            Some(ty) => writeln!(w, "typedef {};", ty.format_named(&type_name, config)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CConst {
    pub name: String,
    pub doc: CDoc,
    pub export_status: TerraffiExportStatus,
    pub source_order: usize,
    pub ty: CType,
    pub value: String,
}

impl CConst {
    pub fn from_item(item: &syn::ItemConst, source_order: usize) -> Result<Option<Self>, String> {
        let ty: CType = match (&*item.ty).try_into() {
            Ok(ty) => ty,
            Err(_) => return Ok(None),
        };

        if ty == CType::Void {
            return Ok(None);
        }

        let name = item.ident.to_string();
        let doc = CDoc::from_attrs(&item.attrs);
        let export_status = if has_terraffi_ignore(&item.attrs) {
            TerraffiExportStatus::ExplicitlyIgnored
        } else if has_terraffi_export(&item.attrs) {
            TerraffiExportStatus::ExplicitlyExported
        } else if matches!(item.vis, Visibility::Public(_)) {
            TerraffiExportStatus::Public
        } else {
            TerraffiExportStatus::None
        };

        let value_tokens: Vec<TokenTree> = item.expr.to_token_stream().into_iter().collect();
        let value = rust_literal_tokens_to_c(value_tokens);

        Ok(Some(CConst {
            name,
            doc,
            export_status,
            source_order,
            ty,
            value,
        }))
    }

    pub fn emit(&self, w: &mut impl Write, config: &TerraffiConfig) -> fmt::Result {
        let name = convert_case(
            &format!(
                "{}{}{}",
                config.constant_prefix, self.name, config.constant_suffix
            ),
            config.constant_case,
        );
        let c_type = self.ty.format(config);
        self.doc.emit_doxygen(w, "")?;
        writeln!(w, "#define {name} (({c_type}){})", self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CMacro {
    pub(crate) name: String,
    pub(crate) doc: CDoc,
    pub(crate) definition: String,
    pub(crate) export_status: TerraffiExportStatus,
    pub(crate) source_order: usize,
}

impl From<String> for CMacro {
    fn from(value: String) -> Self {
        // Parse a C-format macro: "#define NAME DEFINITION" or "#define NAME(args) DEFINITION"
        // Optionally preceded by a doc comment: "/** ... */ #define ..."
        let trimmed = value.trim();

        let mut doc_text = String::new();
        let define_str;

        if trimmed.starts_with("/**") {
            if let Some(end) = trimmed.find("*/") {
                doc_text = trimmed[3..end].trim().to_string();
                define_str = trimmed[end + 2..].trim();
            } else {
                define_str = trimmed;
            }
        } else {
            define_str = trimmed;
        }

        let define_str = define_str
            .strip_prefix("#define")
            .unwrap_or(define_str)
            .trim_start();

        // Split into name (with optional parameter list) and definition
        let (name, definition) = if define_str.find('(').is_some() {
            // Macro with parameters: NAME(args) definition
            if let Some(paren_end) = define_str.find(')') {
                let name_and_args = &define_str[..paren_end + 1];
                let def = define_str[paren_end + 1..].trim();
                (name_and_args.to_string(), def.to_string())
            } else {
                // Malformed, treat first token as name
                let mut parts = define_str.splitn(2, char::is_whitespace);
                let name = parts.next().unwrap_or("").to_string();
                let def = parts.next().unwrap_or("").trim().to_string();
                (name, def)
            }
        } else {
            let mut parts = define_str.splitn(2, char::is_whitespace);
            let name = parts.next().unwrap_or("").to_string();
            let def = parts.next().unwrap_or("").trim().to_string();
            (name, def)
        };

        CMacro {
            name,
            doc: CDoc::from_text(doc_text),
            definition,
            export_status: TerraffiExportStatus::ExplicitlyExported,
            source_order: 0,
        }
    }
}

impl From<&str> for CMacro {
    fn from(value: &str) -> Self {
        CMacro::from(value.to_string())
    }
}

impl CMacro {
    pub fn new(
        name: impl Into<String>,
        doc: impl Into<String>,
        definition: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            doc: CDoc::from_text(doc.into()),
            definition: definition.into(),
            export_status: TerraffiExportStatus::ExplicitlyExported,
            source_order: 0,
        }
    }

    pub(crate) fn emit(&self, w: &mut impl Write, _config: &TerraffiConfig) -> fmt::Result {
        self.doc.emit_doxygen(w, "")?;
        writeln!(w, "#define {} {}", self.name, self.definition)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CStruct {
    pub name: String,
    pub doc: CDoc,
    pub export_status: TerraffiExportStatus,
    pub source_order: usize,
    pub fields: Vec<CStructField>,
}

impl CStruct {
    pub fn from_item(s: &syn::ItemStruct, source_order: usize) -> Result<Self, String> {
        let name = s.ident.to_string();
        let export_status = if has_terraffi_ignore(&s.attrs) {
            TerraffiExportStatus::ExplicitlyIgnored
        } else if has_terraffi_export(&s.attrs) {
            TerraffiExportStatus::ExplicitlyExported
        } else if matches!(s.vis, Visibility::Public(_)) {
            TerraffiExportStatus::Public
        } else {
            TerraffiExportStatus::None
        };

        let doc = CDoc::from_attrs(&s.attrs);
        let fields = match &s.fields {
            Fields::Named(named) => named
                .named
                .iter()
                .map(|f| {
                    let field_name = f.ident.as_ref().unwrap().to_string();
                    let field_doc = CDoc::from_attrs(&f.attrs);
                    let ty = (&f.ty).try_into().unwrap(); // TODO: Handle error
                    CStructField {
                        name: field_name,
                        doc: field_doc,
                        ty,
                    }
                })
                .collect(),
            Fields::Unit => Vec::new(),
            Fields::Unnamed(_) => {
                return Err(format!(
                    "Struct '{}' has unnamed fields which are unsupported",
                    name
                ));
            }
        };
        Ok(CStruct {
            name,
            doc,
            export_status,
            source_order,
            fields,
        })
    }

    pub fn collect_referenced_type_names(&self, names: &mut HashSet<String>) {
        for f in &self.fields {
            f.collect_referenced_type_names(names);
        }
    }

    pub fn emit(&self, w: &mut impl Write, config: &TerraffiConfig) -> fmt::Result {
        let name = config.format_type_name(&self.name);
        self.doc.emit_doxygen(w, "")?;
        writeln!(w, "typedef struct {name} {{")?;
        for field in &self.fields {
            field.emit(w, config, "    ")?;
        }
        writeln!(w, "}} {name};")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CStructField {
    pub name: String,
    pub doc: CDoc,
    pub ty: CType,
}

impl CStructField {
    pub fn collect_referenced_type_names(&self, names: &mut HashSet<String>) {
        self.ty.collect_referenced_type_names(names);
    }

    pub fn emit(&self, w: &mut impl Write, config: &TerraffiConfig, indent: &str) -> fmt::Result {
        if let CType::Slice { is_const, inner } = &self.ty {
            let ptr_ty = CType::Pointer {
                is_const: *is_const,

                inner: inner.clone(),
            };
            self.doc.emit_doxygen(w, indent)?;
            writeln!(w, "{}{} {};", indent, ptr_ty.format(config), self.name)?;
            writeln!(w, "{indent}/** Number of elements in {}. */", self.name)?;
            writeln!(w, "{}size_t {}_len;", indent, self.name)
        } else if let CType::Vec { is_const, inner } = &self.ty {
            let ptr_ty = CType::Pointer {
                is_const: *is_const,

                inner: inner.clone(),
            };
            self.doc.emit_doxygen(w, indent)?;
            writeln!(w, "{}{} {};", indent, ptr_ty.format(config), self.name)?;
            writeln!(w, "{indent}/** Number of elements in {}. */", self.name)?;
            writeln!(w, "{}size_t {}_len;", indent, self.name)?;
            writeln!(w, "{indent}/** Allocated capacity of {}. */", self.name)?;
            writeln!(w, "{}size_t {}_capacity;", indent, self.name)
        } else if let CType::Array { size, inner } = &self.ty {
            self.doc.emit_doxygen(w, indent)?;
            writeln!(
                w,
                "{}{} {}[{}];",
                indent,
                inner.format(config),
                self.name,
                size
            )
        } else {
            self.doc.emit_doxygen(w, indent)?;
            writeln!(w, "{}{};", indent, self.ty.format_named(&self.name, config))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CEnum {
    pub name: String,
    pub doc: CDoc,
    pub export_status: TerraffiExportStatus,
    pub source_order: usize,
    pub variants: Vec<CEnumVariant>,
    pub is_synthesised_from_discriminated_union: bool,
}

impl CEnum {
    pub fn from_item(e: &ItemEnum, source_order: usize) -> Result<Self, String> {
        let name = e.ident.to_string();
        let export_status = if has_terraffi_ignore(&e.attrs) {
            TerraffiExportStatus::ExplicitlyIgnored
        } else if has_terraffi_export(&e.attrs) {
            TerraffiExportStatus::ExplicitlyExported
        } else if matches!(e.vis, Visibility::Public(_)) {
            TerraffiExportStatus::Public
        } else {
            TerraffiExportStatus::None
        };

        let doc = CDoc::from_attrs(&e.attrs);
        let variants = e
            .variants
            .iter()
            .map(|v| {
                let value = v
                    .discriminant
                    .as_ref()
                    .map(|(_, expr)| quote!(#expr).to_string());
                CEnumVariant {
                    name: v.ident.to_string(),
                    doc: CDoc::from_attrs(&v.attrs),
                    value,
                }
            })
            .collect();
        Ok(CEnum {
            name,
            doc,
            export_status,
            source_order,
            variants,
            is_synthesised_from_discriminated_union: false,
        })
    }

    pub fn emit(&self, w: &mut impl fmt::Write, config: &mut TerraffiConfig) -> fmt::Result {
        let type_name = config.format_type_name(&self.name);
        let bare_name = config.format_type_name_bare(&self.name);
        let variant_prefix = if config.is_prefix_enum_cases_with_typename {
            format!("{}_", bare_name)
        } else {
            String::new()
        };
        self.doc.emit_doxygen(w, "")?;
        writeln!(w, "typedef enum {type_name} {{")?;
        for (i, variant) in self.variants.iter().enumerate() {
            variant.emit(w, config, &variant_prefix, i < self.variants.len() - 1)?;
        }
        writeln!(w, "}} {type_name};")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CEnumVariant {
    pub name: String,
    pub doc: CDoc,
    pub value: Option<String>,
}

impl CEnumVariant {
    pub fn emit(
        &self,
        w: &mut impl Write,
        config: &TerraffiConfig,
        variant_prefix: &str,
        is_trailing: bool,
    ) -> fmt::Result {
        self.doc.emit_doxygen(w, "    ")?;
        let name = convert_case(
            &format!("{variant_prefix}{}", self.name),
            config.enum_member_case,
        );
        let trailing = if is_trailing { "," } else { "" };
        match &self.value {
            Some(value) => writeln!(w, "    {name} = {value}{trailing}"),
            _ => writeln!(w, "    {name}{trailing}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CDiscriminatedUnion {
    pub name: String,
    pub doc: CDoc,
    pub export_status: TerraffiExportStatus,
    pub source_order: usize,
    pub discriminant_enum_name: String,
    pub variants: Vec<CTaggedUnionVariant>,
}

impl CDiscriminatedUnion {
    pub fn from_item(
        e: &ItemEnum,
        source_order: &mut usize,
        config: &TerraffiConfig,
    ) -> Result<(CEnum, Self), String> {
        let name = e.ident.to_string();
        let export_status = if has_terraffi_ignore(&e.attrs) {
            TerraffiExportStatus::ExplicitlyIgnored
        } else if has_terraffi_export(&e.attrs) || has_discriminant_enum_derive(&e.attrs) {
            TerraffiExportStatus::ExplicitlyExported
        } else if matches!(e.vis, Visibility::Public(_)) {
            TerraffiExportStatus::Public
        } else {
            TerraffiExportStatus::None
        };

        let doc = CDoc::from_attrs(&e.attrs);

        let variants: Vec<_> = e
            .variants
            .iter()
            .map(|v| {
                let fields = match &v.fields {
                    Fields::Named(named) => named
                        .named
                        .iter()
                        .map(|f| CStructField {
                            name: f.ident.as_ref().unwrap().to_string(),
                            doc: CDoc::from_attrs(&f.attrs),
                            ty: (&f.ty).try_into().unwrap(), // TODO: Handle error
                        })
                        .collect(),
                    Fields::Unnamed(unnamed) => unnamed
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, f)| CStructField {
                            name: format!("_{i}"),
                            doc: CDoc::from_attrs(&f.attrs),
                            ty: (&f.ty).try_into().unwrap(), // TODO: Handle error
                        })
                        .collect(),
                    Fields::Unit => Vec::new(),
                };
                let value = v
                    .discriminant
                    .as_ref()
                    .map(|(_, expr)| quote::quote!(#expr).to_string());
                CTaggedUnionVariant {
                    name: v.ident.to_string(),
                    doc: CDoc::from_attrs(&v.attrs),
                    value,
                    fields,
                }
            })
            .collect();

        let discriminant_enum_name = match get_terraffi_discriminant_enum_name(&e.attrs) {
            Some(explicit) => explicit.clone(),
            None => format!("{}{}", name, config.discriminated_union_tag_typename_suffix),
        };

        let enum_variants = variants
            .iter()
            .map(|v| CEnumVariant {
                name: v.name.clone(),
                doc: v.doc.clone(),
                value: v.value.clone(),
            })
            .collect();

        let discriminant_enum = CEnum {
            name: discriminant_enum_name.clone(),
            doc: doc.clone(),
            export_status,
            source_order: *source_order,
            variants: enum_variants,
            is_synthesised_from_discriminated_union: true,
        };

        *source_order += 1;

        let union = CDiscriminatedUnion {
            name,
            doc: doc.clone(),
            export_status,
            source_order: *source_order,
            discriminant_enum_name,
            variants,
        };

        Ok((discriminant_enum, union))
    }

    pub fn collect_referenced_type_names(&self, names: &mut HashSet<String>) {
        names.insert(self.discriminant_enum_name.clone());
        for v in &self.variants {
            v.collect_referenced_type_names(names);
        }
    }

    pub fn emit(&self, w: &mut impl Write, config: &TerraffiConfig) -> fmt::Result {
        let type_name = config.format_type_name(&self.name);
        let discriminant_name = config.format_type_name(&self.discriminant_enum_name);

        self.doc.emit_doxygen(w, "")?;
        // Outer struct with anonymous union
        let variants_with_fields: Vec<_> = self
            .variants
            .iter()
            .filter(|v| !v.fields.is_empty())
            .collect();
        writeln!(w, "typedef struct {type_name} {{")?;
        let tag_field_name = convert_case(
            &config.discriminated_union_tag_typename_suffix,
            config.field_case,
        );
        writeln!(w, "    {discriminant_name} {tag_field_name};")?;
        if !variants_with_fields.is_empty() {
            w.write_str("    union {\n")?;
            for variant in &variants_with_fields {
                let field_name = convert_case(&variant.name, config.field_case);
                if variant.fields.len() == 1
                    && !matches!(
                        variant.fields[0].ty,
                        CType::Slice { .. } | CType::Vec { .. }
                    )
                {
                    let field = &variant.fields[0];
                    writeln!(w, "        {};", field.ty.format_named(&field_name, config),)?;
                } else {
                    w.write_str("        struct {\n")?;
                    if variant.fields.len() == 1 {
                        let synthetic = CStructField {
                            name: field_name.clone(),
                            doc: CDoc::default(),
                            ty: variant.fields[0].ty.clone(),
                        };
                        synthetic.emit(w, config, "            ")?;
                    } else {
                        for field in &variant.fields {
                            field.emit(w, config, "            ")?;
                        }
                    }
                    writeln!(w, "        }} {field_name};")?;
                }
            }
            writeln!(w, "    }}")?;
        }
        writeln!(w, "}} {type_name};")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CTaggedUnionVariant {
    pub name: String,
    pub doc: CDoc,
    pub value: Option<String>,
    pub fields: Vec<CStructField>,
}

impl CTaggedUnionVariant {
    pub fn collect_referenced_type_names(&self, names: &mut HashSet<String>) {
        for f in &self.fields {
            f.collect_referenced_type_names(names);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CBitflags {
    pub name: String,
    pub doc: CDoc,
    pub export_status: TerraffiExportStatus,
    pub source_order: usize,
    pub underlying_type: CType,
    pub constants: Vec<CBitflagsConstant>,
}

impl CBitflags {
    pub fn from_item(item: &ItemMacro, source_order: usize) -> Result<CBitflags, String> {
        let tokens: Vec<TokenTree> = item.mac.tokens.clone().into_iter().collect();

        // Collect attributes and visibility tokens before `struct`
        let mut prefix_tokens = proc_macro2::TokenStream::new();
        let mut i = 0;
        while i < tokens.len() {
            if let TokenTree::Punct(p) = &tokens[i]
                && p.as_char() == '#'
            {
                prefix_tokens.extend(std::iter::once(tokens[i].clone()));
                i += 1;
                if i < tokens.len()
                    && let TokenTree::Group(_) = &tokens[i]
                {
                    prefix_tokens.extend(std::iter::once(tokens[i].clone()));
                    i += 1;
                }
                continue;
            }
            if let TokenTree::Ident(ident) = &tokens[i] {
                if ident == "pub" || ident == "crate" {
                    prefix_tokens.extend(std::iter::once(tokens[i].clone()));
                    i += 1;
                    if i < tokens.len()
                        && let TokenTree::Group(g) = &tokens[i]
                        && g.delimiter() == proc_macro2::Delimiter::Parenthesis
                    {
                        prefix_tokens.extend(std::iter::once(tokens[i].clone()));
                        i += 1;
                    }
                    continue;
                }
                if ident == "struct" {
                    break;
                }
            }
            i += 1;
        }

        // Parse attributes and visibility by constructing a dummy struct
        let dummy_tokens = quote! { #prefix_tokens struct __Dummy; };
        let (export_status, doc) =
            if let Ok(syn::Item::Struct(dummy)) = syn::parse2::<syn::Item>(dummy_tokens) {
                let status = if has_terraffi_ignore(&dummy.attrs) {
                    TerraffiExportStatus::ExplicitlyIgnored
                } else if has_terraffi_export(&dummy.attrs) {
                    TerraffiExportStatus::ExplicitlyExported
                } else if matches!(dummy.vis, Visibility::Public(_)) {
                    TerraffiExportStatus::Public
                } else {
                    TerraffiExportStatus::None
                };
                let doc = CDoc::from_attrs(&dummy.attrs);
                (status, doc)
            } else {
                (TerraffiExportStatus::None, CDoc::default())
            };

        // Now expect: struct Name : Type { body }
        i += 1; // skip `struct`
        let name = match tokens.get(i) {
            Some(TokenTree::Ident(ident)) => ident.to_string(),
            t => {
                return Err(format!("Invalid bitflags type name token '{:?}'", t));
            }
        };
        i += 1;

        // Expect ':'
        if let Some(TokenTree::Punct(p)) = tokens.get(i) {
            if p.as_char() != ':' {
                return Err("Expected ':' character after bitflags type name".to_string());
            }
        } else {
            return Err("Expected ':' character after bitflags type name".to_string());
        }
        i += 1;

        // Underlying type ident
        let underlying_ident = match tokens.get(i) {
            Some(TokenTree::Ident(ident)) => ident.to_string(),
            t => {
                return Err(format!(
                    "Invalid bitflags underlying type name token '{:?}'",
                    t
                ));
            }
        };
        i += 1;

        let underlying_type = match underlying_ident.as_str() {
            "u8" => CType::UInt8,
            "u16" => CType::UInt16,
            "u32" => CType::UInt32,
            "u64" => CType::UInt64,
            "i8" => CType::Int8,
            "i16" => CType::Int16,
            "i32" => CType::Int32,
            "i64" => CType::Int64,
            t => return Err(format!("Invalid underlying type '{}'", t)),
        };

        // Expect { body }
        let body = if let Some(TokenTree::Group(g)) = tokens.get(i) {
            if g.delimiter() == Delimiter::Brace {
                g.stream()
            } else {
                return Err("Expected '{' after bitflags underlying type".to_string());
            }
        } else {
            return Err("Expected '{' after bitflags underlying type".to_string());
        };

        // Parse constants from body: #[doc = "..."] const NAME = EXPR;
        let body_tokens: Vec<TokenTree> = body.into_iter().collect();
        let mut constants = Vec::new();
        let mut j = 0;
        let mut pending_doc_tokens = proc_macro2::TokenStream::new();
        while j < body_tokens.len() {
            // Collect #[...] attribute tokens before const
            if let TokenTree::Punct(p) = &body_tokens[j]
                && p.as_char() == '#'
            {
                pending_doc_tokens.extend(std::iter::once(body_tokens[j].clone()));
                j += 1;
                if j < body_tokens.len()
                    && let TokenTree::Group(_) = &body_tokens[j]
                {
                    pending_doc_tokens.extend(std::iter::once(body_tokens[j].clone()));
                    j += 1;
                }
                continue;
            }
            if let TokenTree::Ident(ident) = &body_tokens[j]
                && ident == "const"
            {
                j += 1;
                // Name (skip `_` wildcard constants)
                let const_name = if let Some(TokenTree::Ident(name_ident)) = body_tokens.get(j) {
                    let s = name_ident.to_string();
                    if s == "_" {
                        pending_doc_tokens = proc_macro2::TokenStream::new();
                        // Skip `const _ = ...;`
                        while j < body_tokens.len() {
                            if let TokenTree::Punct(p) = &body_tokens[j]
                                && p.as_char() == ';'
                            {
                                j += 1;
                                break;
                            }
                            j += 1;
                        }
                        continue;
                    }
                    s
                } else {
                    j += 1;
                    pending_doc_tokens = proc_macro2::TokenStream::new();
                    continue;
                };
                j += 1;

                // Extract doc from collected attributes
                let const_doc = if !pending_doc_tokens.is_empty() {
                    let dummy = quote! { #pending_doc_tokens struct __Dummy; };
                    if let Ok(syn::Item::Struct(dummy_struct)) = syn::parse2::<syn::Item>(dummy) {
                        CDoc::from_attrs(&dummy_struct.attrs)
                    } else {
                        CDoc::default()
                    }
                } else {
                    CDoc::default()
                };
                pending_doc_tokens = proc_macro2::TokenStream::new();

                // Expect '='
                if let Some(TokenTree::Punct(p)) = body_tokens.get(j)
                    && p.as_char() == '='
                {
                    j += 1;
                }

                // Collect value tokens until ';'
                let mut value_tokens = Vec::new();
                while j < body_tokens.len() {
                    if let TokenTree::Punct(p) = &body_tokens[j]
                        && p.as_char() == ';'
                    {
                        j += 1;
                        break;
                    }
                    value_tokens.push(body_tokens[j].clone());
                    j += 1;
                }

                let value = rust_literal_tokens_to_c(value_tokens);
                constants.push(CBitflagsConstant {
                    name: const_name,
                    doc: const_doc,
                    value,
                });
                continue;
            }
            pending_doc_tokens = proc_macro2::TokenStream::new();
            j += 1;
        }

        Ok(CBitflags {
            name,
            doc,
            export_status,
            source_order,
            underlying_type,
            constants,
        })
    }

    pub fn emit(&self, w: &mut impl Write, config: &TerraffiConfig) -> fmt::Result {
        let type_name = config.format_type_name(&self.name);
        let bare_name = config.format_type_name_bare(&self.name);
        let const_prefix = if config.is_prefix_enum_cases_with_typename {
            format!("{}_", bare_name)
        } else {
            String::new()
        };
        self.doc.emit_doxygen(w, "")?;
        let c_type = self.underlying_type.format(config);
        writeln!(w, "typedef {c_type} {type_name};")?;
        for constant in &self.constants {
            constant.emit(w, config, &type_name, &const_prefix)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CBitflagsConstant {
    pub name: String,
    pub doc: CDoc,
    pub value: String,
}

impl CBitflagsConstant {
    pub fn emit(
        &self,
        w: &mut impl Write,
        config: &TerraffiConfig,
        typename: &str,
        const_prefix: &str,
    ) -> fmt::Result {
        let name = convert_case(
            &format!("{const_prefix}{}", self.name),
            config.constant_case,
        );
        self.doc.emit_doxygen(w, "")?;
        writeln!(w, "#define {name} (({typename}){})", self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CFunction {
    pub name: String,
    pub doc: CFunctionDoc,
    pub export_status: TerraffiExportStatus,
    pub source_order: usize,
    pub params: Vec<CFunctionParam>,
    pub return_type: CType,
}

impl CFunction {
    pub fn from_item(func: &ItemFn, source_order: usize) -> Result<Self, String> {
        let name = func.sig.ident.to_string();
        let export_status = if has_terraffi_ignore(&func.attrs) {
            TerraffiExportStatus::ExplicitlyIgnored
        } else if has_terraffi_export(&func.attrs) {
            TerraffiExportStatus::ExplicitlyExported
        } else if matches!(func.vis, Visibility::Public(_)) {
            TerraffiExportStatus::Public
        } else {
            TerraffiExportStatus::None
        };

        let doc = CFunctionDoc::from_attrs(&func.attrs);

        let params: Vec<_> = func
            .sig
            .inputs
            .iter()
            .map(|arg| CFunctionParam::from_item(arg, &doc.params))
            .collect();

        for p in &params {
            if let Err(e) = p {
                return Err(e.clone());
            }
        }

        let return_type = match &func.sig.output {
            syn::ReturnType::Default => CType::Void,
            syn::ReturnType::Type(_, ty) => ty.as_ref().try_into()?,
        };
        Ok(CFunction {
            name,
            doc,
            export_status,
            source_order,
            params: params.into_iter().map(|p| p.unwrap()).collect(),
            return_type,
        })
    }

    pub fn collect_referenced_type_names(&self, names: &mut HashSet<String>) {
        for p in &self.params {
            p.collect_referenced_type_names(names);
        }
        self.return_type.collect_referenced_type_names(names);
    }

    pub fn emit(&self, w: &mut impl Write, config: &TerraffiConfig) -> fmt::Result {
        let param_docs: Vec<(&str, &Option<String>)> = self
            .params
            .iter()
            .map(|p| (p.name.as_str(), &p.doc))
            .collect();
        self.doc.emit_doxygen(w, &param_docs)?;
        let ret = self.return_type.format(config);
        let params = if self.params.is_empty() {
            "void".to_string()
        } else {
            self.params
                .iter()
                .map(|p| p.ty.format_named(&p.name, config))
                .collect::<Vec<_>>()
                .join(", ")
        };

        if let Some(macro_name) = &config.export_macro {
            write!(w, "{} ", macro_name)?;
        }
        writeln!(w, "{ret} {name}({params});", name = self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CFunctionParam {
    pub name: String,
    pub doc: Option<String>,
    pub ty: CType,
}

impl CFunctionParam {
    pub fn from_item(arg: &FnArg, param_docs: &[CParamDoc]) -> Result<Self, String> {
        match arg {
            FnArg::Typed(pat_type) => {
                let param_name = match pat_type.pat.as_ref() {
                    syn::Pat::Ident(ident) => ident.ident.to_string(),
                    _ => "_".to_string(),
                };
                let doc = param_docs
                    .iter()
                    .find(|pd| pd.name == param_name)
                    .map(|pd| pd.description.clone());
                let ty = pat_type.ty.as_ref().try_into()?;
                Ok(CFunctionParam {
                    name: param_name,
                    doc,
                    ty,
                })
            }
            FnArg::Receiver(_) => Err("Unsupported function argument 'self'".to_string()),
        }
    }

    pub fn collect_referenced_type_names(&self, names: &mut HashSet<String>) {
        self.ty.collect_referenced_type_names(names);
    }
}
