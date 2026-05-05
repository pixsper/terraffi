pub mod doc;
mod items;

pub use items::CMacro;
use items::*;

pub use convert_case::Case;

use convert_case::{Boundary, Converter};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::fmt::Write;
use std::path::PathBuf;

/// Builder pattern type used to construct `TerraffiGenerator`
#[derive(Default, Clone, PartialEq, Eq)]
pub struct TerraffiGeneratorBuilder {
    /// If true, dependency crates are scanned for type definitions. Defaults to `true`.
    is_scan_dependencies: bool,
    /// Crate names to exclude from dependency scanning.
    exclude_crates: Vec<String>,
    exclude_types: Vec<String>,

    /// A comment placed at the top of the generated header, rendered as a block comment (`/* ... */`).
    header_comment: Option<String>,
    /// The name used for the `#ifndef`/`#define` include guard. If `None`, derived from the
    /// crate directory name (e.g. `my_crate` becomes `MY_CRATE_H`).
    header_guard: Option<String>,
    /// A macro name (e.g. `"DLL_API"`) prepended to all function declarations. When set, a
    /// platform-detection `#define` block is also emitted after the include statements to control DLL function imports.
    export_macro: Option<String>,

    /// If true, include statements for `<stdint.h>` and `<stdbool.h>` will be added to the header file. Defaults to true.
    is_add_std_includes: bool,
    /// Additional include statements to add to the header file.
    additional_includes: Vec<String>,
    /// Additional macro definitions to emit after the export macro block.
    additional_macro_definitions: Vec<CMacro>,

    /// If true, all `pub extern "C"` functions are exported. If false, only `#[terraffi_export]` functions are exported.
    is_export_public_functions: bool,
    /// If true, all `pub` types are exported. If false, only `#[terraffi_export]` types are exported.
    is_export_public_types: bool,

    /// A string prepended to struct type names in the generated header. Applied after case conversion.
    struct_prefix: Option<String>,
    /// A string prepended to enum type names in the generated header. Applied after case conversion.
    enum_prefix: Option<String>,
    /// A string prepended to all const definition names in the generated header. Applied after
    /// case conversion.
    constant_prefix: Option<String>,
    /// If true, enum member names are prefixed with the enum type name. Defaults to `true`.
    is_prefix_enum_members_with_typename: Option<bool>,

    /// Suffix appended to struct type names. Defaults to `"_t"`.
    struct_suffix: Option<String>,
    /// Suffix appended to enum type names. Defaults to `"_e"`.
    enum_suffix: Option<String>,
    /// Suffix appended to constant definition names. Defaults to `""`.
    constant_suffix: Option<String>,
    /// Suffix appended to the enum type name generated for discriminated unions. Defaults to `"Kind"`.
    discriminated_union_tag_typename_suffix: Option<String>,

    /// The case convention applied to all type definition names. Defaults to `Case::Snake`.
    typename_case: Option<Case<'static>>,
    /// The case convention applied to function parameter names. Defaults to `Case::Snake`.
    parameter_case: Option<Case<'static>>,
    /// The case convention applied to struct field names. Defaults to `Case::Snake`.
    field_case: Option<Case<'static>>,
    /// The case convention applied to enum variant names. Defaults to `Case::UpperSnake`.
    enum_member_case: Option<Case<'static>>,
    /// The case convention applied to constant values/macros. Defaults to `Case::UpperSnake`.
    constant_case: Option<Case<'static>>,

    /// Maps raw Rust type names to custom C type names, bypassing normal name formatting.
    type_name_map: HashMap<String, String>,
}

impl TerraffiGeneratorBuilder {
    pub fn new() -> Self {
        Self {
            is_add_std_includes: true,
            is_export_public_functions: true,
            is_export_public_types: false,
            is_scan_dependencies: true,
            ..Default::default()
        }
    }

    pub fn disable_scan_dependencies(mut self) -> Self {
        self.is_scan_dependencies = true;
        self
    }

    pub fn exclude_crate(mut self, name: impl Into<String>) -> Self {
        self.exclude_crates.push(name.into());
        self
    }

    pub fn header_comment(mut self, comment: impl Into<String>) -> Self {
        self.header_comment = Some(comment.into());
        self
    }

    pub fn header_guard(mut self, guard: impl Into<String>) -> Self {
        self.header_guard = Some(guard.into());
        self
    }

    pub fn export_macro(mut self, export_macro: impl Into<String>) -> Self {
        self.export_macro = Some(export_macro.into());
        self
    }

    pub fn add_std_includes(mut self, value: bool) -> Self {
        self.is_add_std_includes = value;
        self
    }

    pub fn add_include(mut self, include: impl Into<String>) -> Self {
        self.additional_includes.push(include.into());
        self
    }

    pub fn add_macro_definition(mut self, definition: impl Into<CMacro>) -> Self {
        self.additional_macro_definitions.push(definition.into());
        self
    }

    pub fn export_public_functions(mut self) -> Self {
        self.is_export_public_functions = true;
        self
    }

    pub fn export_only_annotated_functions(mut self) -> Self {
        self.is_export_public_functions = false;
        self
    }

    pub fn export_public_types(mut self) -> Self {
        self.is_export_public_types = true;
        self
    }

    pub fn export_only_annotated_types(mut self) -> Self {
        self.is_export_public_types = false;
        self
    }

    pub fn typename_prefix(mut self, prefix: impl Into<String>) -> Self {
        let p = prefix.into();
        self.struct_prefix = Some(p.clone());
        self.enum_prefix = Some(p);
        self
    }

    pub fn struct_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.struct_prefix = Some(prefix.into());
        self
    }

    pub fn enum_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.enum_prefix = Some(prefix.into());
        self
    }

    pub fn constant_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.constant_prefix = Some(prefix.into());
        self
    }

    pub fn prefix_enum_cases_with_typename(mut self, value: bool) -> Self {
        self.is_prefix_enum_members_with_typename = Some(value);
        self
    }

    pub fn typename_suffix(mut self, suffix: impl Into<String>) -> Self {
        let s = suffix.into();
        self.struct_suffix = Some(s.clone());
        self.enum_suffix = Some(s);
        self
    }

    pub fn struct_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.struct_suffix = Some(suffix.into());
        self
    }

    pub fn enum_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.enum_suffix = Some(suffix.into());
        self
    }

    pub fn constant_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.constant_suffix = Some(suffix.into());
        self
    }

    pub fn discriminated_union_tag_typename_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.discriminated_union_tag_typename_suffix = Some(suffix.into());
        self
    }

    pub fn typename_case(mut self, case: Case<'static>) -> Self {
        self.typename_case = Some(case);
        self
    }

    pub fn parameter_case(mut self, case: Case<'static>) -> Self {
        self.parameter_case = Some(case);
        self
    }

    pub fn field_case(mut self, case: Case<'static>) -> Self {
        self.field_case = Some(case);
        self
    }

    pub fn enum_member_case(mut self, case: Case<'static>) -> Self {
        self.enum_member_case = Some(case);
        self
    }

    pub fn constant_case(mut self, case: Case<'static>) -> Self {
        self.constant_case = Some(case);
        self
    }

    pub fn rename_type(mut self, rust_name: impl Into<String>, c_name: impl Into<String>) -> Self {
        self.type_name_map.insert(rust_name.into(), c_name.into());
        self
    }

    pub fn build(self, crate_dir: impl Into<PathBuf>) -> TerraffiGenerator {
        let config = TerraffiConfig {
            is_scan_dependencies: self.is_scan_dependencies,
            exclude_crates: self.exclude_crates,

            header_comment: self.header_comment,
            header_guard: self.header_guard,
            export_macro: self.export_macro,

            is_add_std_includes: self.is_add_std_includes,
            additional_includes: self.additional_includes,
            additional_macro_definitions: self.additional_macro_definitions,

            is_export_public_functions: self.is_export_public_functions,
            is_export_public_types: self.is_export_public_types,

            struct_prefix: self.struct_prefix.unwrap_or_default(),
            enum_prefix: self.enum_prefix.unwrap_or_default(),
            constant_prefix: self.constant_prefix.unwrap_or_default(),
            is_prefix_enum_cases_with_typename: self
                .is_prefix_enum_members_with_typename
                .unwrap_or(true),

            struct_suffix: self.struct_suffix.unwrap_or_else(|| "_t".to_string()),
            enum_suffix: self.enum_suffix.unwrap_or_else(|| "_e".to_string()),
            constant_suffix: self.constant_suffix.unwrap_or_default(),
            discriminated_union_tag_typename_suffix: self
                .discriminated_union_tag_typename_suffix
                .unwrap_or_else(|| "Kind".to_string()),

            type_case: self.typename_case.unwrap_or(Case::Snake),
            parameter_case: self.parameter_case.unwrap_or(Case::Snake),
            field_case: self.field_case.unwrap_or(Case::Snake),
            enum_member_case: self.enum_member_case.unwrap_or(Case::UpperSnake),
            constant_case: self.constant_case.unwrap_or(Case::UpperSnake),

            type_name_bare_map: self.type_name_map.clone(),
            type_name_map: self.type_name_map,
        };

        TerraffiGenerator::new(config, crate_dir)
    }
}

/// Configuration for terraffi C header generation.
#[derive(Clone, PartialEq, Eq)]
pub struct TerraffiConfig {
    /// If true, dependency crates are scanned for type definitions. Defaults to `true`.
    is_scan_dependencies: bool,
    /// Crate names to exclude from dependency scanning.
    exclude_crates: Vec<String>,

    /// A comment placed at the top of the generated header, rendered as a block comment (`/* ... */`).
    header_comment: Option<String>,
    /// The name used for the `#ifndef`/`#define` include guard. If `None`, derived from the
    /// crate directory name (e.g. `my_crate` becomes `MY_CRATE_H`).
    header_guard: Option<String>,
    /// A macro name (e.g. `"DLL_API"`) prepended to all function declarations. When set, a
    /// platform-detection `#define` block is also emitted after the include statements to control DLL function imports.
    export_macro: Option<String>,

    /// If true, include statements for `<stdint.h>` and `<stdbool.h>` will be added to the header file. Defaults to true.
    is_add_std_includes: bool,
    /// Additional include statements to add to the header file.
    additional_includes: Vec<String>,
    /// Additional macro definitions to emit after the export macro block.
    additional_macro_definitions: Vec<CMacro>,

    /// If true, all `pub extern "C"` functions are exported. If false, only functions annotated with `#[terraffi_export]` are exported.
    is_export_public_functions: bool,
    /// If true, all `pub` types are exported. If false, only types annotated with `#[terraffi_export]` are exported.
    is_export_public_types: bool,

    /// A string prepended to struct type names in the generated header. Applied after case conversion. Defaults to `""`.
    struct_prefix: String,
    /// A string prepended to enum type names in the generated header. Applied after case conversion. Defaults to `""`.
    enum_prefix: String,
    /// A string prepended to all const definition names in the generated header. Applied after
    /// case conversion. Defaults to `""`.
    constant_prefix: String,
    /// If true, enum variant names are prefixed with the enum type name. Defaults to `true`.
    is_prefix_enum_cases_with_typename: bool,

    /// Suffix appended to struct type names. Applied after case conversion. Defaults to `"_t"`.
    struct_suffix: String,
    /// Suffix appended to enum type names. Applied after case conversion. Defaults to `"_e"`.
    enum_suffix: String,
    /// Suffix appended to constant definition names. Applied after case conversion. Defaults to `""`.
    constant_suffix: String,
    /// Suffix appended to the enum kind type name generated from discriminated enums. Defaults to `"Kind"`.
    pub discriminated_union_tag_typename_suffix: String,

    /// The case convention applied to all type definition names. Defaults to `Case::Snake`.
    type_case: Case<'static>,
    /// The case convention applied to function parameter names. Defaults to `Case::Snake`.
    parameter_case: Case<'static>,
    /// The case convention applied to struct field names. Defaults to `Case::Snake`.
    field_case: Case<'static>,
    /// The case convention applied to enum variant names. Defaults to `Case::UpperSnake`.
    enum_member_case: Case<'static>,
    /// The case convention applied to constant values/macros. Defaults to `Case::UpperSnake`.
    constant_case: Case<'static>,

    /// Maps raw type names to their fully-formatted C names. Seeded with user renames, then
    /// populated during generation with computed names for all discovered types.
    type_name_map: HashMap<String, String>,
    /// Maps raw type names to their formatted C names without suffix, populated during generation.
    type_name_bare_map: HashMap<String, String>,
}

impl TerraffiConfig {
    fn format_struct_name(&self, raw_name: &str) -> String {
        convert_case(
            &format!("{}{raw_name}{}", self.struct_prefix, self.struct_suffix),
            self.type_case,
        )
    }

    fn format_struct_name_bare(&self, raw_name: &str) -> String {
        convert_case(&format!("{}{raw_name}", self.struct_prefix), self.type_case)
    }

    fn format_enum_name(&self, raw_name: &str) -> String {
        convert_case(
            &format!("{}{raw_name}{}", self.enum_prefix, self.enum_suffix),
            self.type_case,
        )
    }

    fn format_enum_name_bare(&self, raw_name: &str) -> String {
        convert_case(&format!("{}{raw_name}", self.enum_prefix), self.type_case)
    }

    /// Looks up the fully-formatted C name for a raw type name.
    pub fn format_type_name(&self, raw_name: &str) -> String {
        if let Some(name) = self.type_name_map.get(raw_name) {
            return name.clone();
        }
        convert_case(raw_name, self.type_case)
    }

    /// Like `format_type_name` but without the struct/enum suffix.
    /// Used for deriving enum member prefixes where the suffix should not appear.
    pub fn format_type_name_bare(&self, raw_name: &str) -> String {
        if let Some(name) = self.type_name_bare_map.get(raw_name) {
            return name.clone();
        }
        convert_case(raw_name, self.type_case)
    }
}

pub struct TerraffiGenerator {
    pub config: TerraffiConfig,
    pub crate_dir: PathBuf,
    header_guard: String,
    referenced_types: HashSet<String>,
}

struct DependencySource {
    #[allow(dead_code)]
    pub name: String,
    pub src_dir: PathBuf,
}

impl TerraffiGenerator {
    pub fn new(config: TerraffiConfig, crate_dir: impl Into<PathBuf>) -> Self {
        let crate_dir = crate_dir.into();
        let header_guard = config.header_guard.clone().unwrap_or_else(|| {
            let name = crate_dir
                .file_stem()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "".to_string())
                .replace(".", "_")
                .to_uppercase();

            format!("{}_H", name).to_string()
        });

        Self {
            config,
            crate_dir,
            header_guard,
            referenced_types: HashSet::new(),
        }
    }

    fn discover_dependency_sources(&self) -> Result<Vec<DependencySource>, Box<dyn Error>> {
        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(self.crate_dir.join("Cargo.toml"))
            .exec()?;

        // Find the root package
        let root_package = metadata.root_package().ok_or(
            "Could not determine root package from cargo metadata. \
                Is the manifest path pointing to a workspace virtual manifest? \
                It must point to a specific package's Cargo.toml.",
        )?;

        // Collect direct dependency names in Cargo.toml declaration order
        let direct_dep_names: Vec<&str> = root_package
            .dependencies
            .iter()
            .filter_map(|d| match d.name.as_str() {
                "terraffi_ctypes" | "terraffi_gen" | "terraffi_helpers" | "terraffi_macro" => None,
                _ => Some(d.name.as_str()),
            })
            .collect();

        // Build a lookup from package name to its metadata
        let package_map: std::collections::HashMap<&str, &cargo_metadata::Package> = metadata
            .packages
            .iter()
            .filter(|p| p.id != root_package.id)
            .map(|p| (p.name.as_str(), p))
            .collect();

        let mut sources = Vec::new();

        // Iterate in Cargo.toml declaration order
        for dep_name in &direct_dep_names {
            let package = match package_map.get(dep_name) {
                Some(p) => p,
                None => continue,
            };

            // Skip excluded crates
            if self.config.exclude_crates.contains(&package.name) {
                continue;
            }

            // Skip registry/git crates (only include local/path crates)
            if package.source.is_some() {
                continue;
            }

            // Derive the src directory from the manifest path
            let manifest_dir = package.manifest_path.parent().ok_or_else(|| {
                format!(
                    "Could not get parent dir of manifest: {}",
                    package.manifest_path
                )
            })?;
            let src_dir = manifest_dir.join("src");
            let src_dir_std: PathBuf = src_dir.into();

            if src_dir_std.is_dir() {
                sources.push(DependencySource {
                    name: package.name.as_str().to_string(),
                    src_dir: src_dir_std,
                });
            }
        }

        Ok(sources)
    }

    /// Returns true if the item should be directly exported based on its export status
    /// and the current configuration.
    fn is_item_exported(&self, item: &CItem) -> bool {
        match item.export_status() {
            TerraffiExportStatus::ExplicitlyIgnored => false,
            TerraffiExportStatus::ExplicitlyExported => true,
            TerraffiExportStatus::Public => match item {
                CItem::Function(_) => self.config.is_export_public_functions,
                _ => self.config.is_export_public_types,
            },
            TerraffiExportStatus::None => false,
        }
    }

    pub fn generate(&mut self) -> Result<String, Box<dyn Error>> {
        if !self.crate_dir.is_dir() {
            return Err("Crate dir is not a directory".into());
        }

        let mut source_order: usize = 0;
        let mut items = Vec::new();

        if self.config.is_scan_dependencies {
            let dep_sources = self.discover_dependency_sources()?;
            for dep in &dep_sources {
                let mut dep_items =
                    scan_source_dir(&dep.src_dir, &mut source_order, false, &self.config)?;
                items.append(&mut dep_items);
            }
        }

        // Scan the root crate
        let src_dir = self.crate_dir.join("src");
        let mut root_items = scan_source_dir(&src_dir, &mut source_order, true, &self.config)?;
        items.append(&mut root_items);

        // Remove duplicated enums
        let real_enums: HashSet<_> = items
            .iter()
            .filter_map(|item| match item {
                CItem::Enum(e) if !e.is_synthesised_from_discriminated_union => {
                    Some(e.name.clone())
                }
                _ => None,
            })
            .collect();

        items.retain(|item| match item {
            CItem::Enum(e) if e.is_synthesised_from_discriminated_union => {
                !real_enums.contains(&e.name)
            }
            _ => true,
        });

        // Seed referenced types from directly exported items
        for item in &items {
            if self.is_item_exported(item) {
                item.collect_referenced_type_names(&mut self.referenced_types);
            }
        }

        // Transitive closure: types referenced by referenced types
        loop {
            let prev_len = self.referenced_types.len();
            for item in &items {
                if item.export_status() == TerraffiExportStatus::ExplicitlyIgnored {
                    continue;
                }
                match item {
                    CItem::Struct(s) if self.referenced_types.contains(&s.name) => {
                        s.collect_referenced_type_names(&mut self.referenced_types);
                    }
                    CItem::DiscriminatedUnion(du) if self.referenced_types.contains(&du.name) => {
                        du.collect_referenced_type_names(&mut self.referenced_types);
                    }
                    _ => {}
                }
            }
            if self.referenced_types.len() == prev_len {
                break;
            }
        }

        // Filter to exported functions and referenced/exported types
        let mut relevant_items: Vec<_> = items
            .into_iter()
            .filter(|i| {
                if i.export_status() == TerraffiExportStatus::ExplicitlyIgnored {
                    return false;
                }
                match i {
                    CItem::Function(_) => self.is_item_exported(i),
                    _ => self.is_item_exported(i) || self.referenced_types.contains(i.name()),
                }
            })
            .collect();

        // Deduplicate types by name. When the same type name appears multiple times
        // (e.g. from both a dependency scan and as a referenced type in the root crate),
        // keep only the first full definition, or the first opaque typedef if no full
        // definition exists. Functions are never deduplicated.
        {
            let mut seen: HashMap<String, usize> = HashMap::new();
            let mut to_remove = Vec::new();
            for (idx, item) in relevant_items.iter().enumerate() {
                if matches!(item, CItem::Function(_)) {
                    continue;
                }
                let name = item.name().to_string();
                let is_opaque = matches!(item, CItem::Typedef(t) if t.inner_type.is_none());
                if let Some(&prev_idx) = seen.get(&name) {
                    let prev_is_opaque = matches!(
                        &relevant_items[prev_idx],
                        CItem::Typedef(t) if t.inner_type.is_none()
                    );
                    if prev_is_opaque && !is_opaque {
                        // Replace opaque with full definition
                        to_remove.push(prev_idx);
                        seen.insert(name, idx);
                    } else {
                        // Keep the earlier entry
                        to_remove.push(idx);
                    }
                } else {
                    seen.insert(name, idx);
                }
            }
            // Remove in reverse order to preserve indices
            to_remove.sort_unstable();
            for idx in to_remove.into_iter().rev() {
                relevant_items.remove(idx);
            }
        }

        // Populate type name maps for name resolution (preserving user renames)
        for item in &relevant_items {
            match item {
                CItem::Struct(s) => {
                    let formatted = self.config.format_struct_name(&s.name);
                    let bare = self.config.format_struct_name_bare(&s.name);
                    self.config
                        .type_name_map
                        .entry(s.name.clone())
                        .or_insert(formatted);
                    self.config
                        .type_name_bare_map
                        .entry(s.name.clone())
                        .or_insert(bare);
                }
                CItem::Enum(e) => {
                    let formatted = self.config.format_enum_name(&e.name);
                    let bare = self.config.format_enum_name_bare(&e.name);
                    self.config
                        .type_name_map
                        .entry(e.name.clone())
                        .or_insert(formatted);
                    self.config
                        .type_name_bare_map
                        .entry(e.name.clone())
                        .or_insert(bare);
                }
                CItem::DiscriminatedUnion(du) => {
                    let formatted = self.config.format_struct_name(&du.name);
                    let bare = self.config.format_struct_name_bare(&du.name);
                    self.config
                        .type_name_map
                        .entry(du.name.clone())
                        .or_insert(formatted);
                    self.config
                        .type_name_bare_map
                        .entry(du.name.clone())
                        .or_insert(bare);
                }
                CItem::Bitflags(b) => {
                    let formatted = self.config.format_struct_name(&b.name);
                    let bare = self.config.format_struct_name_bare(&b.name);
                    self.config
                        .type_name_map
                        .entry(b.name.clone())
                        .or_insert(formatted);
                    self.config
                        .type_name_bare_map
                        .entry(b.name.clone())
                        .or_insert(bare);
                }
                CItem::Typedef(t) => {
                    let formatted = self.config.format_struct_name(&t.name);
                    let bare = self.config.format_struct_name_bare(&t.name);
                    self.config
                        .type_name_map
                        .entry(t.name.clone())
                        .or_insert(formatted);
                    self.config
                        .type_name_bare_map
                        .entry(t.name.clone())
                        .or_insert(bare);
                }
                _ => {}
            }
        }

        // Add builtin ctypes definitions for referenced types not defined in scanned items
        let defined_names: HashSet<&str> = relevant_items
            .iter()
            .filter(|i| !matches!(i, CItem::Function(_)))
            .map(|i| i.name())
            .collect();
        let mut builtin_names: Vec<&String> = self
            .referenced_types
            .iter()
            .filter(|name| !defined_names.contains(name.as_str()))
            .collect();
        builtin_names.sort();
        for name in builtin_names {
            let formatted = self.config.format_struct_name(name);
            let bare = self.config.format_struct_name_bare(name);
            self.config
                .type_name_map
                .entry(name.clone())
                .or_insert(formatted);
            self.config
                .type_name_bare_map
                .entry(name.clone())
                .or_insert(bare);

            if let Some(builtin) = builtin_ctypes_item(name) {
                relevant_items.push(builtin);
            }
        }

        sort_items_in_reference_order(&mut relevant_items);

        let mut out = String::new();
        match self.emit_header(&mut out, &relevant_items) {
            Ok(_) => Ok(out),
            Err(e) => Err(e.into()),
        }
    }

    fn emit_header(&mut self, w: &mut impl Write, items: &[CItem]) -> fmt::Result {
        if let Some(comment) = &self.config.header_comment {
            w.write_str("/*\n")?;
            for line in comment.lines() {
                writeln!(w, " * {line}")?;
            }
            w.write_str(" */\n\n")?;
        }

        writeln!(w, "#ifndef {0}\n#define {0}\n", self.header_guard)?;

        if self.config.is_add_std_includes {
            w.write_str("#include <stdint.h>\n")?;
            w.write_str("#include <stddef.h>\n")?;
            w.write_str("#include <stdbool.h>\n")?;
        }
        for include in &self.config.additional_includes {
            let mut include = include.clone();
            if !include.starts_with("#include ") {
                if !(include.starts_with("<") || include.starts_with("\"")) {
                    include.insert(0, '"');
                }
                if !(include.ends_with(">") || include.ends_with("\"")) {
                    include.insert(include.len() - 1, '"');
                }
                include.insert_str(0, "#include ");
            }

            w.write_str(&include)?;
        }

        if let Some(macro_name) = &self.config.export_macro {
            write!(
                w,
                "\n#if defined _WIN32 || defined __CYGWIN__\n    \
             #define {0} __declspec(dllimport)\n\
             #elif __GNUC__ >= 4\n    \
             #define {0} __attribute__ ((visibility (\"default\")))\n\
             #else\n    \
             #define {0}\n\
             #endif\n",
                macro_name
            )?;
        }

        for def in &self.config.additional_macro_definitions {
            w.write_char('\n')?;
            def.emit(w, &self.config)?;
        }

        let mut functions = Vec::new();

        for item in items {
            if let CItem::Function(f) = item {
                functions.push(f);
                continue;
            }
            w.write_char('\n')?;

            item.emit(w, &mut self.config)?;
        }

        if !functions.is_empty() {
            w.write_str("\n#ifdef __cplusplus\nextern \"C\" {\n#endif // __cplusplus\n\n")?;

            for func in functions {
                func.emit(w, &self.config)?;
                w.write_str("\n")?;
            }

            w.write_str("\n#ifdef __cplusplus\n}  // extern \"C\"\n#endif  // __cplusplus\n")?;
        }

        w.write_str("\n#endif\n")
    }
}

/// Scans a crate's `src/` directory for `.rs` files and extracts `CItem`s,
/// following `mod` declaration order starting from `lib.rs` (or `main.rs`).
///
/// If `include_functions` is false, `CItem::Function` variants are filtered out.
/// This is used when scanning dependency crates (we only want their type definitions,
/// not their exported functions).
fn scan_source_dir(
    src_dir: &std::path::Path,
    source_order: &mut usize,
    include_functions: bool,
    config: &TerraffiConfig,
) -> Result<Vec<CItem>, Box<dyn Error>> {
    if !src_dir.is_dir() {
        return Ok(Vec::new());
    }

    let entry_point = if src_dir.join("lib.rs").exists() {
        src_dir.join("lib.rs")
    } else if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        return Ok(Vec::new());
    };

    let mut items = Vec::new();
    scan_file(
        &entry_point,
        src_dir,
        source_order,
        include_functions,
        config,
        &mut items,
    )?;
    Ok(items)
}

/// Parses a single `.rs` file, extracts `CItem`s from it, then recursively
/// follows any `mod` declarations in the order they appear in the source.
fn scan_file(
    path: &std::path::Path,
    src_dir: &std::path::Path,
    source_order: &mut usize,
    include_functions: bool,
    config: &TerraffiConfig,
    items: &mut Vec<CItem>,
) -> Result<(), Box<dyn Error>> {
    let source = std::fs::read_to_string(path)?;
    let file = syn::parse_file(&source)?;

    for item in &file.items {
        // If this is a `mod foo;` declaration (no body), recurse into that module's file
        if let syn::Item::Mod(m) = item
            && m.content.is_none()
        {
            let mod_name = m.ident.to_string();
            if let Some(mod_path) = resolve_mod_path(path, src_dir, &mod_name) {
                scan_file(
                    &mod_path,
                    src_dir,
                    source_order,
                    include_functions,
                    config,
                    items,
                )?;
            }
            continue;
        }

        match CItem::from_item(item, source_order, config) {
            Ok(ci) => {
                for i in ci {
                    if include_functions || !matches!(i, CItem::Function(_)) {
                        items.push(i);
                    }
                }
            }
            Err(e) => return Err(e.into()),
        }
        *source_order += 1;
    }

    Ok(())
}

/// Resolves the file path for a `mod foo;` declaration.
///
/// Checks for `foo.rs` next to the parent file first, then `foo/mod.rs`.
/// For the crate entry point (`lib.rs`/`main.rs`), modules are looked up
/// directly in `src/`. For other files, modules are looked up in a sibling
/// directory named after the parent module.
fn resolve_mod_path(
    parent_file: &std::path::Path,
    _src_dir: &std::path::Path,
    mod_name: &str,
) -> Option<PathBuf> {
    let parent_dir = if is_crate_entry_or_mod_rs(parent_file) {
        // lib.rs, main.rs, or mod.rs: modules live in the same directory
        parent_file.parent()?
    } else {
        // foo.rs: modules live in foo/
        let stem = parent_file.file_stem()?.to_str()?;
        &parent_file.parent()?.join(stem)
    };

    // Try foo.rs first, then foo/mod.rs
    let direct = parent_dir.join(format!("{mod_name}.rs"));
    if direct.is_file() {
        return Some(direct);
    }

    let nested = parent_dir.join(mod_name).join("mod.rs");
    if nested.is_file() {
        return Some(nested);
    }

    None
}

/// Returns true if the file is a crate entry point or a `mod.rs`.
fn is_crate_entry_or_mod_rs(path: &std::path::Path) -> bool {
    matches!(
        path.file_name().and_then(|f| f.to_str()),
        Some("lib.rs") | Some("main.rs") | Some("mod.rs")
    )
}

/// Returns a synthetic `CItem` for well-known `terraffi_ctypes` types that
/// should be emitted as full struct definitions rather than opaque typedefs.
fn builtin_ctypes_item(name: &str) -> Option<CItem> {
    match name {
        "CStringBuffer" => Some(CItem::Struct(CStruct {
            name: "CStringBuffer".to_string(),
            doc: doc::CDoc {
                brief: Some("An owned null-terminated UTF-8 string buffer.".to_string()),
                description: Some("A null pointer represents an absent value (equivalent to None). The buffer must end with a null byte for C compatibility.".to_string()),
            },
            export_status: TerraffiExportStatus::Public,
            source_order: 0,
            fields: vec![
                CStructField {
                    name: "ptr".to_string(),
                    doc: doc::CDoc {
                        brief: Some("Pointer to a null-terminated UTF-8 string, or NULL if absent.".to_string()),
                        description: None,
                    },
                    ty: CType::Pointer {
                        is_const: true,

                        inner: Box::new(CType::Char),
                    },
                },
                CStructField {
                    name: "len".to_string(),
                    doc: doc::CDoc {
                        brief: Some("Length of the string in bytes, including the null terminator.".to_string()),
                        description: None,
                    },
                    ty: CType::USize,
                },
            ],
        })),
        _ => None,
    }
}

fn sort_items_in_reference_order(items: &mut Vec<CItem>) {
    // Separate functions from type items
    let mut functions = Vec::new();
    let mut type_items = Vec::new();
    for item in items.drain(..) {
        if matches!(item, CItem::Function(_)) {
            functions.push(item);
        } else {
            type_items.push(item);
        }
    }

    if type_items.is_empty() {
        *items = functions;
        return;
    }

    // Build name → index map and adjacency (dependencies)
    let name_to_idx: HashMap<String, usize> = type_items
        .iter()
        .enumerate()
        .map(|(i, item)| (item.name().to_string(), i))
        .collect();

    let n = type_items.len();
    // deps[i] = set of indices that item i depends on (must come before i)
    let mut deps: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    // in_degree[i] = number of items that must come before i
    let mut in_degree: Vec<usize> = vec![0; n];

    for (i, item) in type_items.iter().enumerate() {
        let mut referenced = HashSet::new();
        item.collect_referenced_type_names(&mut referenced);
        for ref_name in &referenced {
            if let Some(&j) = name_to_idx.get(ref_name)
                && j != i
                && deps[i].insert(j)
            {
                in_degree[i] += 1;
            }
        }
    }

    let mut sorted = Vec::with_capacity(n);
    let mut ready: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    ready.sort_by_key(|&i| type_items[i].source_order());

    while let Some(idx) = ready.first().copied() {
        ready.remove(0);
        sorted.push(idx);

        // For each item that depends on `idx`, decrease its in-degree
        for i in 0..n {
            if deps[i].contains(&idx) {
                in_degree[i] -= 1;
                if in_degree[i] == 0 {
                    // Insert in sorted position by source_order
                    let order = type_items[i].source_order();
                    let pos = ready.partition_point(|&r| type_items[r].source_order() < order);
                    ready.insert(pos, i);
                }
            }
        }
    }

    // If there are cycles, append remaining items in source_order
    if sorted.len() < n {
        let in_sorted: HashSet<usize> = sorted.iter().copied().collect();
        let mut remaining: Vec<usize> = (0..n).filter(|i| !in_sorted.contains(i)).collect();
        remaining.sort_by_key(|&i| type_items[i].source_order());
        sorted.extend(remaining);
    }

    // Rebuild items in sorted order, then append functions
    let mut sorted_type_items: Vec<CItem> = Vec::with_capacity(n);
    // Use indices to move items out (replace with placeholder to avoid clone)
    let mut type_items_opt: Vec<Option<CItem>> = type_items.into_iter().map(Some).collect();
    for idx in sorted {
        if let Some(item) = type_items_opt[idx].take() {
            sorted_type_items.push(item);
        }
    }

    *items = sorted_type_items;
    items.append(&mut functions);
}

/// Converts a string to the given case, using custom word boundaries that
/// prevent splitting between 'v'/'V' and a digit (e.g. "V6" stays as one word).
pub(crate) fn convert_case(s: &str, case: Case) -> String {
    Converter::new()
        .remove_boundaries(&[Boundary::LowerDigit, Boundary::UpperDigit])
        .add_boundary(Boundary::Custom {
            condition: |gs| {
                gs.first()
                    .is_some_and(|g| g.chars().all(|c| c.is_lowercase()) && *g != "v")
                    && gs
                        .get(1)
                        .is_some_and(|g| g.chars().all(|c| c.is_ascii_digit()))
            },
            start: 1,
            len: 0,
        })
        .add_boundary(Boundary::Custom {
            condition: |gs| {
                gs.first()
                    .is_some_and(|g| g.chars().all(|c| c.is_uppercase()) && *g != "V")
                    && gs
                        .get(1)
                        .is_some_and(|g| g.chars().all(|c| c.is_ascii_digit()))
            },
            start: 1,
            len: 0,
        })
        .to_case(case)
        .convert(s)
}
