//! `aicc-parser`: language detection + Tree-sitter structural parsing.
//!
//! Fase 6: static extension registry + per-file metadata inputs. Detection is
//! deterministic, case-insensitive and total (never panics): unknown or
//! missing extensions yield [`Language::Unknown`].
//!
//! Fase 7: structural parsing for TypeScript (incl. TSX), JavaScript (incl.
//! JSX), Python and Rust via compiled-in Tree-sitter grammars (ADR 0004).
//! Parsing is syntactic only — no type resolution, no call inference, no
//! execution of the analyzed code — and tolerates broken input through
//! Tree-sitter error recovery. Results never carry raw source text.

use aicc_core::model::{Symbol, SymbolKind, Visibility};
use aicc_core::security::contains_secret;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unsupported language for: {0}")]
    Unsupported(String),
    #[error("grammar failed for {language}: {reason}")]
    Grammar { language: String, reason: String },
    #[error("parser failed for {path}: {reason}")]
    Failed { path: String, reason: String },
}

/// Language identifier (stable ids used in index + filters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    TypeScript,
    JavaScript,
    Python,
    Rust,
    Go,
    Java,
    CSharp,
    C,
    Cpp,
    Ruby,
    Php,
    Markdown,
    Json,
    Toml,
    Yaml,
    Unknown,
}

/// Where a language assignment came from. Fase 6 has a single source: the
/// static extension registry. Kept as an enum (not a string) so future
/// sources (shebang, content sniffing) are additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectionSource {
    Extension,
}

impl DetectionSource {
    /// Stable id for filters and JSON (matches serialized form).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Extension => "extension",
        }
    }
}

impl Language {
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Java => "java",
            Self::CSharp => "csharp",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Ruby => "ruby",
            Self::Php => "php",
            Self::Markdown => "markdown",
            Self::Json => "json",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Unknown => "unknown",
        }
    }

    /// Whether structural parsing is planned for v1 (ROADMAP Fase 7/35).
    /// Planning only: use [`Language::has_structural_parser`] to ask what is
    /// actually implemented.
    #[must_use]
    pub fn has_v1_structural_support(self) -> bool {
        matches!(self, Self::TypeScript | Self::JavaScript | Self::Python | Self::Rust)
    }

    /// Whether a structural parser is really implemented and available.
    /// Fase 7: `true` for TypeScript, JavaScript, Python and Rust; every
    /// other language stays unavailable. Kept separate from
    /// [`Language::has_v1_structural_support`] (the plan) so callers cannot
    /// mistake the roadmap for an implementation.
    #[must_use]
    pub fn has_structural_parser(self) -> bool {
        matches!(self, Self::TypeScript | Self::JavaScript | Self::Python | Self::Rust)
    }
}

/// Detect language from file extension (case-insensitive).
///
/// Total function: missing or unknown extensions yield [`Language::Unknown`]
/// instead of panicking. `.h` is ambiguous between C and C++; it falls back
/// to [`Language::C`] (historical choice) — the distinction is heuristic, not
/// perfect. Languages without structural support remain fully eligible for
/// metadata and future textual indexing.
#[must_use]
pub fn detect_language(path: &Path) -> Language {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
        "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
        "py" => Language::Python,
        "rs" => Language::Rust,
        "go" => Language::Go,
        "java" => Language::Java,
        "cs" => Language::CSharp,
        // `.h` fallback is C by convention (see docs); C++-only headers use
        // the unambiguous aliases below.
        "c" | "h" => Language::C,
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Language::Cpp,
        "rb" => Language::Ruby,
        "php" => Language::Php,
        "md" | "markdown" => Language::Markdown,
        "json" => Language::Json,
        "toml" => Language::Toml,
        "yaml" | "yml" => Language::Yaml,
        _ => Language::Unknown,
    }
}

/// Static placeholder stored instead of an import source that matches a
/// high-confidence secret pattern. Never the original value.
pub const REDACTED_MODULE: &str = "[redacted]";

/// Cap on reported syntax diagnostics per file: bounded output, tree order.
const MAX_DIAGNOSTICS: usize = 32;

/// A syntactic import fact (not a graph edge — Fase 8 owns relations).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDeclaration {
    /// Module specifier when determinable (quotes stripped, secret-like
    /// values replaced by [`REDACTED_MODULE`]).
    pub source: Option<String>,
    /// Imported/bound names when identifiable (aliases resolved to the bound
    /// name); empty when the statement binds nothing nameable.
    pub names: Vec<String>,
    /// First line of the statement (1-based, inclusive).
    pub start_line: u32,
    /// Last line of the statement (1-based, inclusive).
    pub end_line: u32,
}

/// A syntactic export fact (not a graph edge). Python has no explicit export
/// syntax, so its results always carry an empty list (documented, not a gap).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportDeclaration {
    /// Exported names (`[]` for `export *`).
    pub names: Vec<String>,
    /// Whether this is a default export.
    pub is_default: bool,
    /// Origin module for re-exports (`export … from "m"`), if present.
    pub source: Option<String>,
    /// First line of the statement (1-based, inclusive).
    pub start_line: u32,
    /// Last line of the statement (1-based, inclusive).
    pub end_line: u32,
}

/// Category of a recoverable syntax problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticKind {
    Error,
    Missing,
}

/// A structured, content-free syntax diagnostic: category, grammar node kind
/// and location only. Never source text or secret values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseDiagnostic {
    pub kind: DiagnosticKind,
    pub node_kind: String,
    pub start_line: u32,
    pub end_line: u32,
}

/// The complete outcome of parsing one file. Symbol/import/export lists are
/// in tree (source) order and deterministic for the same input. No raw
/// source text is stored anywhere in this structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseResult {
    pub language: Language,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<ImportDeclaration>,
    pub exports: Vec<ExportDeclaration>,
    pub diagnostics: Vec<ParseDiagnostic>,
    /// `true` when the tree contains error/missing nodes: the valid symbols
    /// above were still recovered, but the tree is not fully valid.
    pub has_syntax_errors: bool,
}

/// Structural parser seam (ADR 0004). Implementors tolerate broken code via
/// Tree-sitter error recovery and never execute repo code.
pub trait LanguageParser: std::fmt::Debug {
    fn language(&self) -> Language;
    /// Full structural result for `content` attributed to `rel_path`.
    fn parse(&self, rel_path: &str, content: &str) -> Result<ParseResult, ParseError>;
    /// Simplified adapter over [`LanguageParser::parse`].
    fn parse_symbols(
        &self,
        rel_path: &str,
        content: &str,
    ) -> Result<Vec<aicc_core::model::Symbol>, ParseError> {
        Ok(self.parse(rel_path, content)?.symbols)
    }
}

#[derive(Debug)]
struct TypeScriptParser {
    tsx: bool,
}

#[derive(Debug)]
struct JavaScriptParser;

#[derive(Debug)]
struct PythonParser;

#[derive(Debug)]
struct RustParser;

impl LanguageParser for TypeScriptParser {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn parse(&self, rel_path: &str, content: &str) -> Result<ParseResult, ParseError> {
        parse_with(rel_path, content, Language::TypeScript, grammar_typescript(self.tsx))
    }
}

impl LanguageParser for JavaScriptParser {
    fn language(&self) -> Language {
        Language::JavaScript
    }

    fn parse(&self, rel_path: &str, content: &str) -> Result<ParseResult, ParseError> {
        parse_with(rel_path, content, Language::JavaScript, grammar_javascript())
    }
}

impl LanguageParser for PythonParser {
    fn language(&self) -> Language {
        Language::Python
    }

    fn parse(&self, rel_path: &str, content: &str) -> Result<ParseResult, ParseError> {
        parse_with(rel_path, content, Language::Python, grammar_python())
    }
}

impl LanguageParser for RustParser {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn parse(&self, rel_path: &str, content: &str) -> Result<ParseResult, ParseError> {
        parse_with(rel_path, content, Language::Rust, grammar_rust())
    }
}

/// Select the parser for a supported language. The TypeScript parser defaults
/// to the non-TSX grammar; use [`parse_content`] (path-aware) to pick TSX for
/// `.tsx` files. Unsupported languages fail with [`ParseError::Unsupported`].
pub fn parser_for(language: Language) -> Result<Box<dyn LanguageParser>, ParseError> {
    match language {
        Language::TypeScript => Ok(Box::new(TypeScriptParser { tsx: false })),
        Language::JavaScript => Ok(Box::new(JavaScriptParser)),
        Language::Python => Ok(Box::new(PythonParser)),
        Language::Rust => Ok(Box::new(RustParser)),
        other => Err(ParseError::Unsupported(other.id().to_string())),
    }
}

/// Parse `content` attributed to `rel_path`, selecting language and grammar
/// variant from the path extension (`.tsx` uses the TSX grammar).
pub fn parse_content(rel_path: &str, content: &str) -> Result<ParseResult, ParseError> {
    let ext =
        Path::new(rel_path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "ts" | "mts" | "cts" => TypeScriptParser { tsx: false }.parse(rel_path, content),
        "tsx" => TypeScriptParser { tsx: true }.parse(rel_path, content),
        "js" | "mjs" | "cjs" | "jsx" => JavaScriptParser.parse(rel_path, content),
        "py" => PythonParser.parse(rel_path, content),
        "rs" => RustParser.parse(rel_path, content),
        _ => Err(ParseError::Unsupported(rel_path.to_string())),
    }
}

// ---------------------------------------------------------------------------
// Grammar loading (compiled-in crates only; nothing downloaded at runtime).
// An incompatible grammar ABI surfaces as `ParseError::Grammar`, never a
// panic or a fake successful parse.
// ---------------------------------------------------------------------------

fn grammar_typescript(tsx: bool) -> tree_sitter_language::LanguageFn {
    if tsx {
        tree_sitter_typescript::LANGUAGE_TSX
    } else {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT
    }
}

fn grammar_javascript() -> tree_sitter_language::LanguageFn {
    tree_sitter_javascript::LANGUAGE
}

fn grammar_python() -> tree_sitter_language::LanguageFn {
    tree_sitter_python::LANGUAGE
}

fn grammar_rust() -> tree_sitter_language::LanguageFn {
    tree_sitter_rust::LANGUAGE
}

fn parse_with(
    rel_path: &str,
    content: &str,
    language: Language,
    language_func: tree_sitter_language::LanguageFn,
) -> Result<ParseResult, ParseError> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language_func.into()).map_err(|err| ParseError::Grammar {
        language: language.id().to_string(),
        reason: err.to_string(),
    })?;
    let tree = parser.parse(content, None).ok_or_else(|| ParseError::Failed {
        path: rel_path.to_string(),
        reason: String::from("parser produced no tree"),
    })?;
    let root = tree.root_node();
    let bytes = content.as_bytes();
    let mut ctx = WalkContext::new(rel_path, bytes);
    match language {
        Language::TypeScript | Language::JavaScript => extract_ts_js(&mut ctx, root),
        Language::Python => extract_python(&mut ctx, root),
        Language::Rust => extract_rust(&mut ctx, root),
        _ => {}
    }
    collect_diagnostics(&mut ctx, root);
    let has_syntax_errors = root.has_error() || !ctx.diagnostics.is_empty();
    Ok(ParseResult {
        language,
        symbols: ctx.symbols,
        imports: ctx.imports,
        exports: ctx.exports,
        diagnostics: ctx.diagnostics,
        has_syntax_errors,
    })
}

// ---------------------------------------------------------------------------
// Shared walk infrastructure (positions, ids, diagnostics; no source kept).
// ---------------------------------------------------------------------------

struct WalkContext<'a> {
    rel_path: &'a str,
    bytes: &'a [u8],
    symbols: Vec<Symbol>,
    imports: Vec<ImportDeclaration>,
    exports: Vec<ExportDeclaration>,
    diagnostics: Vec<ParseDiagnostic>,
}

impl<'a> WalkContext<'a> {
    fn new(rel_path: &'a str, bytes: &'a [u8]) -> Self {
        Self {
            rel_path,
            bytes,
            symbols: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn text(&self, node: tree_sitter::Node<'_>) -> Option<&'a str> {
        std::str::from_utf8(node_text_bytes(self.bytes, node)).ok()
    }

    fn push_symbol(
        &mut self,
        kind: SymbolKind,
        name: String,
        node: tree_sitter::Node<'_>,
        parent: Option<&str>,
        visibility: Option<Visibility>,
    ) -> String {
        let (start_line, end_line) = span_lines(node);
        let id = symbol_id(self.rel_path, kind, &name, start_line, end_line);
        self.symbols.push(Symbol {
            id: id.clone(),
            name,
            kind,
            file_rel_path: self.rel_path.to_string(),
            start_line,
            end_line,
            parent: parent.map(str::to_string),
            visibility,
        });
        id
    }
}

fn node_text_bytes<'a>(bytes: &'a [u8], node: tree_sitter::Node<'_>) -> &'a [u8] {
    bytes.get(node.start_byte()..node.end_byte()).unwrap_or_default()
}

/// 1-based inclusive span. A node ending exactly at column 0 of a later row
/// owns no text on that row, so the end backs up one line instead of
/// reporting a phantom extra line.
fn span_lines(node: tree_sitter::Node<'_>) -> (u32, u32) {
    let start = node.start_position();
    let end = node.end_position();
    let start_line = start.row as u32 + 1;
    let mut end_line = end.row as u32 + 1;
    if end.column == 0 && end.row > start.row {
        end_line -= 1;
    }
    (start_line, end_line)
}

/// Deterministic FNV-1a 64 id over path, kind, name and span: distinct
/// declarations with equal names (or anonymous ones) still differ.
/// `DefaultHasher` is never used for persistent ids.
fn symbol_id(rel_path: &str, kind: SymbolKind, name: &str, start: u32, end: u32) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in format!("{rel_path}\0{kind:?}\0{name}\0{start}\0{end}").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Redact secret-like module specifiers before storing them.
fn redact_module(raw: &str) -> String {
    if contains_secret(raw) { REDACTED_MODULE.to_string() } else { raw.to_string() }
}

fn strip_quotes(raw: &str) -> &str {
    let trimmed = raw.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        let (first, last) = (bytes[0], bytes[trimmed.len() - 1]);
        if (first == b'"' || first == b'\'' || first == b'`') && last == first {
            return &trimmed[1..trimmed.len() - 1];
        }
    }
    trimmed
}

fn collect_diagnostics(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>) {
    if ctx.diagnostics.len() >= MAX_DIAGNOSTICS {
        return;
    }
    if node.kind() == "ERROR" {
        let (start_line, end_line) = span_lines(node);
        ctx.diagnostics.push(ParseDiagnostic {
            kind: DiagnosticKind::Error,
            node_kind: node.kind().to_string(),
            start_line,
            end_line,
        });
    } else if node.is_missing() {
        let (start_line, end_line) = span_lines(node);
        ctx.diagnostics.push(ParseDiagnostic {
            kind: DiagnosticKind::Missing,
            node_kind: node.kind().to_string(),
            start_line,
            end_line,
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_diagnostics(ctx, child);
        if ctx.diagnostics.len() >= MAX_DIAGNOSTICS {
            return;
        }
    }
}

/// First named child with one of `kinds`, if any.
fn named_child<'t>(node: tree_sitter::Node<'t>, kinds: &[&str]) -> Option<tree_sitter::Node<'t>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).find(|child| kinds.contains(&child.kind()))
}

// ---------------------------------------------------------------------------
// TypeScript / JavaScript (tree-sitter-typescript, tree-sitter-javascript).
// ---------------------------------------------------------------------------

fn extract_ts_js(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>) {
    walk_ts_js(ctx, node, None);
}

fn walk_ts_js(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>, parent: Option<&str>) {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" | "function_signature" => {
            let name = named_child(node, &["identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Function, name, node, parent, None);
            walk_children_ts_js(ctx, node, Some(&id));
        }
        "class_declaration" | "abstract_class_declaration" => {
            let name = named_child(node, &["type_identifier", "identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Class, name, node, parent, None);
            walk_children_ts_js(ctx, node, Some(&id));
        }
        "method_definition" => {
            let name = named_child(
                node,
                &["property_identifier", "private_property_identifier", "identifier"],
            )
            .and_then(|n| ctx.text(n))
            .unwrap_or("<anonymous>")
            .to_string();
            let id = ctx.push_symbol(SymbolKind::Method, name, node, parent, None);
            walk_children_ts_js(ctx, node, Some(&id));
        }
        "interface_declaration" => {
            let name = named_child(node, &["type_identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Interface, name, node, parent, None);
            walk_children_ts_js(ctx, node, Some(&id));
        }
        "type_alias_declaration" => {
            let name = named_child(node, &["type_identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Type, name, node, parent, None);
            walk_children_ts_js(ctx, node, Some(&id));
        }
        "enum_declaration" => {
            let name = named_child(node, &["identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Enum, name, node, parent, None);
            walk_children_ts_js(ctx, node, Some(&id));
        }
        "module" | "namespace" => {
            let name = named_child(node, &["identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Module, name, node, parent, None);
            walk_children_ts_js(ctx, node, Some(&id));
        }
        "lexical_declaration" | "variable_declaration" => {
            // One symbol per declarator; function values become Functions so
            // `const f = () => {}` is not double-counted as Constant.
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "variable_declarator" {
                    emit_ts_declarator(ctx, child, parent);
                }
            }
        }
        "field_definition" => {
            // Arrow/class-field methods (`handler = () => {}`) count as methods.
            let name = named_child(
                node,
                &["property_identifier", "private_property_identifier", "identifier"],
            )
            .and_then(|n| ctx.text(n))
            .map(str::to_string);
            let value = named_child(node, &["arrow_function", "function_expression", "function"]);
            match (name, value) {
                (Some(name), Some(_)) => {
                    let id = ctx.push_symbol(SymbolKind::Method, name, node, parent, None);
                    walk_children_ts_js(ctx, node, Some(&id));
                }
                _ => walk_children_ts_js(ctx, node, parent),
            }
        }
        "import_statement" => emit_ts_import(ctx, node),
        "export_statement" => emit_ts_export(ctx, node, parent),
        _ => walk_children_ts_js(ctx, node, parent),
    }
}

fn walk_children_ts_js(
    ctx: &mut WalkContext<'_>,
    node: tree_sitter::Node<'_>,
    parent: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_ts_js(ctx, child, parent);
    }
}

fn emit_ts_declarator(
    ctx: &mut WalkContext<'_>,
    node: tree_sitter::Node<'_>,
    parent: Option<&str>,
) {
    let name = named_child(node, &["identifier"]).and_then(|n| ctx.text(n)).map(str::to_string);
    let Some(name) = name else {
        walk_children_ts_js(ctx, node, parent);
        return;
    };
    let value = named_child(node, &["arrow_function", "function_expression"]);
    if value.is_some() {
        let id = ctx.push_symbol(SymbolKind::Function, name, node, parent, None);
        walk_children_ts_js(ctx, node, Some(&id));
    } else {
        let id = ctx.push_symbol(SymbolKind::Constant, name, node, parent, None);
        walk_children_ts_js(ctx, node, Some(&id));
    }
}

fn ts_string_source(ctx: &WalkContext<'_>, node: tree_sitter::Node<'_>) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string" {
            let raw = ctx.text(child)?;
            return Some(redact_module(strip_quotes(raw)));
        }
    }
    None
}

fn ts_specifier_name(ctx: &WalkContext<'_>, node: tree_sitter::Node<'_>) -> Option<String> {
    // `import {a as b}` / `export {a as b}` bind `b` when an alias exists.
    let mut cursor = node.walk();
    let mut name: Option<&str> = None;
    let mut alias: Option<&str> = None;
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "identifier" if name.is_none() => name = ctx.text(child),
            "identifier" => alias = ctx.text(child),
            _ => {}
        }
    }
    alias.or(name).map(str::to_string)
}

fn emit_ts_import(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>) {
    let (start_line, end_line) = span_lines(node);
    let mut names: Vec<String> = Vec::new();
    let mut cursor = node.walk();
    // Names live inside `import_clause` (default identifier, `named_imports`
    // or `namespace_import`); the source `string` is a direct child.
    for child in node.named_children(&mut cursor) {
        if child.kind() == "import_clause" {
            collect_ts_clause_names(ctx, child, &mut names);
        }
    }
    ctx.imports.push(ImportDeclaration {
        source: ts_string_source(ctx, node),
        names,
        start_line,
        end_line,
    });
}

fn collect_ts_clause_names(
    ctx: &WalkContext<'_>,
    clause: tree_sitter::Node<'_>,
    names: &mut Vec<String>,
) {
    let mut cursor = clause.walk();
    for child in clause.named_children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                // Default import: `import D from "m"`.
                if let Some(text) = ctx.text(child) {
                    names.push(text.to_string());
                }
            }
            "named_imports" => {
                let mut inner = child.walk();
                for spec in child.named_children(&mut inner) {
                    if spec.kind() == "import_specifier"
                        && let Some(name) = ts_specifier_name(ctx, spec)
                    {
                        names.push(name);
                    }
                }
            }
            "namespace_import" => {
                let mut inner = child.walk();
                for grand in child.named_children(&mut inner) {
                    if grand.kind() == "identifier"
                        && let Some(text) = ctx.text(grand)
                    {
                        names.push(text.to_string());
                    }
                }
            }
            _ => {}
        }
    }
}

fn emit_ts_export(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>, parent: Option<&str>) {
    let (start_line, end_line) = span_lines(node);
    let mut is_default = false;
    let mut names: Vec<String> = Vec::new();
    let mut source: Option<String> = None;
    let mut declaration: Option<tree_sitter::Node<'_>> = None;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "default" => is_default = true,
            "export_clause" => {
                let mut inner = child.walk();
                for spec in child.named_children(&mut inner) {
                    if spec.kind() == "export_specifier"
                        && let Some(name) = ts_specifier_name(ctx, spec)
                    {
                        names.push(name);
                    }
                }
            }
            "namespace_export" => {
                let mut inner = child.walk();
                for grand in child.named_children(&mut inner) {
                    if grand.kind() == "identifier"
                        && let Some(text) = ctx.text(grand)
                    {
                        names.push(text.to_string());
                    }
                }
            }
            "string" => {
                if let Some(raw) = ctx.text(child) {
                    source = Some(redact_module(strip_quotes(raw)));
                }
            }
            _ if child.is_named() => {
                if matches!(
                    child.kind(),
                    "function_declaration"
                        | "generator_function_declaration"
                        | "function_signature"
                        | "class_declaration"
                        | "abstract_class_declaration"
                        | "interface_declaration"
                        | "type_alias_declaration"
                        | "enum_declaration"
                        | "lexical_declaration"
                        | "variable_declaration"
                        | "module"
                        | "namespace"
                ) {
                    declaration = Some(child);
                }
            }
            _ => {}
        }
    }
    // A declared name (`export function f`, `export const x`) is recorded on
    // the export entry; the declaration itself is emitted once by the walker.
    if names.is_empty() {
        if let Some(decl) = declaration {
            names = ts_declared_names(ctx, decl);
        } else if is_default {
            // `export default <expression>;`: the `value` field carries the
            // exported identifier when the expression is a plain name.
            let mut inner = node.walk();
            for child in node.named_children(&mut inner) {
                if child.kind() == "identifier" {
                    if let Some(text) = ctx.text(child) {
                        names.push(text.to_string());
                    }
                    break;
                }
            }
        }
    }
    // `export * from "m"`: no names, only a source.
    ctx.exports.push(ExportDeclaration { names, is_default, source, start_line, end_line });
    // Walk the wrapped declaration exactly once (no duplicate symbols for the
    // `export` wrapper itself).
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_ts_js(ctx, child, parent);
    }
}

/// Declared binding names of an exported declaration (no bodies copied).
fn ts_declared_names(ctx: &WalkContext<'_>, node: tree_sitter::Node<'_>) -> Vec<String> {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" | "function_signature" => {
            named_child(node, &["identifier"])
                .and_then(|n| ctx.text(n))
                .map(|s| vec![s.to_string()])
                .unwrap_or_default()
        }
        "class_declaration"
        | "abstract_class_declaration"
        | "interface_declaration"
        | "type_alias_declaration"
        | "enum_declaration" => named_child(node, &["type_identifier", "identifier"])
            .and_then(|n| ctx.text(n))
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        "lexical_declaration" | "variable_declaration" => {
            let mut out = Vec::new();
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "variable_declarator"
                    && let Some(name) =
                        named_child(child, &["identifier"]).and_then(|n| ctx.text(n))
                {
                    out.push(name.to_string());
                }
            }
            out
        }
        "module" | "namespace" => named_child(node, &["identifier"])
            .and_then(|n| ctx.text(n))
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Python (tree-sitter-python). No explicit exports exist in the language.
// ---------------------------------------------------------------------------

fn extract_python(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>) {
    walk_python(ctx, node, None);
}

fn walk_python(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>, parent: Option<&str>) {
    match node.kind() {
        "decorated_definition" => {
            // One symbol for the decorated declaration (inner span), never a
            // second symbol for the decorator wrapper itself.
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "function_definition" || child.kind() == "class_definition" {
                    walk_python(ctx, child, parent);
                    return;
                }
            }
            walk_children_python(ctx, node, parent);
        }
        "function_definition" => {
            let name = named_child(node, &["identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let kind = match parent_kind(ctx, parent) {
                Some(SymbolKind::Class) => SymbolKind::Method,
                _ => SymbolKind::Function,
            };
            let id = ctx.push_symbol(kind, name, node, parent, None);
            walk_children_python(ctx, node, Some(&id));
        }
        "class_definition" => {
            let name = named_child(node, &["identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Class, name, node, parent, None);
            walk_children_python(ctx, node, Some(&id));
        }
        "import_statement" => emit_python_import(ctx, node),
        "import_from_statement" => emit_python_import_from(ctx, node),
        "expression_statement" => {
            // Module-level ALL_CAPS assignment (`LIMIT = 100`) counts as a
            // constant; anything else is left alone (no value heuristics).
            if parent.is_none()
                && let Some(name) = python_const_target(ctx, node)
            {
                let id = ctx.push_symbol(SymbolKind::Constant, name, node, parent, None);
                walk_children_python(ctx, node, Some(&id));
                return;
            }
            walk_children_python(ctx, node, parent);
        }
        _ => walk_children_python(ctx, node, parent),
    }
}

fn walk_children_python(
    ctx: &mut WalkContext<'_>,
    node: tree_sitter::Node<'_>,
    parent: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_python(ctx, child, parent);
    }
}

fn parent_kind(ctx: &WalkContext<'_>, parent: Option<&str>) -> Option<SymbolKind> {
    let id = parent?;
    ctx.symbols.iter().rev().find(|s| s.id == id).map(|s| s.kind)
}

/// ALL_CAPS binding target of `NAME = …` at module level, if present.
fn python_const_target(ctx: &WalkContext<'_>, node: tree_sitter::Node<'_>) -> Option<String> {
    let mut cursor = node.walk();
    let mut children = node.named_children(&mut cursor);
    let first = children.next()?;
    if first.kind() != "assignment" {
        return None;
    }
    let mut inner = first.walk();
    let mut targets = first.named_children(&mut inner);
    let target = targets.next()?;
    if target.kind() != "identifier" {
        return None;
    }
    let text = ctx.text(target)?;
    if text.len() > 1
        && text.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
    {
        return Some(text.to_string());
    }
    None
}

fn emit_python_import(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>) {
    let (start_line, end_line) = span_lines(node);
    let mut names: Vec<String> = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "dotted_name" => {
                if let Some(text) = ctx.text(child) {
                    names.push(text.to_string());
                }
            }
            "aliased_import" => {
                if let Some(name) = python_alias_target(ctx, child) {
                    names.push(name);
                }
            }
            _ => {}
        }
    }
    ctx.imports.push(ImportDeclaration { source: None, names, start_line, end_line });
}

fn emit_python_import_from(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>) {
    let (start_line, end_line) = span_lines(node);
    // The module slice between the `from` and `import` keywords preserves
    // relative dots (`.foo`) exactly as written.
    let mut from_end: Option<usize> = None;
    let mut import_start: Option<usize> = None;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            if child.kind() == "from" && from_end.is_none() {
                from_end = Some(child.end_byte());
            } else if child.kind() == "import" && import_start.is_none() {
                import_start = Some(child.start_byte());
            }
        }
    }
    let source = match (from_end, import_start) {
        (Some(a), Some(b)) if a <= b => {
            let raw =
                std::str::from_utf8(ctx.bytes.get(a..b).unwrap_or_default()).unwrap_or("").trim();
            if raw.is_empty() { None } else { Some(redact_module(raw)) }
        }
        _ => None,
    };
    let mut names: Vec<String> = Vec::new();
    let mut cursor = node.walk();
    let mut after_import = false;
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            if child.kind() == "import" {
                after_import = true;
            }
            if child.kind() == "*" && after_import {
                names.push("*".to_string());
            }
            continue;
        }
        if !after_import {
            continue;
        }
        match child.kind() {
            "identifier" | "dotted_name" => {
                if let Some(text) = ctx.text(child) {
                    names.push(text.to_string());
                }
            }
            "aliased_import" => {
                if let Some(name) = python_alias_target(ctx, child) {
                    names.push(name);
                }
            }
            _ => {}
        }
    }
    ctx.imports.push(ImportDeclaration { source, names, start_line, end_line });
}

/// Bound name of `x as y` (alias wins) or the plain name.
fn python_alias_target(ctx: &WalkContext<'_>, node: tree_sitter::Node<'_>) -> Option<String> {
    let mut cursor = node.walk();
    let mut first: Option<&str> = None;
    let mut second: Option<&str> = None;
    for child in node.named_children(&mut cursor) {
        if first.is_none() {
            first = ctx.text(child);
        } else if second.is_none() {
            second = ctx.text(child);
        }
    }
    second.or(first).map(str::to_string)
}

// ---------------------------------------------------------------------------
// Rust (tree-sitter-rust).
// ---------------------------------------------------------------------------

fn extract_rust(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>) {
    walk_rust(ctx, node, None);
}

fn walk_rust(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>, parent: Option<&str>) {
    match node.kind() {
        "function_item" | "function_signature_item" => {
            let name = named_child(node, &["identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let kind = match parent_kind(ctx, parent) {
                Some(SymbolKind::Impl) | Some(SymbolKind::Trait) | Some(SymbolKind::Class) => {
                    SymbolKind::Method
                }
                _ => SymbolKind::Function,
            };
            let visibility = rust_visibility(node);
            let id = ctx.push_symbol(kind, name, node, parent, visibility);
            walk_children_rust(ctx, node, Some(&id));
        }
        "impl_item" => {
            // The implemented type's text names the block (never a body).
            let name = node
                .child_by_field_name("type")
                .and_then(|n| ctx.text(n))
                .unwrap_or("<impl>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Impl, name, node, parent, None);
            walk_children_rust(ctx, node, Some(&id));
        }
        "struct_item" => {
            let name = named_child(node, &["type_identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Struct, name, node, parent, rust_visibility(node));
            walk_children_rust(ctx, node, Some(&id));
        }
        "enum_item" => {
            let name = named_child(node, &["type_identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Enum, name, node, parent, rust_visibility(node));
            walk_children_rust(ctx, node, Some(&id));
        }
        "trait_item" => {
            let name = named_child(node, &["type_identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Trait, name, node, parent, rust_visibility(node));
            walk_children_rust(ctx, node, Some(&id));
        }
        "type_item" => {
            let name = named_child(node, &["type_identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Type, name, node, parent, rust_visibility(node));
            walk_children_rust(ctx, node, Some(&id));
        }
        "const_item" | "static_item" => {
            let name = named_child(node, &["identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id =
                ctx.push_symbol(SymbolKind::Constant, name, node, parent, rust_visibility(node));
            walk_children_rust(ctx, node, Some(&id));
        }
        "mod_item" => {
            let name = named_child(node, &["identifier"])
                .and_then(|n| ctx.text(n))
                .unwrap_or("<anonymous>")
                .to_string();
            let id = ctx.push_symbol(SymbolKind::Module, name, node, parent, rust_visibility(node));
            walk_children_rust(ctx, node, Some(&id));
        }
        "use_declaration" => emit_rust_use(ctx, node),
        _ => walk_children_rust(ctx, node, parent),
    }
}

fn walk_children_rust(
    ctx: &mut WalkContext<'_>,
    node: tree_sitter::Node<'_>,
    parent: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_rust(ctx, child, parent);
    }
}

/// Rust visibility from the language's own modifier: `pub…` → Public,
/// otherwise Private. Never invented beyond the modifier's presence.
fn rust_visibility(node: tree_sitter::Node<'_>) -> Option<Visibility> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            return Some(Visibility::Public);
        }
    }
    Some(Visibility::Private)
}

fn emit_rust_use(ctx: &mut WalkContext<'_>, node: tree_sitter::Node<'_>) {
    let (start_line, end_line) = span_lines(node);
    let argument = named_child(
        node,
        &[
            "scoped_identifier",
            "scoped_use_list",
            "use_list",
            "use_as_clause",
            "identifier",
            "use_wildcard",
        ],
    );
    let Some(argument) = argument else { return };
    let raw = ctx.text(argument).unwrap_or("").trim();
    if raw.is_empty() {
        return;
    }
    // Collapse internal whitespace so multiline `use` stays one stable value.
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let (source, names) = rust_use_parts(&collapsed);
    ctx.imports.push(ImportDeclaration {
        source: Some(redact_module(&source)),
        names,
        start_line,
        end_line,
    });
}

/// Split a `use` tree into its leading path and bound names:
/// `a::{x, y as z}` → (`a`, [x, z]); `a::b` → (`a::b`, [b]).
fn rust_use_parts(tree: &str) -> (String, Vec<String>) {
    if let Some(brace) = tree.find('{') {
        let base = tree[..brace].trim().trim_end_matches(':').trim().to_string();
        let inner = tree[brace..].trim_matches(|c| c == '{' || c == '}');
        let mut names = Vec::new();
        for part in inner.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            // `x as y` binds `y`; plain `x` binds itself.
            let bound = match part.split_once(" as ") {
                Some((_, alias)) => alias.trim(),
                None => part,
            };
            names.push(bound.trim_matches(',').to_string());
        }
        (base, names)
    } else if let Some((left, right)) = tree.rsplit_once(" as ") {
        (left.trim().to_string(), vec![right.trim().to_string()])
    } else {
        let tail = tree.rsplit("::").next().unwrap_or(tree).trim().to_string();
        (tree.to_string(), vec![tail])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/parser").join(name)
    }

    fn parse_fixture(name: &str) -> ParseResult {
        let path = fixture_path(name);
        let content = std::fs::read_to_string(&path).expect("fixture must exist");
        parse_content(path.to_str().expect("utf8 path"), &content).expect("fixture must parse")
    }

    fn symbol_by_name<'a>(result: &'a ParseResult, name: &str) -> &'a Symbol {
        result.symbols.iter().find(|s| s.name == name).expect("named symbol")
    }

    #[test]
    fn detects_priority_languages() {
        assert_eq!(detect_language(&PathBuf::from("a.ts")), Language::TypeScript);
        assert_eq!(detect_language(&PathBuf::from("a.tsx")), Language::TypeScript);
        assert_eq!(detect_language(&PathBuf::from("a.mts")), Language::TypeScript);
        assert_eq!(detect_language(&PathBuf::from("a.cts")), Language::TypeScript);
        assert_eq!(detect_language(&PathBuf::from("a.js")), Language::JavaScript);
        assert_eq!(detect_language(&PathBuf::from("a.jsx")), Language::JavaScript);
        assert_eq!(detect_language(&PathBuf::from("a.mjs")), Language::JavaScript);
        assert_eq!(detect_language(&PathBuf::from("a.cjs")), Language::JavaScript);
        assert_eq!(detect_language(&PathBuf::from("a.py")), Language::Python);
        assert_eq!(detect_language(&PathBuf::from("a.rs")), Language::Rust);
    }

    #[test]
    fn detects_second_wave_languages() {
        assert_eq!(detect_language(&PathBuf::from("a.go")), Language::Go);
        assert_eq!(detect_language(&PathBuf::from("A.java")), Language::Java);
        assert_eq!(detect_language(&PathBuf::from("a.cs")), Language::CSharp);
        assert_eq!(detect_language(&PathBuf::from("a.c")), Language::C);
        assert_eq!(detect_language(&PathBuf::from("a.cpp")), Language::Cpp);
        assert_eq!(detect_language(&PathBuf::from("a.cc")), Language::Cpp);
        assert_eq!(detect_language(&PathBuf::from("a.hpp")), Language::Cpp);
    }

    #[test]
    fn detects_cpp_aliases() {
        for name in ["a.cxx", "a.hh", "a.hxx"] {
            assert_eq!(detect_language(&PathBuf::from(name)), Language::Cpp, "{name}");
        }
    }

    #[test]
    fn h_falls_back_to_c_and_is_documented() {
        // Ambiguous header: kept as C by convention, not a perfect split.
        assert_eq!(detect_language(&PathBuf::from("a.h")), Language::C);
    }

    #[test]
    fn detects_textual_formats() {
        assert_eq!(detect_language(&PathBuf::from("a.md")), Language::Markdown);
        assert_eq!(detect_language(&PathBuf::from("a.markdown")), Language::Markdown);
        assert_eq!(detect_language(&PathBuf::from("a.json")), Language::Json);
        assert_eq!(detect_language(&PathBuf::from("a.toml")), Language::Toml);
        assert_eq!(detect_language(&PathBuf::from("a.yaml")), Language::Yaml);
        assert_eq!(detect_language(&PathBuf::from("a.yml")), Language::Yaml);
    }

    #[test]
    fn detection_is_case_insensitive() {
        assert_eq!(detect_language(&PathBuf::from("A.TS")), Language::TypeScript);
        assert_eq!(detect_language(&PathBuf::from("B.Py")), Language::Python);
        assert_eq!(detect_language(&PathBuf::from("C.RS")), Language::Rust);
        assert_eq!(detect_language(&PathBuf::from("D.CPP")), Language::Cpp);
        assert_eq!(detect_language(&PathBuf::from("E.MD")), Language::Markdown);
    }

    #[test]
    fn unknown_for_missing_extension() {
        assert_eq!(detect_language(&PathBuf::from("Makefile")), Language::Unknown);
    }

    #[test]
    fn unknown_for_unknown_extension_and_special_names() {
        for name in ["a.zzz9", "Dockerfile", ".gitignore", "archive.tar.gz9"] {
            assert_eq!(detect_language(&PathBuf::from(name)), Language::Unknown, "{name}");
        }
    }

    #[test]
    fn language_ids_match_serialization() {
        for lang in [
            Language::TypeScript,
            Language::JavaScript,
            Language::Python,
            Language::Rust,
            Language::Go,
            Language::Java,
            Language::CSharp,
            Language::C,
            Language::Cpp,
            Language::Ruby,
            Language::Php,
            Language::Markdown,
            Language::Json,
            Language::Toml,
            Language::Yaml,
            Language::Unknown,
        ] {
            let json = serde_json::to_value(lang).expect("serializable");
            assert_eq!(json, serde_json::Value::String(lang.id().to_string()), "{lang:?}");
        }
        let source = serde_json::to_value(DetectionSource::Extension).expect("serializable");
        assert_eq!(source, serde_json::Value::String("extension".to_string()));
        assert_eq!(DetectionSource::Extension.id(), "extension");
    }

    #[test]
    fn structural_parser_availability_matches_fase_7() {
        for lang in [Language::TypeScript, Language::JavaScript, Language::Python, Language::Rust] {
            assert!(lang.has_structural_parser(), "{lang:?} must be available");
            assert!(parser_for(lang).is_ok(), "{lang:?} must select a parser");
        }
        for lang in [Language::Go, Language::Java, Language::Unknown] {
            assert!(!lang.has_structural_parser(), "{lang:?} stays unavailable");
            assert!(parser_for(lang).is_err(), "{lang:?} must not select a parser");
        }
        // The plan still names the v1 set; it is not an implementation claim.
        assert!(Language::Rust.has_v1_structural_support());
        assert!(!Language::Go.has_v1_structural_support());
    }

    #[test]
    fn unsupported_language_is_typed() {
        let err = parse_content("a.go", "package main\n").unwrap_err();
        assert!(matches!(err, ParseError::Unsupported(_)));
        // Error rendering carries no source text.
        assert!(!err.to_string().contains("package main"));
    }

    #[test]
    fn typescript_fixture_symbols_hierarchy_and_positions() {
        let result = parse_fixture("sample.ts");
        assert_eq!(result.language, Language::TypeScript);
        assert!(!result.has_syntax_errors, "{:?}", result.diagnostics);

        let config = symbol_by_name(&result, "Config");
        assert_eq!(config.kind, SymbolKind::Interface);
        assert_eq!((config.start_line, config.end_line), (5, 8));
        assert_eq!(config.parent, None);

        let alias = symbol_by_name(&result, "Alias");
        assert_eq!(alias.kind, SymbolKind::Type);

        let mode = symbol_by_name(&result, "Mode");
        assert_eq!(mode.kind, SymbolKind::Enum);

        let version = symbol_by_name(&result, "VERSION");
        assert_eq!(version.kind, SymbolKind::Constant);

        let greet = symbol_by_name(&result, "greet");
        assert_eq!(greet.kind, SymbolKind::Function);
        assert_eq!((greet.start_line, greet.end_line), (19, 21));

        let iterate = symbol_by_name(&result, "iterate");
        assert_eq!(iterate.kind, SymbolKind::Function);

        let helper = symbol_by_name(&result, "helper");
        assert_eq!(helper.kind, SymbolKind::Function);

        let service = symbol_by_name(&result, "Service");
        assert_eq!(service.kind, SymbolKind::Class);
        let run = symbol_by_name(&result, "run");
        assert_eq!(run.kind, SymbolKind::Method);
        assert_eq!(run.parent.as_deref(), Some(service.id.as_str()));
        let ctor = symbol_by_name(&result, "constructor");
        assert_eq!(ctor.parent.as_deref(), Some(service.id.as_str()));

        // No duplicate symbols for the `export` wrappers themselves.
        assert_eq!(result.symbols.iter().filter(|s| s.name == "greet").count(), 1);
    }

    #[test]
    fn typescript_fixture_imports_and_exports() {
        let result = parse_fixture("sample.ts");
        assert_eq!(result.imports.len(), 3);
        assert!(
            result
                .imports
                .iter()
                .any(|i| i.source.as_deref() == Some("node:fs") && i.names == vec!["readFile"])
        );
        assert!(
            result
                .imports
                .iter()
                .any(|i| i.source.as_deref() == Some("./api")
                    && i.names == vec!["Api", "ApiOptions"])
        );
        let export_names: Vec<&str> =
            result.exports.iter().flat_map(|e| e.names.iter().map(String::as_str)).collect();
        for expected in ["Config", "greet", "Service", "greetAlias", "helper"] {
            assert!(export_names.contains(&expected), "missing export {expected}");
        }
        let star = result.exports.iter().find(|e| e.source.as_deref() == Some("./extra"));
        assert!(star.is_some_and(|e| e.names.is_empty() && !e.is_default));
        let default = result.exports.iter().find(|e| e.is_default);
        assert!(default.is_some_and(|e| e.names == vec!["helper"]));
    }

    #[test]
    fn tsx_uses_tsx_grammar() {
        let result = parse_fixture("sample.tsx");
        assert_eq!(result.language, Language::TypeScript);
        assert!(!result.has_syntax_errors, "{:?}", result.diagnostics);
        let button = symbol_by_name(&result, "Button");
        assert_eq!(button.kind, SymbolKind::Function);
        let label = symbol_by_name(&result, "Label");
        assert_eq!(label.kind, SymbolKind::Function);
    }

    #[test]
    fn javascript_fixture_symbols_and_exports() {
        let result = parse_fixture("sample.js");
        assert_eq!(result.language, Language::JavaScript);
        assert!(!result.has_syntax_errors, "{:?}", result.diagnostics);
        assert_eq!(symbol_by_name(&result, "add").kind, SymbolKind::Function);
        assert_eq!(symbol_by_name(&result, "multiply").kind, SymbolKind::Function);
        let counter = symbol_by_name(&result, "Counter");
        assert_eq!(counter.kind, SymbolKind::Class);
        assert_eq!(
            symbol_by_name(&result, "increment").parent.as_deref(),
            Some(counter.id.as_str())
        );
        assert_eq!(symbol_by_name(&result, "answer").kind, SymbolKind::Constant);
        assert!(result.exports.iter().any(|e| e.is_default && e.names == vec!["add"]));
        assert_eq!(result.imports.len(), 2);
    }

    #[test]
    fn jsx_parses_without_errors() {
        let result = parse_fixture("sample.jsx");
        assert!(!result.has_syntax_errors, "{:?}", result.diagnostics);
        assert_eq!(symbol_by_name(&result, "App").kind, SymbolKind::Function);
    }

    #[test]
    fn python_fixture_symbols_imports_and_positions() {
        let result = parse_fixture("sample.py");
        assert_eq!(result.language, Language::Python);
        assert!(!result.has_syntax_errors, "{:?}", result.diagnostics);
        // No invented exports in a language without export syntax.
        assert!(result.exports.is_empty());

        let load = symbol_by_name(&result, "load");
        assert_eq!(load.kind, SymbolKind::Function);
        assert_eq!((load.start_line, load.end_line), (9, 11));

        let store = symbol_by_name(&result, "Store");
        assert_eq!(store.kind, SymbolKind::Class);
        let get = symbol_by_name(&result, "get");
        assert_eq!(get.kind, SymbolKind::Method);
        assert_eq!(get.parent.as_deref(), Some(store.id.as_str()));

        // Decorated definitions keep one symbol with the declaration span.
        let size = symbol_by_name(&result, "size");
        assert_eq!((size.start_line, size.end_line), (22, 23));
        assert_eq!(result.symbols.iter().filter(|s| s.name == "size").count(), 1);
        let point = symbol_by_name(&result, "Point");
        assert_eq!((point.start_line, point.end_line), (27, 29));

        let limit = symbol_by_name(&result, "CONSTANT_LIMIT");
        assert_eq!(limit.kind, SymbolKind::Constant);

        assert_eq!(result.imports.len(), 5);
        assert!(result.imports.iter().any(|i| i.names == vec!["os"]));
        assert!(result.imports.iter().any(|i| i.names == vec!["system"]));
        assert!(result.imports.iter().any(|i| i.source.as_deref() == Some("pathlib")));
        assert!(
            result
                .imports
                .iter()
                .any(|i| i.source.as_deref() == Some(".helpers") && i.names == vec!["go"])
        );
    }

    #[test]
    fn python_broken_file_recovers_valid_symbols() {
        let result = parse_fixture("broken.py");
        assert!(result.has_syntax_errors);
        assert!(!result.diagnostics.is_empty());
        assert!(result.symbols.iter().any(|s| s.name == "half"));
        assert!(result.symbols.iter().any(|s| s.name == "after"));
        // Diagnostics carry structure only, never code text.
        for diag in &result.diagnostics {
            assert!(diag.start_line <= diag.end_line);
            assert!(!diag.node_kind.is_empty());
        }
        let debug = format!("{:?}", result.diagnostics);
        assert!(!debug.contains("broken(:"));
        let json = serde_json::to_string(&result).expect("serializable");
        assert!(!json.contains("broken(:"));
    }

    #[test]
    fn rust_fixture_symbols_visibility_and_hierarchy() {
        let result = parse_fixture("sample.rs");
        assert_eq!(result.language, Language::Rust);
        assert!(!result.has_syntax_errors, "{:?}", result.diagnostics);

        let limit = symbol_by_name(&result, "LIMIT");
        assert_eq!(limit.kind, SymbolKind::Constant);
        assert_eq!(limit.visibility, Some(Visibility::Public));
        let private = symbol_by_name(&result, "PRIVATE");
        assert_eq!(private.visibility, Some(Visibility::Private));

        assert_eq!(symbol_by_name(&result, "Alias").kind, SymbolKind::Type);
        assert_eq!(symbol_by_name(&result, "Config").kind, SymbolKind::Struct);
        assert_eq!(symbol_by_name(&result, "Mode").kind, SymbolKind::Enum);
        let store = symbol_by_name(&result, "Store");
        assert_eq!(store.kind, SymbolKind::Trait);
        let get = symbol_by_name(&result, "get");
        assert_eq!(get.kind, SymbolKind::Method);
        assert_eq!(get.parent.as_deref(), Some(store.id.as_str()));

        let new = symbol_by_name(&result, "new");
        assert_eq!(new.kind, SymbolKind::Method);
        assert_eq!(new.visibility, Some(Visibility::Public));
        let helper = symbol_by_name(&result, "helper");
        assert_eq!(helper.visibility, Some(Visibility::Private));
        // Method parents point at the `impl` block symbol.
        let parent_id = new.parent.clone().expect("impl parent");
        let parent = result.symbols.iter().find(|s| s.id == parent_id).expect("impl symbol");
        assert_eq!(parent.kind, SymbolKind::Impl);
        assert_eq!(parent.name, "Memory");
        assert_eq!(helper.parent.as_deref(), Some(parent_id.as_str()));

        let nested = symbol_by_name(&result, "nested");
        assert_eq!(nested.kind, SymbolKind::Module);
        assert_eq!(symbol_by_name(&result, "inner").parent.as_deref(), Some(nested.id.as_str()));

        assert!(result.imports.iter().any(|i| i.source.as_deref() == Some("crate::model")
            && i.names == vec!["Symbol", "Visibility"]));
        assert!(result.imports.iter().any(|i| i.names == vec!["Ser"]));
    }

    #[test]
    fn rust_partial_file_recovers_valid_symbols() {
        let result = parse_fixture("partial.rs");
        assert!(result.has_syntax_errors);
        assert!(result.symbols.iter().any(|s| s.name == "done"));
        assert!(result.symbols.iter().any(|s| s.name == "later"));
    }

    #[test]
    fn parsing_is_deterministic_with_stable_ids() {
        let content = "export function f(a: number): number {\n  return a;\n}\n";
        let first = parse_content("a.ts", content).expect("parses");
        let second = parse_content("a.ts", content).expect("parses");
        assert_eq!(first, second);
        let other = parse_content("b.ts", content).expect("parses");
        assert_ne!(first.symbols[0].id, other.symbols[0].id);
    }

    #[test]
    fn duplicate_names_in_distinct_scopes_stay_distinct() {
        let content =
            "export class A {\n  run(): void {}\n}\nexport class B {\n  run(): void {}\n}\n";
        let result = parse_content("a.ts", content).expect("parses");
        let runs: Vec<&Symbol> = result.symbols.iter().filter(|s| s.name == "run").collect();
        assert_eq!(runs.len(), 2);
        assert_ne!(runs[0].id, runs[1].id);
        assert_ne!(runs[0].parent, runs[1].parent);
    }

    #[test]
    fn comments_and_strings_are_not_symbols() {
        let content = "// function fake() {}\nconst s = \"class Fake {}\";\nfunction real() {}\n";
        let result = parse_content("a.js", content).expect("parses");
        // Only real declarations: the string-holding constant and the function.
        // Nothing named after the comment/string keywords.
        assert_eq!(result.symbols.len(), 2);
        assert!(result.symbols.iter().any(|s| s.name == "real"));
        assert!(result.symbols.iter().all(|s| s.name != "fake" && s.name != "Fake"));
    }

    #[test]
    fn empty_file_parses_cleanly() {
        for (path, expected) in
            [("a.ts", Language::TypeScript), ("a.py", Language::Python), ("a.rs", Language::Rust)]
        {
            let result = parse_content(path, "").expect("empty parses");
            assert_eq!(result.language, expected);
            assert!(result.symbols.is_empty());
            assert!(result.imports.is_empty());
            assert!(!result.has_syntax_errors);
        }
    }

    #[test]
    fn secret_like_import_source_is_redacted() {
        let content = "import x from \"postgres://app:FAKEs3cr3tP4ssw0rdTESTONLY@db/x\";\n";
        let result = parse_content("a.ts", content).expect("parses");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].source.as_deref(), Some(REDACTED_MODULE));
        let json = serde_json::to_string(&result).expect("serializable");
        assert!(!json.contains("FAKEs3cr3t"));
    }

    #[test]
    fn json_carries_no_raw_content() {
        let result = parse_fixture("sample.rs");
        let json = serde_json::to_string(&result).expect("serializable");
        assert!(json.contains("Memory"));
        assert!(!json.contains("map: HashMap"));
    }

    #[test]
    fn go_stays_unsupported() {
        assert!(!Language::Go.has_structural_parser());
        assert!(parse_content("a.go", "package main\n").is_err());
    }
    #[test]
    fn parse_symbols_adapter_matches_full_result() {
        let parser = parser_for(Language::Python).expect("python parser");
        let content = "def f():\n  pass\n";
        let symbols = parser.parse_symbols("a.py", content).expect("symbols");
        let full = parser.parse("a.py", content).expect("full");
        assert_eq!(symbols, full.symbols);
    }
}
