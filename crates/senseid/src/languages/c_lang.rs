use super::LanguageAdapter;
use super::common::{ir_module, ir_parsed_file};
use crate::ir::{ClassKind, IRBase, IRClass, IRFunction, IRParsedFile};
use crate::types::{ParsedFile, ParsedSymbol, SymbolKind};

pub struct CAdapter;

impl LanguageAdapter for CAdapter {
    fn supports_fqn(&self) -> bool {
        true
    }

    fn fqn_output(
        &self,
        abs_path: &str,
        rel_path: &str,
        content: &str,
    ) -> Option<super::fqn::FqnFileOutput> {
        // Needs the path: C has no in-source package declaration, so scope comes
        // from the nearest build root plus the file's position under it.
        Some(c_fqn::produce_fqns(abs_path, rel_path, content))
    }

    fn extensions(&self) -> &[&'static str] {
        &[".c", ".h", ".cpp", ".hpp", ".cc"]
    }

    fn language(&self) -> &str {
        "c"
    }
    fn display_name(&self) -> &str {
        "C"
    }

    fn parse_to_ir(&self, source: &str, file_path: &str) -> crate::ir::IRParsedFile {
        parse_to_ir(source, file_path)
    }

    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        // Skip very large generated files (e.g. tree-sitter parsers)
        if source.len() > 500_000 {
            return ParsedFile {
                file_path: file_path.into(),
                language: "c".into(),
                symbols: vec![],
                edges: vec![],
                imports: vec![],
            };
        }

        let mut symbols = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            let line_num = (i + 1) as u32;

            // Function definitions: type name(params) {
            // Match patterns like: void foo(int x) {  or  int main() {
            if !trimmed.starts_with("//")
                && !trimmed.starts_with("/*")
                && !trimmed.starts_with("*")
                && !trimmed.starts_with("#")
                && trimmed.contains('(')
                && (trimmed.ends_with('{') || trimmed.ends_with(") {"))
                && let Some(name) = extract_c_function_name(trimmed)
                && !name.is_empty()
                && name.len() < 100
                && !name.contains(' ')
            {
                symbols.push(ParsedSymbol {
                    name,
                    kind: SymbolKind::Function,
                    signature: Some(trimmed.trim_end_matches('{').trim().to_string()),
                    docstring: None,
                    line_start: line_num,
                    line_end: line_num,
                    is_exported: true,
                    parent: None,
                });
            }

            // Struct definitions: struct Name {
            if trimmed.starts_with("struct ") && trimmed.contains('{') {
                let name = trimmed
                    .strip_prefix("struct ")
                    .and_then(|s| s.split(|c: char| !c.is_alphanumeric() && c != '_').next())
                    .unwrap_or("")
                    .to_string();
                if !name.is_empty() {
                    symbols.push(ParsedSymbol {
                        name,
                        kind: SymbolKind::Struct,
                        signature: Some(trimmed.to_string()),
                        docstring: None,
                        line_start: line_num,
                        line_end: line_num,
                        is_exported: true,
                        parent: None,
                    });
                }
            }

            // Typedef
            if trimmed.starts_with("typedef ") {
                // typedef struct ... Name; or typedef type Name;
                if let Some(name) = trimmed
                    .strip_prefix("typedef ")
                    .and_then(|s| s.trim_end_matches(';').rsplit_once(|c: char| c.is_whitespace()))
                    .map(|(_, name)| name.trim().to_string())
                    && !name.is_empty()
                    && !name.contains('{')
                {
                    symbols.push(ParsedSymbol {
                        name,
                        kind: SymbolKind::Type,
                        signature: Some(trimmed.to_string()),
                        docstring: None,
                        line_start: line_num,
                        line_end: line_num,
                        is_exported: true,
                        parent: None,
                    });
                }
            }

            // #define constants
            if trimmed.starts_with("#define ") {
                let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
                if parts.len() >= 2 {
                    let name = parts[1].split('(').next().unwrap_or("").to_string();
                    if !name.is_empty() && name == name.to_uppercase() {
                        symbols.push(ParsedSymbol {
                            name,
                            kind: SymbolKind::Const,
                            signature: Some(trimmed.to_string()),
                            docstring: None,
                            line_start: line_num,
                            line_end: line_num,
                            is_exported: true,
                            parent: None,
                        });
                    }
                }
            }
        }

        // Extract #include as imports
        let imports = source
            .lines()
            .filter(|l| l.trim().starts_with("#include"))
            .filter_map(|l| {
                let path = l
                    .trim()
                    .strip_prefix("#include")?
                    .trim()
                    .trim_start_matches(['<', '"'])
                    .trim_end_matches(['>', '"'])
                    .to_string();
                Some(crate::types::ParsedImport { target_path: path.clone(), names: vec![path] })
            })
            .collect();

        ParsedFile {
            file_path: file_path.into(),
            language: "c".into(),
            symbols,
            edges: vec![],
            imports,
        }
    }
}

fn extract_c_function_name(line: &str) -> Option<String> {
    // Find the function name before the opening paren
    let paren_pos = line.find('(')?;
    let before_paren = line[..paren_pos].trim();
    // Last word before ( is the function name
    let name = before_paren
        .rsplit(|c: char| c.is_whitespace() || c == '*')
        .next()
        .unwrap_or("")
        .to_string();
    if name.is_empty()
        || name.starts_with('#')
        || ["if", "while", "for", "switch", "return"].contains(&name.as_str())
    {
        None
    } else {
        Some(name)
    }
}

/// Parse C/C++ into IR — functions and structs.
pub fn parse_to_ir(source: &str, file_path: &str) -> IRParsedFile {
    let pf = CAdapter.parse(source, file_path);
    let mut functions = Vec::new();
    let mut classes = Vec::new();

    for sym in &pf.symbols {
        match sym.kind {
            SymbolKind::Function => {
                functions.push(IRFunction {
                    base: IRBase {
                        name: sym.name.clone(),
                        line_start: sym.line_start,
                        line_end: sym.line_end,
                        docstring: sym.docstring.clone(),
                        is_exported: sym.is_exported,
                        node_type: Some("function".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            }
            SymbolKind::Class => {
                classes.push(IRClass {
                    base: IRBase {
                        name: sym.name.clone(),
                        line_start: sym.line_start,
                        line_end: sym.line_end,
                        is_exported: sym.is_exported,
                        node_type: Some("class".into()),
                        ..Default::default()
                    },
                    class_kind: ClassKind::Struct,
                    ..Default::default()
                });
            }
            _ => {}
        }
    }

    let module =
        ir_module(file_path, "c", functions, Vec::new(), Vec::new(), file_path.contains("test"));
    ir_parsed_file(file_path, "c", module, classes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile {
        CAdapter.parse(src, "test.c")
    }

    #[test]
    fn parses_function() {
        let pf = parse("int main(int argc, char **argv) {\n  return 0;\n}");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "main");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn parses_struct() {
        let pf = parse("struct Point {\n  int x;\n  int y;\n};");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "Point");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Struct);
    }

    #[test]
    fn parses_define() {
        let pf = parse("#define MAX_SIZE 100\n#define min(a,b) ((a)<(b)?(a):(b))");
        let consts: Vec<_> = pf.symbols.iter().filter(|s| s.kind == SymbolKind::Const).collect();
        assert_eq!(consts.len(), 1);
        assert_eq!(consts[0].name, "MAX_SIZE");
    }

    #[test]
    fn parses_includes() {
        let pf = parse("#include <stdio.h>\n#include \"mylib.h\"\nint main() {");
        assert_eq!(pf.imports.len(), 2);
        assert_eq!(pf.imports[0].target_path, "stdio.h");
        assert_eq!(pf.imports[1].target_path, "mylib.h");
    }

    #[test]
    fn skips_large_generated_files() {
        let large = "a\n".repeat(300_000);
        let pf = parse(&large);
        assert!(pf.symbols.is_empty(), "should skip large generated files");
    }

    #[test]
    fn language() {
        let pf = parse("int x;");
        assert_eq!(pf.language, "c");
    }
}

/// C FQN production — file-scoped, because C has no namespaces.
///
/// A C symbol is either file-scoped (`static`) or has external linkage and is
/// globally unique by the linker's rules. Either way the FILE is the correct
/// scope, so the fqn is `c·<package>·<module>·<name>` with `module` the
/// build-root-relative path. That is strictly MORE precise than the bare-name
/// matching C symbols fell back to: two `init` functions in different
/// translation units were previously indistinguishable to a name lookup.
///
/// `package` comes from the nearest ancestor holding a build file, mirroring how
/// the TS and Rust producers use `package.json` / `Cargo.toml`. This is a
/// PROJECTION of the existing line-based `parse()` rather than a second parser —
/// the symbols and their kinds are already extracted, and re-deriving them would
/// be a copy that could disagree.
pub(crate) mod c_fqn {
    use super::super::LanguageAdapter;
    use super::super::fqn::{self, FqnDefinition, FqnFileOutput};
    use super::CAdapter;

    const C_LANG: &str = "c";
    /// Markers that identify a C build root, in the order they are preferred.
    const BUILD_FILES: &[&str] = &["CMakeLists.txt", "Makefile", "makefile", "configure.ac"];

    /// `(package, module)` for a C file: the nearest ancestor with a build file
    /// names the package, and the path below it names the module.
    ///
    /// With no build file anywhere the package is empty and the module is the
    /// file stem — degraded but still file-scoped, which beats returning `None`
    /// and dropping the whole file back onto bare-name matching.
    pub(crate) fn c_file_context(abs_path: &str, rel_path: &str) -> (String, String) {
        let path = std::path::Path::new(abs_path);

        let mut dir = path.parent();
        while let Some(d) = dir {
            if BUILD_FILES.iter().any(|f| d.join(f).is_file()) {
                let package =
                    d.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
                if let Some(module) =
                    path.strip_prefix(d).ok().and_then(|r| r.to_str()).map(strip_ext)
                {
                    return (package, module);
                }
            }
            dir = d.parent();
        }

        // No build root: fall back to the FOLDER-RELATIVE path, which is what
        // `nodes.file_path` stores. Deliberately not the file stem — a real
        // project in this corpus has parallel `Cpp/` and `Hpp/` trees, so
        // stems collide between a header and its implementation. Deliberately
        // not `abs_path` either: that would bake a home directory into every fqn.
        (String::new(), strip_ext(rel_path))
    }

    /// Drop a single trailing extension, whatever its case (`.CPP`, `.h`).
    fn strip_ext(p: &str) -> String {
        match p.rsplit_once('.') {
            // Only strip when the dot is in the final segment, so a dotted
            // directory name is never mistaken for an extension.
            Some((head, tail)) if !tail.contains('/') && !tail.is_empty() => head.to_string(),
            _ => p.to_string(),
        }
    }

    pub fn produce_fqns(abs_path: &str, rel_path: &str, content: &str) -> FqnFileOutput {
        let (package, module) = c_file_context(abs_path, rel_path);
        let parsed = CAdapter.parse(content, abs_path);

        let defs: Vec<FqnDefinition> = parsed
            .symbols
            .into_iter()
            .filter(|s| !s.name.trim().is_empty())
            .map(|s| FqnDefinition {
                fqn: fqn::item(C_LANG, &package, &module, &s.name),
                name: s.name,
                kind: s.kind,
                line_start: s.line_start,
                line_end: s.line_end,
                is_exported: s.is_exported,
                signature: s.signature,
                docstring: s.docstring,
                parent_type: None,
                parent_fqn: None,
            })
            .collect();

        FqnFileOutput { defs, refs: Vec::new(), package, module, relations: Vec::new() }
    }
}

#[cfg(test)]
mod c_fqn_tests {
    use super::c_fqn::{c_file_context, produce_fqns};

    /// The build root names the package and the path below it names the module,
    /// mirroring `package.json` / `Cargo.toml` for TS and Rust.
    ///
    /// Breaking mutation: drop the build-file walk and always use the file stem —
    /// two `parser.c` files in different subdirectories collide.
    #[test]
    fn the_nearest_build_file_names_the_package_and_the_path_names_the_module() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("libfoo");
        std::fs::create_dir_all(root.join("src/net")).unwrap();
        std::fs::write(root.join("Makefile"), "all:\n").unwrap();
        let file = root.join("src/net/sock.c");
        std::fs::write(&file, "int connect_now(void) {\n  return 1;\n}\n").unwrap();

        let (pkg, module) = c_file_context(&file.to_string_lossy(), "src/net/sock.c");
        assert_eq!(pkg, "libfoo", "the build-root directory names the package");
        assert_eq!(module, "src/net/sock", "path below the root, extension stripped");

        let out = produce_fqns(
            &file.to_string_lossy(),
            "src/net/sock.c",
            "int connect_now(void) {\n  return 1;\n}\n",
        );
        let fqns: Vec<&str> = out.defs.iter().map(|d| d.fqn.as_str()).collect();
        assert!(fqns.contains(&"c·libfoo·src/net/sock·connect_now"), "{fqns:?}");
    }

    /// Two same-named symbols in different translation units must get DIFFERENT
    /// fqns — that is the whole point of file scoping, and exactly what bare-name
    /// matching could not express.
    #[test]
    fn same_name_in_two_files_does_not_collide() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("a")).unwrap();
        std::fs::create_dir_all(tmp.path().join("b")).unwrap();
        std::fs::write(tmp.path().join("Makefile"), "all:\n").unwrap();
        // The line-based C parser needs the brace at end-of-line, which is how real
        // C is written; a one-liner is invalid input to it, not a missing feature.
        let src = "static int init(void) {\n  return 0;\n}\n";

        let fa = tmp.path().join("a/mod.c");
        let fb = tmp.path().join("b/mod.c");
        std::fs::write(&fa, src).unwrap();
        std::fs::write(&fb, src).unwrap();

        let a = produce_fqns(&fa.to_string_lossy(), "a/mod.c", src);
        let b = produce_fqns(&fb.to_string_lossy(), "b/mod.c", src);
        let fa_init = a.defs.iter().find(|d| d.name == "init").expect("a init").fqn.clone();
        let fb_init = b.defs.iter().find(|d| d.name == "init").expect("b init").fqn.clone();
        assert_ne!(fa_init, fb_init, "file scope must distinguish them: {fa_init} vs {fb_init}");
    }

    /// REGRESSION, found in real corpus data rather than by reasoning: a C/C++
    /// project with parallel `Cpp/` and `Hpp/` trees and NO build file. Both
    /// halves of a header/impl pair share a file stem, so a stem-only fallback
    /// gave them the same module — and a header/impl pair declares and defines
    /// the same names by construction, so the collision is guaranteed.
    ///
    /// The fallback must therefore still distinguish directories.
    ///
    /// Breaking mutation: make the no-build-root fallback return the bare file
    /// stem — `Cpp/ADVMATH` and `Hpp/ADVMATH` collapse onto each other.
    #[test]
    fn a_header_impl_pair_in_parallel_trees_does_not_collide_without_a_build_root() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("Bezier3D");
        std::fs::create_dir_all(repo.join("Cpp")).unwrap();
        std::fs::create_dir_all(repo.join("Hpp")).unwrap();
        // A repo marker but deliberately NO Makefile/CMakeLists.
        std::fs::create_dir_all(repo.join(".git")).unwrap();

        let impl_src = "double vec_len(double x) {\n  return x;\n}\n";
        let hdr_src = "double vec_len(double x) {\n  return x;\n}\n";
        let cpp = repo.join("Cpp/ADVMATH.CPP");
        let hpp = repo.join("Hpp/ADVMATH.H");
        std::fs::write(&cpp, impl_src).unwrap();
        std::fs::write(&hpp, hdr_src).unwrap();

        let a = produce_fqns(&cpp.to_string_lossy(), "Cpp/ADVMATH.CPP", impl_src);
        let b = produce_fqns(&hpp.to_string_lossy(), "Hpp/ADVMATH.H", hdr_src);
        assert_ne!(a.module, b.module, "modules must differ: {} vs {}", a.module, b.module);

        let fa = a.defs.iter().find(|d| d.name == "vec_len").expect("impl def").fqn.clone();
        let fb = b.defs.iter().find(|d| d.name == "vec_len").expect("header def").fqn.clone();
        assert_ne!(fa, fb, "header and impl must not share an fqn: {fa} vs {fb}");
    }

    /// No build file anywhere: degraded to the file stem rather than returning
    /// nothing, because `None` would drop the file back to bare-name matching —
    /// the very thing this producer exists to replace.
    #[test]
    fn a_file_with_no_build_root_still_gets_a_file_scoped_fqn() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("loose.c");
        let src = "int only(void) {\n  return 1;\n}\n";
        std::fs::write(&file, src).unwrap();

        let out = produce_fqns(&file.to_string_lossy(), "loose.c", src);
        assert_eq!(out.package, "");
        assert_eq!(out.module, "loose");
        let fqns: Vec<&str> = out.defs.iter().map(|d| d.fqn.as_str()).collect();
        assert!(fqns.contains(&"c·loose·only"), "{fqns:?}");
    }
}
