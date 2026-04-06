use std::fmt;
use std::fmt::Write;

const MAX_LINE_WIDTH: usize = 120;

/// Writes a long text as wrapped doxygen comment lines.
/// `prefix` is the full prefix for the first line (e.g. `" * "` or `" * @param name "`).
/// `continuation` is the prefix for subsequent wrapped lines (e.g. `" *   "`).
fn write_wrapped(w: &mut impl Write, prefix: &str, continuation: &str, text: &str) -> fmt::Result {
    let max_text = MAX_LINE_WIDTH - prefix.len();
    if max_text == 0 || text.len() <= max_text {
        return writeln!(w, "{prefix}{text}");
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return writeln!(w, "{prefix}");
    }

    let mut line = String::new();
    let mut first = true;
    let max_cont = MAX_LINE_WIDTH - continuation.len();

    for word in &words {
        let limit = if first { max_text } else { max_cont };
        if line.is_empty() {
            line.push_str(word);
        } else if line.len() + 1 + word.len() > limit {
            if first {
                writeln!(w, "{prefix}{line}")?;
                first = false;
            } else {
                writeln!(w, "{continuation}{line}")?;
            }
            line.clear();
            line.push_str(word);
        } else {
            line.push(' ');
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        if first {
            writeln!(w, "{prefix}{line}")?;
        } else {
            writeln!(w, "{continuation}{line}")?;
        }
    }
    Ok(())
}

/// Documentation for an item which can only have a brief and description
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CDoc {
    /// The first sentence of the doc comment.
    pub brief: Option<String>,
    /// The remaining description text after the brief.
    pub description: Option<String>,
}

impl CDoc {
    /// Parses a doc from attributes, splitting into brief and description.
    /// The first sentence (ending with `.` or separated by a blank line) becomes
    /// the brief; the remaining text becomes the description.
    pub fn from_attrs(attrs: &[syn::Attribute]) -> Self {
        let raw = match extract_doc_string(attrs) {
            Some(s) => s,
            None => return Self::default(),
        };

        let mut brief = None;
        let mut description_lines: Vec<&str> = Vec::new();
        let mut found_brief = false;

        for line in raw.lines() {
            let trimmed = line.trim();

            if !found_brief {
                if trimmed.is_empty() {
                    if brief.is_some() {
                        found_brief = true;
                    }
                    continue;
                }
                if let Some(ref mut b) = brief {
                    *b = format!("{} {}", b, trimmed);
                } else {
                    brief = Some(trimmed.to_string());
                }
                if trimmed.ends_with('.') {
                    found_brief = true;
                }
            } else {
                if trimmed.is_empty() && description_lines.is_empty() {
                    continue;
                }
                description_lines.push(line);
            }
        }

        while description_lines
            .last()
            .is_some_and(|l| l.trim().is_empty())
        {
            description_lines.pop();
        }

        let description = if description_lines.is_empty() {
            None
        } else {
            Some(collapse_single_newlines(
                description_lines.join("\n").trim(),
            ))
        };

        CDoc { brief, description }
    }

    pub fn from_text(text: String) -> Self {
        if text.is_empty() {
            return Self::default();
        }
        CDoc {
            brief: Some(text),
            description: None,
        }
    }

    pub fn emit_doxygen(&self, w: &mut impl Write, indent: &str) -> fmt::Result {
        if self.brief.is_none() && self.description.is_none() {
            return Ok(());
        }
        // Single-line form: /** brief */
        if self.description.is_none()
            && let Some(brief) = &self.brief
            && !brief.contains('\n')
        {
            // "{indent}/** {brief} */\n"
            let single_line_len = indent.len() + 4 + brief.len() + 4;
            if single_line_len <= MAX_LINE_WIDTH {
                return writeln!(w, "{indent}/** {brief} */");
            }
        }
        // Multi-line form
        let first_prefix = format!("{indent}/** ");
        let prefix = format!("{indent} * ");
        let cont = format!("{indent} *   ");
        let mut first_line_written = false;
        if let Some(brief) = &self.brief {
            write_wrapped(w, &first_prefix, &cont, brief)?;
            first_line_written = true;
        }
        if let Some(desc) = &self.description {
            if first_line_written {
                writeln!(w, "{indent} *")?;
            }
            for (i, line) in desc.lines().enumerate() {
                if line.trim().is_empty() {
                    writeln!(w, "{indent} *")?;
                } else {
                    let p = if !first_line_written && i == 0 {
                        &first_prefix
                    } else {
                        &prefix
                    };
                    write_wrapped(w, p, &cont, line)?;
                    first_line_written = true;
                }
            }
        }
        writeln!(w, "{indent} */")
    }
}

/// A single documented parameter entry parsed from a `# Parameters` section.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CParamDoc {
    /// The parameter name.
    pub name: String,
    /// The description of the parameter.
    pub description: String,
}

/// Documentation for a function, parsed from Rust doc comments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CFunctionDoc {
    /// The first sentence of the doc comment.
    pub brief: Option<String>,
    /// The remaining description text after the brief, before any `#` sections.
    pub description: Option<String>,
    /// Parameter docs parsed from a `# Parameters` section.
    pub params: Vec<CParamDoc>,
    /// Return value doc parsed from a `# Returns` section.
    pub returns: Option<String>,
}

impl CFunctionDoc {
    /// Parses a function doc from attributes, splitting into brief, description,
    /// parameters, and return value sections.
    pub fn from_attrs(attrs: &[syn::Attribute]) -> Self {
        let raw = match extract_doc_string(attrs) {
            Some(s) => s,
            None => return Self::default(),
        };

        let mut brief = None;
        let mut description_lines: Vec<&str> = Vec::new();
        let mut params: Vec<CParamDoc> = Vec::new();
        let mut returns_lines: Vec<&str> = Vec::new();

        #[derive(PartialEq)]
        enum Section {
            Body,
            Params,
            Returns,
            Other,
        }

        let mut current_section = Section::Body;
        let mut found_brief = false;

        for line in raw.lines() {
            let trimmed = line.trim();

            // Detect section headers
            if let Some(header) = trimmed.strip_prefix("# ") {
                let header_lower = header.trim().to_lowercase();
                match header_lower.as_str() {
                    "parameters" | "params" | "arguments" | "args" => {
                        current_section = Section::Params;
                        continue;
                    }
                    "returns" | "return" | "return value" => {
                        current_section = Section::Returns;
                        continue;
                    }
                    _ => {
                        current_section = Section::Other;
                        continue;
                    }
                }
            }

            match current_section {
                Section::Body => {
                    if !found_brief {
                        if trimmed.is_empty() {
                            if brief.is_some() {
                                found_brief = true;
                            }
                            continue;
                        }
                        if let Some(ref mut b) = brief {
                            *b = format!("{} {}", b, trimmed);
                        } else {
                            brief = Some(trimmed.to_string());
                        }
                        if trimmed.ends_with('.') {
                            found_brief = true;
                        }
                    } else {
                        if trimmed.is_empty() && description_lines.is_empty() {
                            continue;
                        }
                        description_lines.push(line);
                    }
                }
                Section::Params => {
                    if let Some(rest) = trimmed.strip_prefix("- `")
                        && let Some(colon_pos) = rest.find("`:")
                    {
                        let name = rest[..colon_pos].to_string();
                        let desc = rest[colon_pos + 2..].trim().to_string();
                        params.push(CParamDoc {
                            name,
                            description: desc,
                        });
                    }
                }
                Section::Returns => {
                    if !trimmed.is_empty() || !returns_lines.is_empty() {
                        returns_lines.push(trimmed);
                    }
                }
                Section::Other => {}
            }
        }

        while description_lines
            .last()
            .is_some_and(|l| l.trim().is_empty())
        {
            description_lines.pop();
        }

        let description = if description_lines.is_empty() {
            None
        } else {
            Some(collapse_single_newlines(
                description_lines.join("\n").trim(),
            ))
        };

        while returns_lines.last().is_some_and(|l| l.is_empty()) {
            returns_lines.pop();
        }

        let returns = if returns_lines.is_empty() {
            None
        } else {
            Some(collapse_single_newlines(returns_lines.join("\n").trim()))
        };

        CFunctionDoc {
            brief,
            description,
            params,
            returns,
        }
    }

    pub fn emit_doxygen(
        &self,
        w: &mut impl Write,
        params: &[(&str, &Option<String>)],
    ) -> fmt::Result {
        if self.brief.is_none()
            && self.description.is_none()
            && self.params.is_empty()
            && self.returns.is_none()
        {
            return Ok(());
        }
        let first_prefix = "/** ";
        let prefix = " * ";
        let cont = " *   ";
        let mut first_line_written = false;
        if let Some(brief) = &self.brief {
            write_wrapped(w, first_prefix, cont, brief)?;
            first_line_written = true;
        }
        if let Some(desc) = &self.description {
            if first_line_written {
                w.write_str(" *\n")?;
            }
            for (i, line) in desc.lines().enumerate() {
                if line.trim().is_empty() {
                    w.write_str(" *\n")?;
                } else {
                    let p = if !first_line_written && i == 0 {
                        first_prefix
                    } else {
                        prefix
                    };
                    write_wrapped(w, p, cont, line)?;
                    first_line_written = true;
                }
            }
        }
        if !params.is_empty() {
            let has_any_doc = params.iter().any(|(_, d)| d.is_some());
            if has_any_doc {
                if first_line_written {
                    w.write_str(" *\n")?;
                }
                for (j, (name, d)) in params.iter().enumerate() {
                    if let Some(d) = d {
                        let p = if !first_line_written && j == 0 {
                            format!("/** @param {name} ")
                        } else {
                            format!(" * @param {name} ")
                        };
                        write_wrapped(w, &p, cont, d)?;
                        first_line_written = true;
                    }
                }
            }
        }
        if let Some(ret) = &self.returns {
            if first_line_written {
                w.write_str(" *\n")?;
            }
            let p = if !first_line_written {
                "/** @return "
            } else {
                " * @return "
            };
            write_wrapped(w, p, cont, ret)?;
        }
        w.write_str(" */\n")
    }
}

/// Collapses single line breaks into spaces, preserving double line breaks
/// (paragraph separators).
fn collapse_single_newlines(text: &str) -> String {
    text.split("\n\n")
        .map(|paragraph| {
            paragraph
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Extracts the raw doc string from `#[doc = "..."]` attributes, joining
/// multiple doc lines with newlines and stripping leading whitespace uniformly.
fn extract_doc_string(attrs: &[syn::Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            if let syn::Meta::NameValue(nv) = &attr.meta
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
            {
                return Some(s.value());
            }
            None
        })
        .collect();

    if lines.is_empty() {
        return None;
    }

    // Each doc line typically has a leading space from `/// text` → `#[doc = " text"]`
    let joined: String = lines
        .iter()
        .map(|l| {
            if let Some(stripped) = l.strip_prefix(' ') {
                stripped
            } else {
                l.as_str()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let trimmed = joined.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(convert_rust_links_to_doxygen(&trimmed))
    }
}

/// Converts Rust intra-doc links to doxygen cross-references.
///
/// - `` [`Foo`] `` → `@ref Foo`
/// - `` [`Foo::bar`] `` → `@ref Foo::bar`
/// - `` [text](`path`) `` → `text (@ref path)`
fn convert_rust_links_to_doxygen(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'[' {
            // Try [`path`] form
            if i + 2 < len
                && bytes[i + 1] == b'`'
                && let Some(backtick_end) = text[i + 2..].find('`')
            {
                let path_end = i + 2 + backtick_end;
                if path_end + 1 < len && bytes[path_end + 1] == b']' {
                    let path = &text[i + 2..path_end];
                    result.push_str("@ref ");
                    result.push_str(path);
                    i = path_end + 2;
                    continue;
                }
            }
            // Try [text](`path`) form
            if let Some(bracket_end) = text[i + 1..].find(']') {
                let bracket_end = i + 1 + bracket_end;
                if bracket_end + 3 < len
                    && bytes[bracket_end + 1] == b'('
                    && bytes[bracket_end + 2] == b'`'
                    && let Some(backtick_end) = text[bracket_end + 3..].find('`')
                {
                    let path_end = bracket_end + 3 + backtick_end;
                    if path_end + 1 < len && bytes[path_end + 1] == b')' {
                        let link_text = &text[i + 1..bracket_end];
                        let path = &text[bracket_end + 3..path_end];
                        result.push_str(link_text);
                        result.push_str(" (@ref ");
                        result.push_str(path);
                        result.push(')');
                        i = path_end + 2;
                        continue;
                    }
                }
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }

    result
}
