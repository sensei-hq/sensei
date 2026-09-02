use super::LanguageAdapter;
use super::common::{ir_module, ir_parsed_file};
use crate::ir::{
    ClassKind, IRBase, IRClass, IRConstant, IRFunction, IRImport, IRMethod, IRParsedFile,
};
use crate::types::{ParsedEdge, ParsedFile, ParsedImport, ParsedSymbol, SymbolKind};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;

pub struct TypeScriptAdapter;
pub struct JavaScriptAdapter;

impl LanguageAdapter for TypeScriptAdapter {
    fn language(&self) -> &str {
        "typescript"
    }
    fn parse_to_ir(&self, source: &str, file_path: &str) -> crate::ir::IRParsedFile {
        parse_to_ir(source, file_path)
    }
    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        parse_oxc(source, file_path)
    }
    fn fqn_output(&self, abs_path: &str, content: &str) -> Option<super::fqn::FqnFileOutput> {
        typescript_fqn::ts_file_context(abs_path)
            .map(|ctx| typescript_fqn::produce_fqns(content, &ctx))
    }
}

impl LanguageAdapter for JavaScriptAdapter {
    fn language(&self) -> &str {
        "javascript"
    }
    fn parse_to_ir(&self, source: &str, file_path: &str) -> crate::ir::IRParsedFile {
        parse_to_ir(source, file_path)
    }
    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        parse_oxc(source, file_path)
    }
    fn fqn_output(&self, abs_path: &str, content: &str) -> Option<super::fqn::FqnFileOutput> {
        typescript_fqn::ts_file_context(abs_path)
            .map(|ctx| typescript_fqn::produce_fqns(content, &ctx))
    }
}

fn parse_oxc(source: &str, file_path: &str) -> ParsedFile {
    let source_type = SourceType::from_path(file_path).unwrap_or_default();
    let lang_name = if source_type.is_typescript() { "typescript" } else { "javascript" };

    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, source_type).parse();

    if ret.panicked {
        return ParsedFile {
            file_path: file_path.into(),
            language: lang_name.into(),
            symbols: vec![],
            edges: vec![],
            imports: vec![],
        };
    }

    let lines: Vec<&str> = source.lines().collect();
    let program = &ret.program;
    let mut symbols = Vec::new();
    let mut imports = Vec::new();
    let mut edges = Vec::new();

    for stmt in &program.body {
        extract_statement(stmt, source, &lines, &mut symbols, &mut imports, &mut edges);
    }

    ParsedFile { file_path: file_path.into(), language: lang_name.into(), symbols, edges, imports }
}

fn line_col(source: &str, offset: u32) -> u32 {
    // Convert byte offset to 1-based line number
    source[..offset as usize].matches('\n').count() as u32 + 1
}

fn extract_statement(
    stmt: &Statement,
    source: &str,
    lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>,
    imports: &mut Vec<ParsedImport>,
    _edges: &mut Vec<ParsedEdge>,
) {
    match stmt {
        Statement::FunctionDeclaration(f) => {
            if let Some(id) = &f.id {
                let start = line_col(source, f.span.start);
                let end = line_col(source, f.span.end);
                symbols.push(make_sym(
                    id.name.to_string(),
                    SymbolKind::Function,
                    start,
                    end,
                    lines,
                    false,
                ));
            }
        }
        Statement::ClassDeclaration(c) => {
            if let Some(id) = &c.id {
                let start = line_col(source, c.span.start);
                let end = line_col(source, c.span.end);
                let class_name = id.name.to_string();
                symbols.push(make_sym(
                    class_name.clone(),
                    SymbolKind::Class,
                    start,
                    end,
                    lines,
                    false,
                ));
                extract_class_body(&c.body, source, lines, symbols, &class_name);
            }
        }
        Statement::VariableDeclaration(var) => {
            extract_var_decl(var, source, lines, symbols, false);
        }
        Statement::ExportNamedDeclaration(export) => {
            if let Some(decl) = &export.declaration {
                extract_exported_decl(decl, source, lines, symbols);
            }
        }
        Statement::ExportDefaultDeclaration(export) => match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                let name =
                    f.id.as_ref().map(|i| i.name.to_string()).unwrap_or_else(|| "default".into());
                let start = line_col(source, f.span.start);
                let end = line_col(source, f.span.end);
                symbols.push(make_sym(name, SymbolKind::Function, start, end, lines, true));
            }
            ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                let name =
                    c.id.as_ref().map(|i| i.name.to_string()).unwrap_or_else(|| "default".into());
                let start = line_col(source, c.span.start);
                let end = line_col(source, c.span.end);
                symbols.push(make_sym(name.clone(), SymbolKind::Class, start, end, lines, true));
                extract_class_body(&c.body, source, lines, symbols, &name);
            }
            ExportDefaultDeclarationKind::TSInterfaceDeclaration(iface) => {
                let start = line_col(source, iface.span.start);
                let end = line_col(source, iface.span.end);
                symbols.push(make_sym(
                    iface.id.name.to_string(),
                    SymbolKind::Interface,
                    start,
                    end,
                    lines,
                    true,
                ));
            }
            _ => {}
        },
        Statement::ImportDeclaration(import) => {
            let target = import.source.value.to_string();
            let names: Vec<String> = import
                .specifiers
                .as_ref()
                .map(|specs| {
                    specs
                        .iter()
                        .map(|s| match s {
                            ImportDeclarationSpecifier::ImportSpecifier(named) => {
                                named.local.name.to_string()
                            }
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(def) => {
                                def.local.name.to_string()
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns) => {
                                format!("* as {}", ns.local.name)
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            imports.push(ParsedImport { target_path: target, names });
        }
        Statement::TSInterfaceDeclaration(iface) => {
            let start = line_col(source, iface.span.start);
            let end = line_col(source, iface.span.end);
            symbols.push(make_sym(
                iface.id.name.to_string(),
                SymbolKind::Interface,
                start,
                end,
                lines,
                false,
            ));
        }
        Statement::TSTypeAliasDeclaration(alias) => {
            let start = line_col(source, alias.span.start);
            let end = line_col(source, alias.span.end);
            symbols.push(make_sym(
                alias.id.name.to_string(),
                SymbolKind::Type,
                start,
                end,
                lines,
                false,
            ));
        }
        Statement::TSEnumDeclaration(en) => {
            let start = line_col(source, en.span.start);
            let end = line_col(source, en.span.end);
            symbols.push(make_sym(
                en.id.name.to_string(),
                SymbolKind::Enum,
                start,
                end,
                lines,
                false,
            ));
        }
        _ => {}
    }
}

fn extract_exported_decl(
    decl: &Declaration,
    source: &str,
    lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>,
) {
    match decl {
        Declaration::FunctionDeclaration(f) => {
            if let Some(id) = &f.id {
                let start = line_col(source, f.span.start);
                let end = line_col(source, f.span.end);
                symbols.push(make_sym(
                    id.name.to_string(),
                    SymbolKind::Function,
                    start,
                    end,
                    lines,
                    true,
                ));
            }
        }
        Declaration::ClassDeclaration(c) => {
            if let Some(id) = &c.id {
                let start = line_col(source, c.span.start);
                let end = line_col(source, c.span.end);
                let class_name = id.name.to_string();
                symbols.push(make_sym(
                    class_name.clone(),
                    SymbolKind::Class,
                    start,
                    end,
                    lines,
                    true,
                ));
                extract_class_body(&c.body, source, lines, symbols, &class_name);
            }
        }
        Declaration::VariableDeclaration(var) => {
            extract_var_decl(var, source, lines, symbols, true);
        }
        Declaration::TSInterfaceDeclaration(iface) => {
            let start = line_col(source, iface.span.start);
            let end = line_col(source, iface.span.end);
            symbols.push(make_sym(
                iface.id.name.to_string(),
                SymbolKind::Interface,
                start,
                end,
                lines,
                true,
            ));
        }
        Declaration::TSTypeAliasDeclaration(alias) => {
            let start = line_col(source, alias.span.start);
            let end = line_col(source, alias.span.end);
            symbols.push(make_sym(
                alias.id.name.to_string(),
                SymbolKind::Type,
                start,
                end,
                lines,
                true,
            ));
        }
        Declaration::TSEnumDeclaration(en) => {
            let start = line_col(source, en.span.start);
            let end = line_col(source, en.span.end);
            symbols.push(make_sym(
                en.id.name.to_string(),
                SymbolKind::Enum,
                start,
                end,
                lines,
                true,
            ));
        }
        _ => {}
    }
}

fn extract_var_decl(
    var: &VariableDeclaration,
    source: &str,
    lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>,
    is_exported: bool,
) {
    for decl in &var.declarations {
        if let BindingPattern::BindingIdentifier(id) = &decl.id {
            let name = id.name.to_string();
            let kind = match &decl.init {
                Some(Expression::ArrowFunctionExpression(_))
                | Some(Expression::FunctionExpression(_)) => SymbolKind::Function,
                _ => SymbolKind::Const,
            };
            let start = line_col(source, decl.span.start);
            let end = line_col(source, decl.span.end);
            symbols.push(make_sym(name, kind, start, end, lines, is_exported));
        }
    }
}

fn extract_class_body(
    body: &ClassBody,
    source: &str,
    lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>,
    class_name: &str,
) {
    for element in &body.body {
        if let ClassElement::MethodDefinition(m) = element
            && let Some(name) = method_name(&m.key)
        {
            let start = line_col(source, m.span.start);
            let end = line_col(source, m.span.end);
            let mut sym = make_sym(name, SymbolKind::Method, start, end, lines, true);
            sym.parent = Some(class_name.to_string());
            symbols.push(sym);
        }
    }
}

fn method_name(key: &PropertyKey) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::PrivateIdentifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}

fn make_sym(
    name: String,
    kind: SymbolKind,
    line_start: u32,
    line_end: u32,
    lines: &[&str],
    is_exported: bool,
) -> ParsedSymbol {
    let signature = lines.get(line_start.saturating_sub(1) as usize).map(|l| l.trim().to_string());
    ParsedSymbol {
        name,
        kind,
        signature,
        docstring: None,
        line_start,
        line_end,
        is_exported,
        parent: None,
    }
}

/// Parse TypeScript/JavaScript source into IR.
/// Uses OXC parser internally, converts ParsedFile output to IR types.
pub fn parse_to_ir(source: &str, file_path: &str) -> IRParsedFile {
    let pf = parse_oxc(source, file_path);

    // Convert ParsedSymbol list → IR structures
    let mut functions = Vec::new();
    let mut classes: Vec<IRClass> = Vec::new();
    let mut imports = Vec::new();
    let mut constants = Vec::new();

    for sym in &pf.symbols {
        match sym.kind {
            SymbolKind::Function | SymbolKind::Hook | SymbolKind::Component => {
                functions.push(IRFunction {
                    base: IRBase {
                        name: sym.name.clone(),
                        line_start: sym.line_start,
                        line_end: sym.line_end,
                        docstring: sym.docstring.clone(),
                        is_exported: sym.is_exported,
                        node_type: Some(
                            match sym.kind {
                                SymbolKind::Hook => "hook",
                                SymbolKind::Component => "component",
                                _ => "function",
                            }
                            .into(),
                        ),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            }
            SymbolKind::Class | SymbolKind::Interface | SymbolKind::Type => {
                let kind = match sym.kind {
                    SymbolKind::Interface => ClassKind::Interface,
                    SymbolKind::Type => ClassKind::Class,
                    _ => ClassKind::Class,
                };
                classes.push(IRClass {
                    base: IRBase {
                        name: sym.name.clone(),
                        line_start: sym.line_start,
                        line_end: sym.line_end,
                        docstring: sym.docstring.clone(),
                        is_exported: sym.is_exported,
                        node_type: Some("class".into()),
                        ..Default::default()
                    },
                    class_kind: kind,
                    ..Default::default()
                });
            }
            SymbolKind::Method => {
                // Attach to last class
                if let Some(class) = classes.last_mut() {
                    class.methods.push(IRMethod {
                        base: IRBase {
                            name: sym.name.clone(),
                            line_start: sym.line_start,
                            line_end: sym.line_end,
                            is_exported: sym.is_exported,
                            node_type: Some("method".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                }
            }
            SymbolKind::Const | SymbolKind::Enum => {
                constants.push(IRConstant {
                    base: IRBase {
                        name: sym.name.clone(),
                        line_start: sym.line_start,
                        line_end: sym.line_end,
                        is_exported: sym.is_exported,
                        node_type: Some("const".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            }
            _ => {}
        }
    }

    for imp in &pf.imports {
        imports.push(IRImport {
            source: imp.target_path.clone(),
            names: imp.names.clone(),
            is_reexport: false,
        });
    }

    let module = ir_module(
        file_path,
        &pf.language,
        functions,
        constants,
        imports,
        file_path.contains("test") || file_path.contains("spec"),
    );
    ir_parsed_file(file_path, &pf.language, module, classes)
}

// ── FQN producer (plan Phase 6.1) ────────────────────────────────────────────
// TypeScript/JS on the oxc AST. Package = nearest package.json `name`; module = the
// file path relative to the package (src/ stripped). One `typescript` namespace for
// TS + JS so `.ts`↔`.js` interop merges. Refs are extracted with the oxc Visit trait
// (the bare parse path had none): a relative import resolves to the sibling module's
// fqn, a bare-package import → a lib node, `this.m()`/`new T(); v.m()` → the class.
pub(crate) mod typescript_fqn {
    use super::super::fqn::{self, FileFqnContext, FqnDefinition, FqnFileOutput, FqnReference};
    use crate::types::SymbolKind;
    use oxc_allocator::Allocator;
    use oxc_ast::ast::*;
    use oxc_ast_visit::{Visit, walk};
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::collections::{HashMap, HashSet};

    const TS_LANG: &str = "typescript";
    const TS_CALL_DENYLIST: &[&str] = &[
        "map", "filter", "forEach", "reduce", "then", "catch", "push", "pop", "get", "set", "has",
        "add", "delete", "toString", "log", "warn", "error", "includes", "find", "some", "every",
        "slice", "split", "join", "trim", "replace", "test", "call", "apply", "bind", "keys",
        "values", "entries", "concat", "at",
    ];

    #[derive(Clone)]
    struct ImportTarget {
        external: bool,
        module_or_pkg: String,
        spec: String,
    }

    fn line_of(source: &str, offset: u32) -> u32 {
        source.get(..offset as usize).map(|s| s.matches('\n').count() as u32 + 1).unwrap_or(1)
    }
    fn sig(source: &str, offset: u32) -> Option<String> {
        let line = line_of(source, offset) as usize;
        source.lines().nth(line.saturating_sub(1)).map(|l| l.trim().to_string())
    }

    pub fn produce_fqns(source: &str, ctx: &FileFqnContext) -> FqnFileOutput {
        let allocator = Allocator::default();
        let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
        if ret.panicked {
            return FqnFileOutput::default();
        }
        let program = &ret.program;

        let mut imports: HashMap<String, ImportTarget> = HashMap::new();
        for stmt in &program.body {
            if let Statement::ImportDeclaration(imp) = stmt {
                let target = classify_import(&imp.source.value, &ctx.module);
                if let Some(specs) = &imp.specifiers {
                    for s in specs {
                        let local = match s {
                            ImportDeclarationSpecifier::ImportSpecifier(n) => {
                                n.local.name.to_string()
                            }
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(d) => {
                                d.local.name.to_string()
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns) => {
                                ns.local.name.to_string()
                            }
                        };
                        imports.insert(local, target.clone());
                    }
                }
            }
        }

        let mut out = FqnFileOutput {
            package: ctx.package.clone(),
            module: ctx.module.clone(),
            ..Default::default()
        };
        for stmt in &program.body {
            walk_stmt(stmt, source, ctx, &imports, &mut out);
        }
        out
    }

    /// ECMAScript language built-ins — available with no import in every JS/TS
    /// runtime, so a call to one is a call into the LANGUAGE, not into the calling
    /// module. Attributing them to `ctx.module` minted a separate fabricated node
    /// per caller: live, 1,330 `String` references across 1,321 distinct FQNs.
    ///
    /// Kept deliberately narrow — only names that are unambiguously built-in. A
    /// name absent from these lists falls through to the existing behaviour rather
    /// than being guessed at from the other direction.
    const ECMASCRIPT_GLOBALS: &[&str] = &[
        "Array",
        "ArrayBuffer",
        "BigInt",
        "Boolean",
        "DataView",
        "Date",
        "Error",
        "EvalError",
        "Float32Array",
        "Float64Array",
        "Function",
        "Infinity",
        "Int8Array",
        "Int16Array",
        "Int32Array",
        "Intl",
        "JSON",
        "Map",
        "Math",
        "NaN",
        "Number",
        "Object",
        "Promise",
        "Proxy",
        "RangeError",
        "ReferenceError",
        "Reflect",
        "RegExp",
        "Set",
        "String",
        "Symbol",
        "SyntaxError",
        "TypeError",
        "URIError",
        "Uint8Array",
        "Uint16Array",
        "Uint32Array",
        "WeakMap",
        "WeakSet",
        "decodeURI",
        "decodeURIComponent",
        "encodeURI",
        "encodeURIComponent",
        "globalThis",
        "isFinite",
        "isNaN",
        "parseFloat",
        "parseInt",
        "undefined",
    ];

    /// Web/Node platform globals. Named apart from the language built-ins because
    /// they are a different population — a project can run without them (a pure
    /// library never calls `fetch`), and lumping the two would make that
    /// undetectable.
    const PLATFORM_GLOBALS: &[&str] = &[
        "AbortController",
        "Blob",
        "Buffer",
        "FormData",
        "Headers",
        "Request",
        "Response",
        "TextDecoder",
        "TextEncoder",
        "URL",
        "URLSearchParams",
        "WebSocket",
        "atob",
        "btoa",
        "clearInterval",
        "clearTimeout",
        "console",
        "crypto",
        "document",
        "fetch",
        "localStorage",
        "navigator",
        "process",
        "queueMicrotask",
        "sessionStorage",
        "setInterval",
        "setTimeout",
        "structuredClone",
        "window",
    ];

    /// The runtime a bare global belongs to, if any.
    fn global_runtime(name: &str) -> Option<&'static str> {
        if ECMASCRIPT_GLOBALS.contains(&name) {
            Some("ecmascript")
        } else if PLATFORM_GLOBALS.contains(&name) {
            Some("webapi")
        } else {
            None
        }
    }

    fn classify_import(spec: &str, current_module: &str) -> ImportTarget {
        if spec.starts_with('.') {
            ImportTarget {
                external: false,
                module_or_pkg: resolve_relative(current_module, spec),
                spec: spec.to_string(),
            }
        } else {
            let pkg = if let Some(rest) = spec.strip_prefix('@') {
                let mut it = rest.splitn(3, '/');
                match (it.next(), it.next()) {
                    (Some(a), Some(b)) => format!("@{a}/{b}"),
                    _ => spec.to_string(),
                }
            } else {
                spec.split('/').next().unwrap_or(spec).to_string()
            };
            ImportTarget { external: true, module_or_pkg: pkg, spec: spec.to_string() }
        }
    }

    // `resolve_relative` and `strip_ext` MOVED to `languages::import_target`,
    // which owns specifier arithmetic. The import resolver in `process.rs` needs
    // the identical arithmetic to build its lookup candidates, and a second copy
    // here would drift from it — the failure mode named at the top of that module.
    use crate::languages::import_target::{resolve_relative, strip_ext};

    fn walk_stmt(
        stmt: &Statement,
        source: &str,
        ctx: &FileFqnContext,
        imports: &HashMap<String, ImportTarget>,
        out: &mut FqnFileOutput,
    ) {
        match stmt {
            Statement::FunctionDeclaration(f) => emit_function(f, false, ctx, imports, source, out),
            Statement::ClassDeclaration(c) => emit_class(c, false, ctx, imports, source, out),
            Statement::VariableDeclaration(v) => emit_var(v, false, ctx, imports, source, out),
            Statement::ExportNamedDeclaration(e) => {
                if let Some(decl) = &e.declaration {
                    emit_decl(decl, ctx, imports, source, out);
                }
            }
            Statement::ExportDefaultDeclaration(e) => match &e.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                    emit_function(f, true, ctx, imports, source, out)
                }
                ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                    emit_class(c, true, ctx, imports, source, out)
                }
                _ => {}
            },
            Statement::TSInterfaceDeclaration(i) => type_def(
                &i.id.name,
                i.span.start,
                i.span.end,
                SymbolKind::Interface,
                false,
                ctx,
                source,
                out,
            ),
            Statement::TSTypeAliasDeclaration(a) => type_def(
                &a.id.name,
                a.span.start,
                a.span.end,
                SymbolKind::Type,
                false,
                ctx,
                source,
                out,
            ),
            Statement::TSEnumDeclaration(en) => type_def(
                &en.id.name,
                en.span.start,
                en.span.end,
                SymbolKind::Enum,
                false,
                ctx,
                source,
                out,
            ),
            _ => {}
        }
    }

    fn emit_decl(
        decl: &Declaration,
        ctx: &FileFqnContext,
        imports: &HashMap<String, ImportTarget>,
        source: &str,
        out: &mut FqnFileOutput,
    ) {
        match decl {
            Declaration::FunctionDeclaration(f) => {
                emit_function(f, true, ctx, imports, source, out)
            }
            Declaration::ClassDeclaration(c) => emit_class(c, true, ctx, imports, source, out),
            Declaration::VariableDeclaration(v) => emit_var(v, true, ctx, imports, source, out),
            Declaration::TSInterfaceDeclaration(i) => type_def(
                &i.id.name,
                i.span.start,
                i.span.end,
                SymbolKind::Interface,
                true,
                ctx,
                source,
                out,
            ),
            Declaration::TSTypeAliasDeclaration(a) => type_def(
                &a.id.name,
                a.span.start,
                a.span.end,
                SymbolKind::Type,
                true,
                ctx,
                source,
                out,
            ),
            Declaration::TSEnumDeclaration(en) => type_def(
                &en.id.name,
                en.span.start,
                en.span.end,
                SymbolKind::Enum,
                true,
                ctx,
                source,
                out,
            ),
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn type_def(
        name: &str,
        start: u32,
        end: u32,
        kind: SymbolKind,
        exported: bool,
        ctx: &FileFqnContext,
        source: &str,
        out: &mut FqnFileOutput,
    ) {
        out.defs.push(FqnDefinition {
            fqn: fqn::item(TS_LANG, &ctx.package, &ctx.module, name),
            name: name.to_string(),
            kind,
            line_start: line_of(source, start),
            line_end: line_of(source, end),
            is_exported: exported,
            signature: sig(source, start),
            docstring: None,
            parent_type: None,
            parent_fqn: None,
        });
    }

    fn emit_function(
        f: &Function,
        exported: bool,
        ctx: &FileFqnContext,
        imports: &HashMap<String, ImportTarget>,
        source: &str,
        out: &mut FqnFileOutput,
    ) {
        let Some(id) = &f.id else { return };
        let name = id.name.to_string();
        let fqn_str = fqn::item(TS_LANG, &ctx.package, &ctx.module, &name);
        out.defs.push(FqnDefinition {
            fqn: fqn_str.clone(),
            name,
            kind: SymbolKind::Function,
            line_start: line_of(source, f.span.start),
            line_end: line_of(source, f.span.end),
            is_exported: exported,
            signature: sig(source, f.span.start),
            docstring: None,
            parent_type: None,
            parent_fqn: None,
        });
        if let Some(body) = &f.body {
            scan_body(body, &fqn_str, None, ctx, imports, source, out);
        }
    }

    fn emit_class(
        c: &Class,
        exported: bool,
        ctx: &FileFqnContext,
        imports: &HashMap<String, ImportTarget>,
        source: &str,
        out: &mut FqnFileOutput,
    ) {
        let Some(id) = &c.id else { return };
        let class_name = id.name.to_string();
        let class_fqn = fqn::item(TS_LANG, &ctx.package, &ctx.module, &class_name);
        out.defs.push(FqnDefinition {
            fqn: class_fqn.clone(),
            name: class_name.clone(),
            kind: SymbolKind::Class,
            line_start: line_of(source, c.span.start),
            line_end: line_of(source, c.span.end),
            is_exported: exported,
            signature: sig(source, c.span.start),
            docstring: None,
            parent_type: None,
            parent_fqn: None,
        });
        for element in &c.body.body {
            if let ClassElement::MethodDefinition(m) = element
                && let Some(name) = method_name(&m.key)
            {
                let mfqn = fqn::method(TS_LANG, &ctx.package, &ctx.module, &class_name, &name);
                out.defs.push(FqnDefinition {
                    fqn: mfqn.clone(),
                    name,
                    kind: SymbolKind::Method,
                    line_start: line_of(source, m.span.start),
                    line_end: line_of(source, m.span.end),
                    is_exported: true,
                    signature: sig(source, m.span.start),
                    docstring: None,
                    parent_type: Some(class_name.clone()),
                    parent_fqn: Some(class_fqn.clone()),
                });
                if let Some(body) = &m.value.body {
                    scan_body(body, &mfqn, Some(&class_name), ctx, imports, source, out);
                }
            }
        }
    }

    /// `const foo = () => {…}` / `const foo = function(){}` → a function def (arrow
    /// bodies are scanned for calls); other consts are ignored by the FQN producer.
    fn emit_var(
        v: &VariableDeclaration,
        exported: bool,
        ctx: &FileFqnContext,
        imports: &HashMap<String, ImportTarget>,
        source: &str,
        out: &mut FqnFileOutput,
    ) {
        for d in &v.declarations {
            let BindingPattern::BindingIdentifier(id) = &d.id else { continue };
            let name = id.name.to_string();
            match &d.init {
                Some(Expression::ArrowFunctionExpression(arrow)) => {
                    let fqn_str = fqn::item(TS_LANG, &ctx.package, &ctx.module, &name);
                    out.defs.push(FqnDefinition {
                        fqn: fqn_str.clone(),
                        name,
                        kind: SymbolKind::Function,
                        line_start: line_of(source, d.span.start),
                        line_end: line_of(source, d.span.end),
                        is_exported: exported,
                        signature: sig(source, d.span.start),
                        docstring: None,
                        parent_type: None,
                        parent_fqn: None,
                    });
                    scan_body(&arrow.body, &fqn_str, None, ctx, imports, source, out);
                }
                Some(Expression::FunctionExpression(f)) => {
                    let fqn_str = fqn::item(TS_LANG, &ctx.package, &ctx.module, &name);
                    out.defs.push(FqnDefinition {
                        fqn: fqn_str.clone(),
                        name,
                        kind: SymbolKind::Function,
                        line_start: line_of(source, d.span.start),
                        line_end: line_of(source, d.span.end),
                        is_exported: exported,
                        signature: sig(source, d.span.start),
                        docstring: None,
                        parent_type: None,
                        parent_fqn: None,
                    });
                    if let Some(body) = &f.body {
                        scan_body(body, &fqn_str, None, ctx, imports, source, out);
                    }
                }
                _ => {}
            }
        }
    }

    fn method_name(key: &PropertyKey) -> Option<String> {
        match key {
            PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
            PropertyKey::PrivateIdentifier(id) => Some(id.name.to_string()),
            _ => None,
        }
    }

    /// Collect calls in a function body, attributed to `caller_fqn`.
    fn scan_body(
        body: &FunctionBody,
        caller_fqn: &str,
        class: Option<&str>,
        ctx: &FileFqnContext,
        imports: &HashMap<String, ImportTarget>,
        source: &str,
        out: &mut FqnFileOutput,
    ) {
        let mut bindings = HashMap::new();
        for stmt in &body.statements {
            collect_binding(stmt, &mut bindings);
        }
        let mut v = CallVisitor {
            ctx,
            imports,
            class,
            bindings,
            caller_fqn,
            source,
            seen: HashSet::new(),
            refs: Vec::new(),
        };
        for stmt in &body.statements {
            v.visit_statement(stmt);
        }
        out.refs.append(&mut v.refs);
    }

    /// Bounded binding→type (plan 0.7): `const x = new Type()`.
    fn collect_binding(stmt: &Statement, map: &mut HashMap<String, String>) {
        if let Statement::VariableDeclaration(var) = stmt {
            for d in &var.declarations {
                if let BindingPattern::BindingIdentifier(id) = &d.id
                    && let Some(Expression::NewExpression(n)) = &d.init
                    && let Expression::Identifier(t) = &n.callee
                {
                    map.insert(id.name.to_string(), t.name.to_string());
                }
            }
        }
    }

    struct CallVisitor<'v> {
        ctx: &'v FileFqnContext,
        imports: &'v HashMap<String, ImportTarget>,
        class: Option<&'v str>,
        bindings: HashMap<String, String>,
        caller_fqn: &'v str,
        source: &'v str,
        seen: HashSet<String>,
        refs: Vec<FqnReference>,
    }

    impl<'a> Visit<'a> for CallVisitor<'_> {
        fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
            if let Some((target_fqn, is_lib, target_name)) = self.resolve_callee(&call.callee) {
                let key = target_fqn.clone().unwrap_or_else(|| format!("?{target_name}"));
                if self.seen.insert(key) {
                    self.refs.push(FqnReference {
                        caller_fqn: self.caller_fqn.to_string(),
                        caller_line: line_of(self.source, call.span.start),
                        target_fqn,
                        target_name,
                        is_lib,
                    });
                }
            }
            walk::walk_call_expression(self, call);
        }
    }

    impl CallVisitor<'_> {
        fn resolve_callee(&self, callee: &Expression) -> Option<(Option<String>, bool, String)> {
            match callee {
                Expression::Identifier(id) => {
                    let name = id.name.to_string();
                    if TS_CALL_DENYLIST.contains(&name.as_str()) {
                        return None;
                    }
                    Some(self.resolve_name(&name))
                }
                Expression::StaticMemberExpression(m) => {
                    let method = m.property.name.to_string();
                    if TS_CALL_DENYLIST.contains(&method.as_str()) {
                        return None;
                    }
                    match &m.object {
                        Expression::ThisExpression(_) => match self.class {
                            Some(cls) => Some((
                                Some(fqn::method(
                                    TS_LANG,
                                    &self.ctx.package,
                                    &self.ctx.module,
                                    cls,
                                    &method,
                                )),
                                false,
                                method,
                            )),
                            None => Some((None, false, method)),
                        },
                        Expression::Identifier(oid) => {
                            let oname = oid.name.as_str();
                            if let Some(t) = self.imports.get(oname) {
                                return Some(self.resolve_member(t, &method));
                            }
                            if let Some(ty) = self.bindings.get(oname) {
                                return Some((
                                    Some(fqn::method(
                                        TS_LANG,
                                        &self.ctx.package,
                                        &self.ctx.module,
                                        ty,
                                        &method,
                                    )),
                                    false,
                                    method,
                                ));
                            }
                            Some((None, false, method))
                        }
                        _ => Some((None, false, method)),
                    }
                }
                _ => None,
            }
        }
        fn resolve_name(&self, name: &str) -> (Option<String>, bool, String) {
            match self.imports.get(name) {
                Some(t) if t.external => {
                    (Some(fqn::lib(&t.module_or_pkg, &t.spec, name)), true, name.to_string())
                }
                Some(t) => (
                    Some(fqn::item(TS_LANG, &self.ctx.package, &t.module_or_pkg, name)),
                    false,
                    name.to_string(),
                ),
                // A runtime global needs no import, so its absence from the map is
                // not evidence that it lives here. Naming the runtime also merges
                // every caller's reference onto ONE node instead of minting a
                // fabricated module-local one each time.
                None => match global_runtime(name) {
                    Some(runtime) => {
                        (Some(fqn::lib(runtime, "globalThis", name)), true, name.to_string())
                    }
                    None => (
                        Some(fqn::item(TS_LANG, &self.ctx.package, &self.ctx.module, name)),
                        false,
                        name.to_string(),
                    ),
                },
            }
        }
        fn resolve_member(&self, t: &ImportTarget, method: &str) -> (Option<String>, bool, String) {
            if t.external {
                (Some(fqn::lib(&t.module_or_pkg, &t.spec, method)), true, method.to_string())
            } else {
                (
                    Some(fqn::item(TS_LANG, &self.ctx.package, &t.module_or_pkg, method)),
                    false,
                    method.to_string(),
                )
            }
        }
    }

    /// Resolve a TS/JS file's FQN context: nearest package.json `name` + the file's
    /// package-relative module path (extension dropped, a leading `src/` stripped).
    pub(crate) fn ts_file_context(abs_path: &str) -> Option<FileFqnContext> {
        let file = std::path::Path::new(abs_path);
        let mut dir = file.parent();
        while let Some(d) = dir {
            let manifest = d.join("package.json");
            if manifest.is_file()
                && let Some(package) = package_json_name(&manifest)
            {
                return Some(FileFqnContext { package, module: ts_module_path(file, d) });
            }
            dir = d.parent();
        }
        None
    }
    fn package_json_name(manifest: &std::path::Path) -> Option<String> {
        let text = std::fs::read_to_string(manifest).ok()?;
        let val: serde_json::Value = serde_json::from_str(&text).ok()?;
        val.get("name")?.as_str().map(str::to_string)
    }
    fn ts_module_path(file: &std::path::Path, pkg_root: &std::path::Path) -> String {
        let rel = file.strip_prefix(pkg_root).unwrap_or(file);
        let mut comps: Vec<String> =
            rel.components().filter_map(|c| c.as_os_str().to_str().map(str::to_string)).collect();
        if let Some(last) = comps.last_mut() {
            *last = strip_ext(last).to_string();
        }
        // Strip a leading `src/` so `src/lib/util` → `lib/util`.
        if comps.first().map(String::as_str) == Some("src") {
            comps.remove(0);
        }
        comps.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ts_src(src: &str) -> ParsedFile {
        TypeScriptAdapter.parse(src, "test.ts")
    }
    fn parse_js_src(src: &str) -> ParsedFile {
        JavaScriptAdapter.parse(src, "test.js")
    }

    // ── FQN producer (Phase 6.1) ────────────────────────────────────────────
    use crate::languages::fqn::{FileFqnContext, FqnFileOutput, FqnReference};
    fn produce_ts(src: &str, package: &str, module: &str) -> FqnFileOutput {
        typescript_fqn::produce_fqns(
            src,
            &FileFqnContext { package: package.into(), module: module.into() },
        )
    }
    fn def_fqn<'a>(out: &'a FqnFileOutput, name: &str) -> &'a str {
        out.defs.iter().find(|d| d.name == name).map(|d| d.fqn.as_str()).unwrap_or("<no-def>")
    }
    fn ref_to<'a>(out: &'a FqnFileOutput, target_name: &str) -> &'a FqnReference {
        out.refs
            .iter()
            .find(|r| r.target_name == target_name)
            .unwrap_or_else(|| panic!("no ref to `{target_name}` in {:?}", out.refs))
    }

    #[test]
    fn ts_def_fqn() {
        let out = produce_ts(
            "export function top() {}\nexport class Widget {\n  spin() {}\n}\n",
            "app",
            "lib/util",
        );
        assert_eq!(
            def_fqn(&out, "top"),
            "typescript·app·lib/util·top",
            "module = package-relative path"
        );
        assert_eq!(def_fqn(&out, "Widget"), "typescript·app·lib/util·Widget");
        assert_eq!(
            def_fqn(&out, "spin"),
            "typescript·app·lib/util·Widget·spin",
            "method nests on its class"
        );
    }

    /// A call to a runtime global is not a call into the calling module.
    ///
    /// The unresolved arm attributed every unknown name to `ctx.module`, so each
    /// caller minted its OWN `String`/`fetch`/`setTimeout` node: live, 1,330
    /// `String` stubs across 1,321 distinct FQNs, 826 `fetch` across 818, 652
    /// `Number` across 617. Naming the runtime collapses each to one node.
    #[test]
    fn ts_runtime_globals_resolve_to_the_runtime_not_the_caller() {
        let out = produce_ts(
            "export function f(x: unknown) {\n  String(x);\n  Number(x);\n  parseInt('1');\n}\n",
            "app",
            "lib/convert",
        );
        for (name, want) in [
            ("String", "lib·ecmascript·globalThis·String"),
            ("Number", "lib·ecmascript·globalThis·Number"),
            ("parseInt", "lib·ecmascript·globalThis·parseInt"),
        ] {
            let r = ref_to(&out, name);
            assert_eq!(r.target_fqn.as_deref(), Some(want), "`{name}` belongs to the runtime");
            assert!(r.is_lib, "`{name}` is not this project's code");
        }
    }

    /// Platform globals are named for the platform, not for ECMAScript — `fetch`
    /// and `setTimeout` are Web/Node APIs, not language built-ins, and saying so
    /// keeps the two populations countable apart.
    #[test]
    fn ts_platform_globals_are_named_as_platform_not_language() {
        let out = produce_ts(
            "export async function f() {\n  await fetch('/x');\n  setTimeout(() => {}, 1);\n}\n",
            "app",
            "lib/net",
        );
        assert_eq!(
            ref_to(&out, "fetch").target_fqn.as_deref(),
            Some("lib·webapi·globalThis·fetch")
        );
        assert_eq!(
            ref_to(&out, "setTimeout").target_fqn.as_deref(),
            Some("lib·webapi·globalThis·setTimeout")
        );
    }

    /// An imported name still wins over the globals list — a module that exports
    /// its own `fetch` is that module's, and the import states it.
    #[test]
    fn ts_an_import_outranks_the_globals_list() {
        let out = produce_ts(
            "import { fetch } from './http';\nexport function f() { fetch('/x'); }\n",
            "app",
            "lib/net",
        );
        assert_eq!(
            ref_to(&out, "fetch").target_fqn.as_deref(),
            Some("typescript·app·lib/http·fetch"),
            "the import is evidence; the globals list is only a fallback"
        );
    }

    #[test]
    fn ts_ref_fqn_import() {
        let out = produce_ts(
            "import { helper } from './util';\nexport function build() { helper(); }\n",
            "app",
            "lib/builder",
        );
        let r = ref_to(&out, "helper");
        assert_eq!(
            r.target_fqn.as_deref(),
            Some("typescript·app·lib/util·helper"),
            "relative import resolves to the sibling module"
        );
        assert!(!r.is_lib);
        assert_eq!(r.caller_fqn, "typescript·app·lib/builder·build");
    }

    #[test]
    fn ts_method_scope() {
        let src = "class Engine {\n  run() {\n    this.tick();\n    const g = new Gadget();\n    g.spin();\n  }\n  tick() {}\n}\n";
        let out = produce_ts(src, "app", "engine");
        assert_eq!(
            ref_to(&out, "tick").target_fqn.as_deref(),
            Some("typescript·app·engine·Engine·tick"),
            "this.m → enclosing class"
        );
        assert_eq!(
            ref_to(&out, "spin").target_fqn.as_deref(),
            Some("typescript·app·engine·Gadget·spin"),
            "const x = new T(); x.m() → T.m (0.7 binding)"
        );
    }

    #[test]
    fn ts_external_is_lib() {
        let out = produce_ts(
            "import { readFile } from 'fs';\nexport function load() { readFile('/x'); }\n",
            "app",
            "io",
        );
        let r = ref_to(&out, "readFile");
        assert_eq!(
            r.target_fqn.as_deref(),
            Some("lib·fs·fs·readFile"),
            "bare-package import → lib node"
        );
        assert!(r.is_lib);
    }

    #[test]
    fn ts_function() {
        let pf = parse_ts_src("function hello(name: string): string { return name; }");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "hello");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn ts_class_with_methods() {
        let pf = parse_ts_src("class Foo {\n  bar() {}\n  baz() {}\n}");
        let names: Vec<&str> = pf.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Foo"));
        assert!(names.contains(&"bar"));
        assert!(names.contains(&"baz"));
        assert_eq!(pf.symbols[0].kind, SymbolKind::Class);
    }

    #[test]
    fn ts_interface_and_type() {
        let pf = parse_ts_src("interface Foo { x: number }\ntype Bar = string;");
        assert_eq!(pf.symbols.len(), 2);
        assert_eq!(pf.symbols[0].kind, SymbolKind::Interface);
        assert_eq!(pf.symbols[1].kind, SymbolKind::Type);
    }

    #[test]
    fn ts_enum() {
        let pf = parse_ts_src("enum Color { Red, Green }");
        assert_eq!(pf.symbols[0].name, "Color");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Enum);
    }

    #[test]
    fn ts_const_and_arrow() {
        let pf = parse_ts_src("const TIMEOUT = 30;\nconst greet = (name: string) => name;");
        assert_eq!(pf.symbols.len(), 2);
        assert_eq!(pf.symbols[0].kind, SymbolKind::Const);
        assert_eq!(pf.symbols[1].kind, SymbolKind::Function);
    }

    #[test]
    fn ts_exports() {
        let pf = parse_ts_src("export function hello() {}\nfunction internal() {}");
        assert_eq!(pf.symbols.len(), 2);
        assert!(pf.symbols[0].is_exported);
        assert!(!pf.symbols[1].is_exported);
    }

    #[test]
    fn ts_imports() {
        let pf = parse_ts_src("import { readFile } from 'fs';\nimport express from 'express';");
        assert_eq!(pf.imports.len(), 2);
        assert_eq!(pf.imports[0].target_path, "fs");
        assert_eq!(pf.imports[0].names, vec!["readFile"]);
        assert_eq!(pf.imports[1].target_path, "express");
        assert_eq!(pf.imports[1].names, vec!["express"]);
    }

    #[test]
    fn tsx_jsx() {
        let pf = TypeScriptAdapter
            .parse("export function App() { return <div>Hello</div>; }", "test.tsx");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "App");
    }

    #[test]
    fn js_function() {
        let pf = parse_js_src("function hello() { return 1; }");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
        assert_eq!(pf.language, "javascript");
    }

    #[test]
    fn js_jsx() {
        let pf = JavaScriptAdapter.parse("function App() { return <div/>; }", "test.jsx");
        assert_eq!(pf.symbols.len(), 1);
    }

    #[test]
    fn syntax_error_returns_empty() {
        let pf = parse_ts_src("function {{{ broken");
        assert!(pf.symbols.is_empty());
    }

    #[test]
    fn namespace_import() {
        let pf = parse_ts_src("import * as path from 'path';");
        assert_eq!(pf.imports.len(), 1);
        assert_eq!(pf.imports[0].target_path, "path");
        assert_eq!(pf.imports[0].names, vec!["* as path"]);
    }

    #[test]
    fn export_default_function() {
        let pf = parse_ts_src("export default function main() {}");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "main");
        assert!(pf.symbols[0].is_exported);
    }

    #[test]
    fn method_parent_set_on_class() {
        let pf = parse_ts_src("class Foo {\n  bar() {}\n  baz() {}\n}");
        let foo = pf.symbols.iter().find(|s| s.name == "Foo").unwrap();
        assert!(foo.parent.is_none(), "class itself should have no parent");
        let bar = pf.symbols.iter().find(|s| s.name == "bar").unwrap();
        assert_eq!(bar.parent.as_deref(), Some("Foo"));
        assert_eq!(bar.kind, SymbolKind::Method);
        let baz = pf.symbols.iter().find(|s| s.name == "baz").unwrap();
        assert_eq!(baz.parent.as_deref(), Some("Foo"));
    }

    #[test]
    fn method_parent_on_exported_class() {
        let pf = parse_ts_src("export class Service {\n  handle() {}\n}");
        let handle = pf.symbols.iter().find(|s| s.name == "handle").unwrap();
        assert_eq!(handle.parent.as_deref(), Some("Service"));
        assert!(handle.is_exported);
    }

    #[test]
    fn method_parent_on_default_export_class() {
        let pf = parse_ts_src("export default class Router {\n  route() {}\n}");
        let route = pf.symbols.iter().find(|s| s.name == "route").unwrap();
        assert_eq!(route.parent.as_deref(), Some("Router"));
    }

    #[test]
    fn standalone_function_has_no_parent() {
        let pf = parse_ts_src("function standalone() {}\nconst arrow = () => {};");
        for sym in &pf.symbols {
            assert!(sym.parent.is_none(), "{} should have no parent", sym.name);
        }
    }

    #[test]
    fn interface_and_enum_have_no_parent() {
        let pf = parse_ts_src("interface Foo { x: number }\nenum Color { Red }");
        for sym in &pf.symbols {
            assert!(sym.parent.is_none(), "{} should have no parent", sym.name);
        }
    }
}
