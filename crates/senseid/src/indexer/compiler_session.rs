//! THE TASK LAYER'S HALF OF THE COMPILER STEP — the IO an adapter describes and
//! never performs (docs/plan/2026-10-13-indexer-full-call-placement.md).
//!
//! An adapter declares its compiler ([`CompilerEngine`]): how to find the
//! project a package belongs to, how to compile it, how a resolved symbol becomes
//! an identity. This module runs those commands, once per project per scan, and
//! reads definition files back through their own adapters when a target's
//! identity is the declaration the walk mints there.
//!
//! Language-neutral: nothing here names a language. A new language with a
//! compiler is an adapter declaring one.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::indexer::facts::{FileFacts, Fqn, Reason};
use crate::indexer::lang::compiler::{CompilerEngine, CompilerIndex, Declarations, Invocation};
use crate::indexer::lang::{self, Source, TypeHomes};

/// How long one compile may take before it is abandoned. A bounded wait: a
/// compiler that never returns must not hold every file task of its project.
const COMPILE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30 * 60);
/// How long the build tool may take to name a package's project.
const LOCATE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// One compiled project: its root and what its compiler resolved.
pub struct Project {
    pub root: PathBuf,
    pub index: CompilerIndex,
}

type Shared<T> = Arc<tokio::sync::OnceCell<Result<Arc<T>, String>>>;

/// The compiler answers of ONE scan of one repository. Built lazily — a project
/// is compiled the first time one of its files asks — and shared by every file
/// task of the scan, so each project is compiled once however many files it has.
pub struct CompilerSessions {
    repo_root: PathBuf,
    /// package directory → the project root its build tool names.
    located: Mutex<HashMap<(&'static str, PathBuf), Shared<PathBuf>>>,
    /// project root → the compiled project.
    compiled: Mutex<HashMap<(&'static str, PathBuf), Shared<Project>>>,
    /// Definition files read back through their adapters, for this scan.
    declared: Mutex<HashMap<PathBuf, Result<Arc<FileFacts>, Reason>>>,
}

impl CompilerSessions {
    /// `repo_root` is canonicalised: the build tool answers with canonical
    /// paths (`/private/var/…` for `/var/…`), and a file is related to its
    /// project by path prefix, which two spellings of one directory defeat.
    pub fn new(repo_root: &Path) -> Self {
        Self {
            repo_root: canonical(repo_root),
            located: Mutex::new(HashMap::new()),
            compiled: Mutex::new(HashMap::new()),
            declared: Mutex::new(HashMap::new()),
        }
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// The compiled project the package at `package_dir` belongs to. An `Err`
    /// carries why — the build tool or the compiler failed, or is missing — and
    /// the caller's calls then say `engine_not_run`.
    pub async fn project_for(
        &self,
        engine: &'static dyn CompilerEngine,
        package_dir: &Path,
    ) -> Result<Arc<Project>, String> {
        let key = (engine.manifest(), package_dir.to_path_buf());
        let cell = Self::cell(&self.located, key)?;
        let root = cell
            .get_or_init(|| async {
                let out =
                    run(&engine.locate(&package_dir.to_string_lossy()), LOCATE_TIMEOUT).await?;
                engine
                    .project_root(&out)
                    .map(|r| Arc::new(canonical(Path::new(&r))))
                    .ok_or_else(|| format!("no project root in {out:?}"))
            })
            .await
            .clone()?;

        let cell = Self::cell(&self.compiled, (engine.manifest(), (*root).clone()))?;
        cell.get_or_init(|| async { compile(engine, &root).await.map(Arc::new) }).await.clone()
    }

    fn cell<T>(
        map: &Mutex<HashMap<(&'static str, PathBuf), Shared<T>>>,
        key: (&'static str, PathBuf),
    ) -> Result<Shared<T>, String> {
        let mut map = map.lock().map_err(|_| "compiler session cache poisoned".to_string())?;
        Ok(map.entry(key).or_default().clone())
    }

    /// The declaration lookup for files of `project`.
    pub fn declarations<'a>(&'a self, project: &'a Project) -> DiskDeclarations<'a> {
        DiskDeclarations { sessions: self, project_root: &project.root }
    }
}

/// The sessions of each repository's CURRENT scan, by folder.
fn scans() -> &'static Mutex<HashMap<uuid::Uuid, Arc<CompilerSessions>>> {
    static SCANS: std::sync::OnceLock<Mutex<HashMap<uuid::Uuid, Arc<CompilerSessions>>>> =
        std::sync::OnceLock::new();
    SCANS.get_or_init(|| Mutex::new(HashMap::new()))
}

impl CompilerSessions {
    /// The sessions of this repository's current scan, shared by all its file
    /// tasks. Lives and dies with the scan exactly as `TellFile::shared` does.
    pub fn shared(folder_id: &uuid::Uuid, repo_root: &Path) -> Result<Arc<Self>, String> {
        let mut scans =
            scans().lock().map_err(|_| "compiler session cache poisoned".to_string())?;
        Ok(scans.entry(*folder_id).or_insert_with(|| Arc::new(Self::new(repo_root))).clone())
    }

    /// Drop a repository's sessions. ONE WRITER, the same as `TellFile::forget`:
    /// the gate that runs once per repo before the file fan-out, so a scan never
    /// sees answers compiled for the tree before it.
    pub fn forget(folder_id: &uuid::Uuid) {
        if let Ok(mut scans) = scans().lock() {
            scans.remove(folder_id);
        }
    }
}

/// A path with its symlinks resolved, or as given when it cannot be (a path
/// that does not exist resolves to nothing, and is compared as written).
pub fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Compile `root` into a SCIP index and read it.
async fn compile(engine: &'static dyn CompilerEngine, root: &Path) -> Result<Project, String> {
    let out_dir = std::env::temp_dir().join("sensei-compiler");
    tokio::fs::create_dir_all(&out_dir)
        .await
        .map_err(|e| format!("cannot create {out_dir:?}: {e}"))?;
    let name: String = root
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let output = out_dir.join(format!("{name}.scip"));
    let root_str = root.to_string_lossy();
    let compiler = run(&engine.locate_compiler(&root_str), LOCATE_TIMEOUT).await?;
    let compiler = compiler.trim();
    if compiler.is_empty() {
        return Err("the compiler's binary could not be located".to_string());
    }
    run(&engine.compile(compiler, &root_str, &output.to_string_lossy()), COMPILE_TIMEOUT).await?;
    let bytes = tokio::fs::read(&output)
        .await
        .map_err(|e| format!("compiler wrote no index at {output:?}: {e}"))?;
    let index = CompilerIndex::from_bytes(&bytes)?;
    Ok(Project { root: root.to_path_buf(), index })
}

/// Run an adapter's invocation, bounded by `timeout`; its stdout on success.
async fn run(inv: &Invocation, timeout: std::time::Duration) -> Result<String, String> {
    let child = tokio::process::Command::new(&inv.program)
        .args(&inv.args)
        .current_dir(&inv.cwd)
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(timeout, child)
        .await
        .map_err(|_| format!("`{}` did not finish within {timeout:?}", inv.program))?
        .map_err(|e| format!("`{}` could not run: {e}", inv.program))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("`{}` failed ({}): {}", inv.program, out.status, err.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// [`Declarations`] read from disk: a definition file is read through its own
/// adapter, placed and walked exactly as its own index task does, and the
/// declaration named so whose span holds the line is the answer.
pub struct DiskDeclarations<'a> {
    sessions: &'a CompilerSessions,
    project_root: &'a Path,
}

impl Declarations for DiskDeclarations<'_> {
    fn declared_at(&self, path: &str, line: u32, name: &str) -> Result<Fqn, Reason> {
        let facts = self.facts_of(&self.project_root.join(path))?;
        facts
            .symbols
            .iter()
            .filter(|s| s.name == name && s.span.start_line <= line && line <= s.span.end_line)
            .max_by_key(|s| s.span.start_line)
            .map(|s| s.fqn.clone())
            .ok_or(Reason::DeclarationNotFound)
    }
}

impl DiskDeclarations<'_> {
    fn facts_of(&self, abs: &Path) -> Result<Arc<FileFacts>, Reason> {
        if let Some(found) = self.sessions.declared.lock().ok().and_then(|m| m.get(abs).cloned()) {
            return found;
        }
        let read = read_declarations(self.sessions.repo_root(), abs);
        if let Ok(mut map) = self.sessions.declared.lock() {
            map.insert(abs.to_path_buf(), read.clone());
        }
        read
    }
}

/// Read one definition file the way its own index task does: the same
/// repo-relative path, the same placement, the same adapter, no type table.
fn read_declarations(repo_root: &Path, abs: &Path) -> Result<Arc<FileFacts>, Reason> {
    let abs = &canonical(abs);
    let rel = abs.strip_prefix(repo_root).map_err(|_| Reason::DeclarationNotFound)?;
    let rel = rel.to_string_lossy().into_owned();
    let ext =
        rel.rsplit_once('.').map(|(_, e)| format!(".{e}")).ok_or(Reason::DeclarationNotFound)?;
    let adapter = lang::adapter_for_ext(&ext).ok_or(Reason::DeclarationNotFound)?;
    let bytes = std::fs::read(abs).map_err(|_| Reason::DeclarationNotFound)?;
    let crate::classifiers::Decoded::Text(text) = crate::classifiers::decode_source(&bytes) else {
        return Err(Reason::DeclarationNotFound);
    };
    let placement = crate::indexer::pipeline::placement_on_disk(abs, repo_root, adapter.language())
        .ok_or(Reason::DeclarationNotFound)?;
    let source =
        Source { package: &placement.package, module: &placement.module, path: &rel, text: &text };
    adapter
        .read(&source, &TypeHomes::unknown())
        .map(Arc::new)
        .map_err(|_| Reason::DeclarationNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real repository on disk: the definition is read back through the Rust
    /// adapter and the identity is the one the walk mints for it.
    #[test]
    fn a_declaration_is_read_back_as_the_walk_mints_it() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/net.rs"),
            "pub struct Listener;\n\nimpl Listener {\n    pub fn bind() -> Self {\n        Listener\n    }\n}\n",
        )
        .unwrap();
        let sessions = CompilerSessions::new(root);
        let project = Project {
            root: root.to_path_buf(),
            index: CompilerIndex::from_index(&scip::types::Index::new()),
        };
        let decl = sessions.declarations(&project);
        assert_eq!(
            decl.declared_at("src/net.rs", 4, "bind").unwrap().as_str(),
            "rust·demo·net·Listener·bind·item"
        );
        assert_eq!(
            decl.declared_at("src/net.rs", 1, "Listener").unwrap().as_str(),
            "rust·demo·net·Listener·item"
        );
        assert_eq!(decl.declared_at("src/net.rs", 4, "nothing"), Err(Reason::DeclarationNotFound));
        assert_eq!(decl.declared_at("src/gone.rs", 1, "x"), Err(Reason::DeclarationNotFound));
    }

    /// Two declarations of one name, one inside the other: the compiler's line
    /// names the INNER one, which is what a call to it reaches.
    #[test]
    fn the_innermost_declaration_holding_the_line_is_the_answer() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/lib.rs"),
            "pub mod listener {\n    pub fn listener() {}\n}\n",
        )
        .unwrap();
        let sessions = CompilerSessions::new(root);
        let project = Project {
            root: root.to_path_buf(),
            index: CompilerIndex::from_index(&scip::types::Index::new()),
        };
        let got = sessions.declarations(&project).declared_at("src/lib.rs", 2, "listener").unwrap();
        assert!(got.as_str().ends_with("listener·listener·item"), "got {got:?}");
    }

    /// A command that is not installed is an `Err` that says so, not a hang
    /// and not an empty answer.
    #[tokio::test]
    async fn a_missing_engine_is_an_error_naming_it() {
        let inv =
            Invocation { program: "sensei-no-such-compiler".into(), args: vec![], cwd: ".".into() };
        let err = run(&inv, std::time::Duration::from_secs(5)).await.unwrap_err();
        assert!(err.contains("sensei-no-such-compiler"), "{err}");
    }
}
