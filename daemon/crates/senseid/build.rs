fn main() {
    // Compile vendored Kotlin grammar against tree-sitter 0.24 headers
    let kotlin_dir = std::path::PathBuf::from("grammars/kotlin/src");
    cc::Build::new()
        .include(&kotlin_dir)
        .file(kotlin_dir.join("parser.c"))
        .file(kotlin_dir.join("scanner.c"))
        .warnings(false)
        .compile("tree-sitter-kotlin");

    // Compile vendored Swift grammar (0.6.0, ABI 14) against tree-sitter 0.24 headers
    let swift_dir = std::path::PathBuf::from("grammars/swift/src");
    cc::Build::new()
        .include(&swift_dir)
        .file(swift_dir.join("parser.c"))
        .file(swift_dir.join("scanner.c"))
        .warnings(false)
        .compile("tree-sitter-swift");
}
