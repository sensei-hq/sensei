//! `CMakeManifestAdapter` — `CMakeLists.txt`.
//!
//! The manifest C and C++ actually have. Every other build file those languages
//! use — `Makefile`, `configure.ac`, `meson.build` — is a build SCRIPT that
//! states no package name, and this registry's contract is that a manifest
//! NAMES the package it roots. CMake is the one that does:
//!
//! ```cmake
//! project(mylib VERSION 1.2.0 LANGUAGES C)
//! ```
//!
//! # A Makefile is deliberately not here
//!
//! `parse_manifest` receives the CONTENT and nothing else — not the path — so
//! an adapter for `Makefile` could only answer `None`, and `package_named_by`
//! turns that into "keep climbing". Registering one would therefore change
//! nothing except to put a file in front of the walk that can never answer.
//!
//! The directory name is NOT substituted, and the API is what forbids it: a
//! name derived from a path is not a name the source stated, and
//! `placement_on_disk` climbing past a silent build file to a manifest that
//! does state one is the correct behaviour rather than a gap. MEASURED over the
//! 165 `.c`/`.h` files in the watched roots: 158 place through an enclosing
//! `pom.xml`, `Cargo.toml`, `composer.json` or `package.json`, and the 7 that
//! do not have no manifest of any kind above them — including no CMake.
//!
//! So this adapter rescues NOTHING in the current corpus, and it is here
//! because CMake is how C and C++ projects declare themselves and the next one
//! added will say so in a `project()` call.
//!
//! # Dependencies
//!
//! CMake has no dependency manifest in the npm/Cargo sense. `find_package(X)`
//! declares a REQUIREMENT on something the system is expected to provide, and
//! `FetchContent_Declare`/`ExternalProject_Add` fetch by URL at configure time.
//! `find_package` is read here because it is the closest thing to a declared
//! dependency and it is what a stack summary wants; a version is recorded only
//! when the call states one.

use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::DepVersion;
use regex::Regex;
use std::sync::OnceLock;

pub struct CMakeManifestAdapter;

impl ManifestAdapter for CMakeManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["CMakeLists.txt"]
    }

    fn ecosystem(&self) -> &'static str {
        "cmake"
    }

    /// `find_package(Foo 1.2 REQUIRED)` — the name, and the version only when
    /// the call states one.
    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        let stripped = strip_cmake_comments(content);
        let mut out = Vec::new();
        for cap in find_package_re().captures_iter(&stripped) {
            let Some(name) = cap.get(1).map(|m| m.as_str().to_string()) else { continue };
            if name.is_empty() {
                continue;
            }
            // `*` is this registry's spelling for "no version constraint
            // stated", which is what every other adapter writes when the
            // manifest gives none.
            let version =
                cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_else(|| "*".to_string());
            out.push(DepVersion {
                lib_name: name,
                version: version.clone(),
                raw_version: version,
                source: "CMakeLists.txt".into(),
                // CMake has no dev/test dependency section. Claiming one would
                // be a fact the file does not carry.
                dev: false,
                local_source: None,
            });
        }
        out
    }

    /// CMake has no workspace concept. `add_subdirectory` composes a build, not
    /// a set of independently-versioned packages, and reading it as a workspace
    /// root would make every non-trivial CMake project one.
    fn is_workspace_root(&self, _content: &str) -> bool {
        false
    }

    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        let stripped = strip_cmake_comments(content);
        let captured = project_re().captures(&stripped);
        let name = captured.as_ref().and_then(|c| c.get(1)).map(|m| m.as_str().to_string());
        // `project(foo VERSION 1.2.0)`. Absent when the call states none —
        // never defaulted to `0.0.0`, which a caller could not tell from a real
        // version.
        let version = captured.as_ref().and_then(|c| c.get(2)).map(|m| m.as_str().to_string());
        ParsedManifest { name, version, description: None }
    }

    /// The `LANGUAGES` clause states them, and it is the only place CMake does.
    /// A project that omits it means `C CXX`, which is CMake's own default.
    fn stack_labels(&self, content: &str) -> Vec<&'static str> {
        let stripped = strip_cmake_comments(content);
        let Some(languages) = languages_re().captures(&stripped).and_then(|c| c.get(1)) else {
            return vec!["c", "cpp"];
        };
        // WHOLE TOKENS, not substrings. `"CXX".contains("C")` is true, so a
        // C++-only project read as C++ AND C — the label `c` on a project with
        // no C in it at all.
        let stated: Vec<String> = languages
            .as_str()
            .split(|c: char| c.is_whitespace() || c == ';')
            .filter(|t| !t.is_empty())
            .map(|t| t.to_ascii_uppercase())
            .collect();
        let mut out = Vec::new();
        for (token, label) in
            [("CXX", "cpp"), ("C", "c"), ("CUDA", "cuda"), ("FORTRAN", "fortran"), ("ASM", "asm")]
        {
            if stated.iter().any(|t| t == token) {
                out.push(label);
            }
        }
        if out.is_empty() { vec!["c", "cpp"] } else { out }
    }

    /// Built by hand rather than through `conventional_commands`, and the
    /// reason is CMake's own shape: its build verb is a FLAG (`cmake --build`)
    /// and its test verb is a DIFFERENT BINARY (`ctest`). The helper prefixes
    /// one CLI to a subcommand, which fits neither.
    ///
    /// "How do I test this?" is the load-bearing question, so `ctest` is
    /// emitted rather than omitted — answering it with a `cmake` invocation
    /// that does not run tests would be worse than saying nothing.
    fn parse_commands(&self, _content: &str) -> Vec<super::DiscoveredCommand> {
        vec![
            super::DiscoveredCommand {
                raw_name: "build".to_string(),
                command_line: "cmake --build build".to_string(),
                category: Some(super::canonical_verb("build")),
            },
            super::DiscoveredCommand {
                raw_name: "test".to_string(),
                command_line: "ctest --test-dir build".to_string(),
                category: Some(super::canonical_verb("test")),
            },
        ]
    }
}

/// `project(NAME ...)` with an optional `VERSION x.y.z`.
///
/// Case-insensitive because CMake's commands are: `PROJECT(foo)` and
/// `project(foo)` are the same call, and a project using the upper-case style
/// would otherwise name nothing.
fn project_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\bproject\s*\(\s*([A-Za-z0-9_.+-]+)(?:[^)]*?\bVERSION\s+([0-9][0-9A-Za-z.+-]*))?[^)]*\)")
            .expect("a literal pattern compiles")
    })
}

fn find_package_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\bfind_package\s*\(\s*([A-Za-z0-9_.+-]+)(?:\s+([0-9][0-9A-Za-z.]*))?")
            .expect("a literal pattern compiles")
    })
}

fn languages_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\bproject\s*\([^)]*?\bLANGUAGES\s+([^)]*)\)")
            .expect("a literal pattern compiles")
    })
}

/// Drop `#` comments. CMake has no block comment in the form this needs to
/// handle — `#[[ ]]` exists but is vanishingly rare — and a `#` inside a quoted
/// string is left alone so a path or a flag is not truncated mid-argument.
fn strip_cmake_comments(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    for line in content.lines() {
        let mut quoted = false;
        let mut cut = line.len();
        for (at, ch) in line.char_indices() {
            match ch {
                '"' => quoted = !quoted,
                '#' if !quoted => {
                    cut = at;
                    break;
                }
                _ => {}
            }
        }
        out.push_str(&line[..cut]);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **THE PACKAGE IS THE ONE CMake STATES**, and the shapes it states it in.
    ///
    /// MUTATION: drop the `(?i)` — a project written `PROJECT(foo)` names
    /// nothing, and `placement_on_disk` climbs straight past it.
    #[test]
    fn project_names_the_package_however_the_call_is_written() {
        let name = |s: &str| CMakeManifestAdapter.parse_manifest(s).name;
        assert_eq!(name("project(mylib)"), Some("mylib".into()));
        assert_eq!(name("PROJECT(MyLib)"), Some("MyLib".into()));
        assert_eq!(name("project( my-lib  C )"), Some("my-lib".into()));
        assert_eq!(
            name("cmake_minimum_required(VERSION 3.10)\nproject(core LANGUAGES C)\n"),
            Some("core".into())
        );
        // A file that states NO project names nothing, which is what makes
        // `placement_on_disk` keep climbing rather than place a file wrong.
        assert_eq!(name("add_subdirectory(src)\n"), None);
        assert_eq!(name(""), None);
    }

    /// A version is recorded only when the call STATES one.
    ///
    /// MUTATION: default it to `0.0.0` — a caller cannot tell that from a real
    /// version, which is the failure mode the no-fabrication rule names.
    #[test]
    fn a_version_is_absent_unless_the_project_call_states_it() {
        let v = |s: &str| CMakeManifestAdapter.parse_manifest(s).version;
        assert_eq!(v("project(mylib VERSION 1.2.0)"), Some("1.2.0".into()));
        assert_eq!(v("project(mylib VERSION 2.0 LANGUAGES C CXX)"), Some("2.0".into()));
        assert_eq!(v("project(mylib LANGUAGES C)"), None);
        assert_eq!(v("project(mylib)"), None);
    }

    /// A comment is not a declaration.
    ///
    /// MUTATION: stop stripping comments — a commented-out `project()` above
    /// the real one wins, and every file under it is placed under a package the
    /// build does not have.
    #[test]
    fn a_commented_out_call_declares_nothing() {
        let adapter = CMakeManifestAdapter;
        assert_eq!(
            adapter.parse_manifest("# project(old)\nproject(new)\n").name,
            Some("new".into())
        );
        assert!(adapter.parse_dependencies("# find_package(Boost)\n").is_empty());
        // A `#` INSIDE a quoted argument is not a comment — truncating there
        // would cut a flag or a path in half.
        assert_eq!(
            adapter.parse_manifest("project(app)\nset(F \"-Wall#keep\")\n").name,
            Some("app".into())
        );
    }

    /// `find_package` is the closest thing CMake has to a declared dependency.
    #[test]
    fn find_package_is_the_declared_dependency_and_its_version_is_optional() {
        let deps = CMakeManifestAdapter.parse_dependencies(
            "find_package(Threads REQUIRED)\nfind_package(Boost 1.70 COMPONENTS system)\n",
        );
        let got: Vec<(&str, &str)> =
            deps.iter().map(|d| (d.lib_name.as_str(), d.version.as_str())).collect();
        assert!(got.contains(&("Threads", "*")), "{got:?}");
        assert!(got.contains(&("Boost", "1.70")), "{got:?}");
        assert!(deps.iter().all(|d| !d.dev), "CMake states no dev section");
    }

    /// `LANGUAGES` states the stack, and CMake's own default is `C CXX`.
    #[test]
    fn the_languages_clause_states_the_stack_and_its_absence_means_c_and_cpp() {
        let s = |c: &str| CMakeManifestAdapter.stack_labels(c);
        assert_eq!(s("project(a LANGUAGES C)"), vec!["c"]);
        // `CXX` must NOT also match `C`. A substring check said this project
        // was C++ AND C, putting the label `c` on a project with no C in it.
        assert_eq!(s("project(a LANGUAGES CXX)"), vec!["cpp"]);
        assert_eq!(s("project(a LANGUAGES C CXX)"), vec!["cpp", "c"]);
        assert_eq!(s("project(a)"), vec!["c", "cpp"]);
    }

    /// **CMAKE IS NOT A WORKSPACE FORMAT.** `add_subdirectory` composes one
    /// build; it does not declare independently-versioned packages.
    ///
    /// MUTATION: read `add_subdirectory` as a workspace marker — every
    /// non-trivial CMake project becomes a monorepo.
    /// The build verb is a FLAG and the test verb is a different BINARY.
    #[test]
    fn the_commands_are_cmakes_own_and_ctest_answers_for_test() {
        let cmds = CMakeManifestAdapter.parse_commands("");
        let test = cmds.iter().find(|c| c.category == Some("test")).expect("test is emitted");
        assert!(test.command_line.starts_with("ctest"), "{}", test.command_line);
        let build = cmds.iter().find(|c| c.category == Some("build")).expect("build is emitted");
        assert!(build.command_line.starts_with("cmake --build"), "{}", build.command_line);
    }

    #[test]
    fn add_subdirectory_is_not_a_workspace() {
        assert!(!CMakeManifestAdapter.is_workspace_root("add_subdirectory(src)\n"));
        assert!(!CMakeManifestAdapter.is_workspace_root("project(a)\nadd_subdirectory(lib)\n"));
    }
}
