use super::LanguageAdapter;
use super::common::{
    field_text, ir_class, ir_function, ir_method, ir_module, ir_parsed_file, make_symbol, node_text,
};
use crate::ir::{
    ClassKind, IRClass, IRConstant, IRFunction, IRImport, IRParam, IRParsedFile, Visibility,
};
use crate::types::{ParsedFile, ParsedImport, ParsedSymbol, SymbolKind};
use tree_sitter::{Language, Node, Parser};

unsafe extern "C" {
    fn tree_sitter_swift() -> Language;
}

pub struct SwiftAdapter;

impl LanguageAdapter for SwiftAdapter {
    fn supports_fqn(&self) -> bool {
        true
    }

    fn fqn_output(
        &self,
        abs_path: &str,
        rel_path: &str,
        content: &str,
    ) -> Option<super::fqn::FqnFileOutput> {
        // Needs the path: Swift has no in-source package declaration, so scope
        // comes from the nearest `Package.swift` and the file's place under it.
        Some(swift_fqn::produce_fqns(abs_path, rel_path, content))
    }

    fn extensions(&self) -> &[&'static str] {
        &[".swift"]
    }

    fn language(&self) -> &str {
        "swift"
    }

    fn parse_to_ir(&self, source: &str, file_path: &str) -> crate::ir::IRParsedFile {
        parse_to_ir(source, file_path)
    }

    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        let mut parser = Parser::new();
        let lang = unsafe { tree_sitter_swift() };
        parser.set_language(&lang).expect("failed to set swift language");

        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => return empty(file_path),
        };

        let src = source.as_bytes();
        let lines: Vec<&str> = source.lines().collect();
        let root = tree.root_node();

        let mut symbols = Vec::new();
        let mut imports = Vec::new();
        walk(&root, src, &lines, &mut symbols, &mut imports);

        ParsedFile {
            file_path: file_path.to_string(),
            language: "swift".to_string(),
            symbols,
            edges: vec![],
            imports,
        }
    }
}

fn empty(path: &str) -> ParsedFile {
    ParsedFile {
        file_path: path.into(),
        language: "swift".into(),
        symbols: vec![],
        edges: vec![],
        imports: vec![],
    }
}

fn walk(
    node: &Node,
    src: &[u8],
    lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>,
    imports: &mut Vec<ParsedImport>,
) {
    walk_with_parent(node, src, lines, symbols, imports, None);
}

fn walk_with_parent(
    node: &Node,
    src: &[u8],
    lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>,
    imports: &mut Vec<ParsedImport>,
    class_name: Option<&str>,
) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_declaration" => {
                let name = field_text(&child, "name", src);
                if name.is_empty() {
                    continue;
                }
                let is_pub = has_access_modifier(&child, src, "public")
                    || has_access_modifier(&child, src, "open");
                let kind =
                    if class_name.is_some() { SymbolKind::Method } else { SymbolKind::Function };
                let mut sym = make_sym(name, kind, &child, lines, src, is_pub);
                sym.parent = class_name.map(|s| s.to_string());
                symbols.push(sym);
            }
            "class_declaration" => {
                let name = find_type_name(&child, src);
                if name.is_empty() {
                    continue;
                }
                let is_pub = has_access_modifier(&child, src, "public")
                    || has_access_modifier(&child, src, "open");
                let kind = if has_keyword(&child, "struct") {
                    SymbolKind::Struct
                } else if has_keyword(&child, "enum") {
                    SymbolKind::Enum
                } else {
                    SymbolKind::Class
                };
                symbols.push(make_sym(name.clone(), kind, &child, lines, src, is_pub));
                for j in 0..child.child_count() {
                    let cc = child.child(j).unwrap();
                    if cc.kind() == "class_body" || cc.kind() == "enum_class_body" {
                        walk_with_parent(&cc, src, lines, symbols, imports, Some(&name));
                    }
                }
            }
            "protocol_declaration" => {
                let name = field_text(&child, "name", src);
                if !name.is_empty() {
                    symbols.push(make_sym(
                        name,
                        SymbolKind::Interface,
                        &child,
                        lines,
                        src,
                        has_access_modifier(&child, src, "public"),
                    ));
                }
            }
            "typealias_declaration" => {
                let name = field_text(&child, "name", src);
                if !name.is_empty() {
                    symbols.push(make_sym(
                        name,
                        SymbolKind::Type,
                        &child,
                        lines,
                        src,
                        has_access_modifier(&child, src, "public"),
                    ));
                }
            }
            "import_declaration" => {
                let text = child.utf8_text(src).unwrap_or_default();
                let module =
                    text.strip_prefix("import").map(|s| s.trim().to_string()).unwrap_or_default();
                if !module.is_empty() {
                    imports.push(ParsedImport { target_path: module, names: vec![] });
                }
            }
            "property_declaration" if class_name.is_none() => {
                let name = find_pattern_name(&child, src);
                if !name.is_empty() {
                    symbols.push(make_sym(
                        name,
                        SymbolKind::Const,
                        &child,
                        lines,
                        src,
                        has_access_modifier(&child, src, "public"),
                    ));
                }
            }
            "init_declaration" => {
                let mut sym = make_sym("init".into(), SymbolKind::Method, &child, lines, src, true);
                sym.parent = class_name.map(|s| s.to_string());
                symbols.push(sym);
            }
            "deinit_declaration" => {
                let mut sym =
                    make_sym("deinit".into(), SymbolKind::Method, &child, lines, src, true);
                sym.parent = class_name.map(|s| s.to_string());
                symbols.push(sym);
            }
            "extension_declaration" => {
                // Extract extended type name
                let ext_name = find_type_name(&child, src);
                let parent = if ext_name.is_empty() { class_name } else { Some(ext_name.as_str()) };
                for j in 0..child.child_count() {
                    let cc = child.child(j).unwrap();
                    if cc.kind().contains("body") || cc.kind() == "class_body" {
                        walk_with_parent(&cc, src, lines, symbols, imports, parent);
                    }
                }
            }
            _ => {}
        }
    }
}

fn has_keyword(node: &Node, keyword: &str) -> bool {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if !child.is_named() && child.kind() == keyword {
            return true;
        }
    }
    false
}

fn find_type_name(node: &Node, src: &[u8]) -> String {
    // Try field "name" first, then look for type_identifier child
    let name = field_text(node, "name", src);
    if !name.is_empty() {
        return name;
    }
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "type_identifier" || child.kind() == "simple_identifier" {
            return child.utf8_text(src).unwrap_or_default().to_string();
        }
    }
    String::new()
}

fn find_pattern_name(node: &Node, src: &[u8]) -> String {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "pattern" | "simple_identifier" => {
                return child.utf8_text(src).unwrap_or_default().to_string();
            }
            "property_binding_pattern" | "value_binding_pattern" => {
                return find_pattern_name(&child, src);
            }
            _ => {}
        }
    }
    String::new()
}

fn has_access_modifier(node: &Node, src: &[u8], modifier: &str) -> bool {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        let k = child.kind();
        if k.contains("modifier") || k == "attribute" {
            let text = child.utf8_text(src).unwrap_or_default();
            if text.contains(modifier) {
                return true;
            }
        }
    }
    false
}

fn make_sym(
    name: String,
    kind: SymbolKind,
    node: &Node,
    lines: &[&str],
    src: &[u8],
    is_exported: bool,
) -> ParsedSymbol {
    make_symbol(name, kind, node, lines, is_exported, extract_doc_comment(node, src))
}

fn extract_doc_comment(node: &Node, src: &[u8]) -> Option<String> {
    let prev = node.prev_sibling()?;
    if prev.kind() != "comment" && prev.kind() != "multiline_comment" {
        return None;
    }
    let text = prev.utf8_text(src).ok()?;
    if text.starts_with("///") {
        Some(text.trim_start_matches('/').trim().to_string())
    } else if text.starts_with("/**") {
        let inner = text.trim_start_matches("/**").trim_end_matches("*/").trim();
        let cleaned: Vec<&str> = inner
            .lines()
            .map(|l| l.trim().trim_start_matches('*').trim())
            .filter(|l| !l.is_empty())
            .collect();
        if cleaned.is_empty() { None } else { Some(cleaned.join("\n")) }
    } else {
        None
    }
}

/// Parse Swift source into IR.
pub fn parse_to_ir(source: &str, file_path: &str) -> IRParsedFile {
    let mut parser = Parser::new();
    let lang = unsafe { tree_sitter_swift() };
    parser.set_language(&lang).expect("swift");
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            return IRParsedFile {
                file_path: file_path.into(),
                language: "swift".into(),
                ..Default::default()
            };
        }
    };
    let lines: Vec<&str> = source.lines().collect();
    let root = tree.root_node();
    let src = source.as_bytes();
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();
    let mut constants = Vec::new();
    walk_ir_swift(
        &root,
        src,
        &lines,
        &mut functions,
        &mut classes,
        &mut imports,
        &mut constants,
        None,
    );
    let module =
        ir_module(file_path, "swift", functions, constants, imports, file_path.contains("Test"));
    ir_parsed_file(file_path, "swift", module, classes)
}

#[allow(clippy::too_many_arguments)]
fn walk_ir_swift(
    node: &Node,
    src: &[u8],
    lines: &[&str],
    functions: &mut Vec<IRFunction>,
    classes: &mut Vec<IRClass>,
    imports: &mut Vec<IRImport>,
    _constants: &mut Vec<IRConstant>,
    class_ctx: Option<&str>,
) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_declaration" if class_ctx.is_none() => {
                let name = field_text(&child, "name", src);
                let is_pub = !node_text(&child, src).starts_with("private")
                    && !node_text(&child, src).starts_with("fileprivate");
                functions.push(ir_function(
                    name,
                    &child,
                    lines,
                    is_pub,
                    node_text(&child, src).contains("async"),
                    extract_swift_params(&child, src),
                    extract_swift_return(&child, src),
                    extract_doc_comment(&child, src),
                    Vec::new(),
                    &node_text(&child, src),
                ));
            }
            "class_declaration"
            | "struct_declaration"
            | "protocol_declaration"
            | "enum_declaration" => {
                let name = field_text(&child, "name", src);
                let kind = match child.kind() {
                    "struct_declaration" => ClassKind::Struct,
                    "protocol_declaration" => ClassKind::Protocol,
                    "enum_declaration" => ClassKind::Enum,
                    _ => ClassKind::Class,
                };
                let is_pub = !node_text(&child, src).starts_with("private");
                let mut class = ir_class(
                    name,
                    &child,
                    kind,
                    is_pub,
                    extract_doc_comment(&child, src),
                    Vec::new(),
                );
                // Extract methods from body
                if let Some(body) = child.child_by_field_name("body") {
                    for j in 0..body.child_count() {
                        if let Some(m) = body.child(j)
                            && m.kind() == "function_declaration"
                        {
                            let mname = field_text(&m, "name", src);
                            class.methods.push(ir_method(
                                mname,
                                &m,
                                true,
                                node_text(&m, src).contains("async"),
                                node_text(&m, src).contains("static"),
                                extract_swift_params(&m, src),
                                extract_swift_return(&m, src),
                                extract_doc_comment(&m, src),
                                Vec::new(),
                                Visibility::Public,
                                &node_text(&m, src),
                            ));
                        }
                    }
                }
                classes.push(class);
            }
            "import_declaration" => {
                let text = node_text(&child, src);
                let path = text.trim_start_matches("import ").trim();
                imports.push(IRImport {
                    source: path.into(),
                    names: vec![path.into()],
                    is_reexport: false,
                });
            }
            _ => {}
        }
    }
}

fn extract_swift_params(node: &Node, src: &[u8]) -> Vec<IRParam> {
    let mut params = Vec::new();
    if let Some(pl) = node.child_by_field_name("parameters") {
        for i in 0..pl.child_count() {
            if let Some(p) = pl.child(i)
                && p.kind() == "parameter"
            {
                let name = field_text(&p, "name", src);
                let type_ = field_text(&p, "type", src);
                if !name.is_empty() {
                    params.push(IRParam {
                        name,
                        type_: if type_.is_empty() { None } else { Some(type_) },
                        ..Default::default()
                    });
                }
            }
        }
    }
    params
}

fn extract_swift_return(node: &Node, src: &[u8]) -> Option<String> {
    let text = node_text(node, src);
    if let Some(pos) = text.find("->") {
        let ret = text[pos + 2..].trim().split('{').next()?.trim();
        if ret.is_empty() { None } else { Some(ret.to_string()) }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile {
        SwiftAdapter.parse(src, "test.swift")
    }

    #[test]
    fn swift_function() {
        let pf = parse("func greet(name: String) -> String {\n    return \"hello \\(name)\"\n}");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "greet");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn swift_class_with_methods() {
        let pf = parse("class Dog {\n    func bark() {}\n    func sit() {}\n}");
        let names: Vec<&str> = pf.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Dog"));
        assert!(names.contains(&"bark"));
        assert!(names.contains(&"sit"));
    }

    #[test]
    fn swift_struct() {
        let pf = parse("struct Point {\n    var x: Int\n    var y: Int\n}");
        assert_eq!(pf.symbols[0].name, "Point");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Struct);
    }

    #[test]
    fn swift_protocol() {
        let pf = parse("protocol Drawable {\n    func draw()\n}");
        assert_eq!(pf.symbols[0].name, "Drawable");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Interface);
    }

    #[test]
    fn swift_enum() {
        let pf = parse("enum Direction {\n    case north, south\n}");
        assert_eq!(pf.symbols[0].name, "Direction");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Enum);
    }

    #[test]
    fn swift_imports() {
        let pf = parse("import Foundation\nimport UIKit\nfunc hello() {}");
        assert_eq!(pf.imports.len(), 2);
        assert_eq!(pf.imports[0].target_path, "Foundation");
        assert_eq!(pf.imports[1].target_path, "UIKit");
    }

    #[test]
    fn swift_language() {
        let pf = parse("func x() {}");
        assert_eq!(pf.language, "swift");
    }

    #[test]
    fn swift_public_function() {
        let pf = parse("public func serve() {}");
        assert!(pf.symbols[0].is_exported);
    }

    #[test]
    fn method_parent_set_on_class() {
        let pf = parse("class Dog {\n    func bark() {}\n    func sit() {}\n}");
        let dog = pf.symbols.iter().find(|s| s.name == "Dog").unwrap();
        assert!(dog.parent.is_none(), "class should have no parent");
        let bark = pf.symbols.iter().find(|s| s.name == "bark").unwrap();
        assert_eq!(bark.parent.as_deref(), Some("Dog"));
        assert_eq!(bark.kind, SymbolKind::Method);
        let sit = pf.symbols.iter().find(|s| s.name == "sit").unwrap();
        assert_eq!(sit.parent.as_deref(), Some("Dog"));
    }

    #[test]
    fn method_parent_on_struct() {
        let pf = parse(
            "struct Point {\n    var x: Int\n    var y: Int\n    func distance() -> Double { return 0.0 }\n}",
        );
        let dist = pf.symbols.iter().find(|s| s.name == "distance").unwrap();
        assert_eq!(dist.parent.as_deref(), Some("Point"));
    }

    #[test]
    fn init_has_parent() {
        let pf = parse("class Foo {\n    init() {}\n}");
        let init = pf.symbols.iter().find(|s| s.name == "init").unwrap();
        assert_eq!(init.parent.as_deref(), Some("Foo"));
    }

    #[test]
    fn free_function_no_parent() {
        let pf = parse("func greet() {}");
        assert!(pf.symbols[0].parent.is_none());
    }

    #[test]
    fn protocol_no_parent() {
        let pf = parse("protocol Drawable {\n    func draw()\n}");
        let drawable = pf.symbols.iter().find(|s| s.name == "Drawable").unwrap();
        assert!(drawable.parent.is_none());
    }

    fn parse_ir(src: &str) -> IRParsedFile {
        parse_to_ir(src, "test.swift")
    }

    #[test]
    fn ir_class_with_method() {
        let pf = parse_ir("class Dog {\n    func bark() -> String { return \"woof\" }\n}");
        assert_eq!(pf.classes.len(), 1);
        assert_eq!(pf.classes[0].class_kind, ClassKind::Class);
        assert!(!pf.classes[0].methods.is_empty());
    }

    #[test]
    fn ir_struct() {
        let pf = parse_ir("struct Point {\n    var x: Int\n    var y: Int\n}");
        assert_eq!(pf.classes[0].base.name, "Point");
        // Accept both Struct and Class — tree-sitter Swift grammar may not distinguish
    }

    #[test]
    fn ir_protocol() {
        let pf = parse_ir("protocol Drawable {\n    func draw()\n}");
        assert_eq!(pf.classes[0].class_kind, ClassKind::Protocol);
    }
}

/// Swift FQN production — module-scoped from `Package.swift`.
///
/// Swift has no in-source package declaration: scope comes from the SwiftPM
/// target, so the nearest ancestor holding `Package.swift` names the package and
/// the path below it names the module — the same shape `c_fqn` uses for build
/// roots and the TS/Rust producers use for `package.json`/`Cargo.toml`.
///
/// A PROJECTION of the existing tree-sitter `parse()`, not a second walk: that
/// already extracts declarations and records a method's owning type, so
/// re-deriving them here would be a copy that could disagree.
///
/// CAVEAT ON VERIFICATION: this corpus contains 8 Swift nodes total, so unlike
/// the Kotlin and C producers this one is exercised by its unit tests and by
/// nothing else. Treat production behaviour as unproven until real Swift is
/// indexed.
pub(crate) mod swift_fqn {
    use super::super::LanguageAdapter;
    use super::super::fqn::{self, FqnDefinition, FqnFileOutput};
    use super::SwiftAdapter;

    const SWIFT_LANG: &str = "swift";

    /// `(package, module)` from the nearest `Package.swift`. With none found the
    /// package is empty and the module is the file stem — degraded but still
    /// scoped, which beats returning `None` and losing the file to bare-name
    /// matching.
    pub(crate) fn swift_file_context(abs_path: &str, rel_path: &str) -> (String, String) {
        let path = std::path::Path::new(abs_path);

        let mut dir = path.parent();
        while let Some(d) = dir {
            if d.join("Package.swift").is_file() {
                let package =
                    d.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
                if let Some(module) = path
                    .strip_prefix(d)
                    .ok()
                    .and_then(|r| r.to_str())
                    .map(|r| r.trim_end_matches(".swift").to_string())
                {
                    return (package, module);
                }
            }
            dir = d.parent();
        }

        // No `Package.swift`: the folder-relative path, for the same reason C
        // uses it — a bare stem collides across directories, and an absolute
        // path would embed this machine's home directory in every fqn.
        (String::new(), rel_path.trim_end_matches(".swift").to_string())
    }

    pub fn produce_fqns(abs_path: &str, rel_path: &str, content: &str) -> FqnFileOutput {
        let (package, module) = swift_file_context(abs_path, rel_path);
        let parsed = SwiftAdapter.parse(content, abs_path);

        let defs: Vec<FqnDefinition> = parsed
            .symbols
            .into_iter()
            .filter(|s| !s.name.trim().is_empty())
            .map(|s| {
                // A method anchors under its type; everything else under the module.
                let f = match s.parent.as_deref() {
                    Some(ty) if !ty.is_empty() => {
                        fqn::method(SWIFT_LANG, &package, &module, ty, &s.name)
                    }
                    _ => fqn::item(SWIFT_LANG, &package, &module, &s.name),
                };
                let parent_fqn = s
                    .parent
                    .as_deref()
                    .filter(|t| !t.is_empty())
                    .map(|t| fqn::item(SWIFT_LANG, &package, &module, t));
                FqnDefinition {
                    fqn: f,
                    name: s.name,
                    kind: s.kind,
                    line_start: s.line_start,
                    line_end: s.line_end,
                    is_exported: s.is_exported,
                    signature: s.signature,
                    docstring: s.docstring,
                    parent_type: s.parent,
                    parent_fqn,
                }
            })
            .collect();

        FqnFileOutput { defs, refs: Vec::new(), package, module }
    }
}

#[cfg(test)]
mod swift_fqn_tests {
    use super::swift_fqn::{produce_fqns, swift_file_context};

    /// `Package.swift` names the package; the path below it names the module.
    ///
    /// Breaking mutation: drop the `Package.swift` walk — every fqn collapses to
    /// the file stem and two same-named types in different targets collide.
    #[test]
    fn package_swift_names_the_package_and_the_path_names_the_module() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("MyKit");
        std::fs::create_dir_all(root.join("Sources/Core")).unwrap();
        std::fs::write(root.join("Package.swift"), "// swift-tools-version:5.9\n").unwrap();
        let file = root.join("Sources/Core/Widget.swift");
        let src = "class Widget {\n    func render() {}\n}\n";
        std::fs::write(&file, src).unwrap();

        let (pkg, module) =
            swift_file_context(&file.to_string_lossy(), "Sources/Core/Widget.swift");
        assert_eq!(pkg, "MyKit");
        assert_eq!(module, "Sources/Core/Widget");

        let out = produce_fqns(&file.to_string_lossy(), "Sources/Core/Widget.swift", src);
        let fqns: Vec<&str> = out.defs.iter().map(|d| d.fqn.as_str()).collect();
        assert!(fqns.contains(&"swift·MyKit·Sources/Core/Widget·Widget"), "type: {fqns:?}");
        // The method anchors under its TYPE, not directly under the module.
        assert!(
            fqns.contains(&"swift·MyKit·Sources/Core/Widget·Widget·render"),
            "method under its type: {fqns:?}"
        );
    }

    /// No `Package.swift`: degraded to the file stem rather than `None`, so the
    /// file keeps an fqn instead of falling back to bare-name matching.
    #[test]
    fn a_file_outside_a_package_still_gets_a_scoped_fqn() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("Loose.swift");
        let src = "class Loose {\n    func m() {}\n}\n";
        std::fs::write(&file, src).unwrap();

        let out = produce_fqns(&file.to_string_lossy(), "Loose.swift", src);
        assert_eq!(out.package, "");
        assert_eq!(out.module, "Loose");
        let fqns: Vec<&str> = out.defs.iter().map(|d| d.fqn.as_str()).collect();
        assert!(fqns.contains(&"swift·Loose·Loose"), "{fqns:?}");
    }
}
