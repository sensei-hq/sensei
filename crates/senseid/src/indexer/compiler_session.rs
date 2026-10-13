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

/// One compiled project: its root, what its compiler resolved, and the state
/// of the sources it was compiled from.
pub struct Project {
    pub root: PathBuf,
    pub index: CompilerIndex,
    pub fingerprint: Fingerprint,
}

type Shared<T> = Arc<tokio::sync::OnceCell<Result<Arc<T>, String>>>;

/// What a project's sources were when it was compiled: every file the
/// repository does not ignore, by path, length and modification time.
type Fingerprint = String;

/// An engine's manifest and a directory: what a lookup is keyed by.
type Key = (&'static str, PathBuf);

/// One project's compiles: the one for the latest state of its sources, the
/// last one that finished, and the turn every compile of it takes.
#[derive(Default)]
struct Compiles {
    fingerprint: Fingerprint,
    latest: Shared<Project>,
    finished: Option<Arc<Project>>,
    /// Held while a compile of this project runs: a compile is a whole
    /// compiler's memory, and two of one project never run together.
    turn: Arc<tokio::sync::Mutex<()>>,
}

/// The compiler answers of one repository, kept across its scans. A project is
/// compiled once per state of its sources: asking again while nothing changed
/// shares the compile (however many gates of however many scans ask), and a
/// change compiles it afresh.
pub struct CompilerSessions {
    repo_root: PathBuf,
    /// package directory → the project root its build tool names.
    located: Mutex<HashMap<Key, Result<Arc<PathBuf>, String>>>,
    /// project root → its compiles.
    compiled: Mutex<HashMap<Key, Compiles>>,
    /// Definition files read back through their adapters, since the last compile.
    declared: Mutex<HashMap<PathBuf, Result<Arc<FileFacts>, Reason>>>,
    /// project root → its sources' state as last looked at, and when.
    looked: Mutex<HashMap<Key, (std::time::Instant, Fingerprint)>>,
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
            looked: Mutex::new(HashMap::new()),
        }
    }

    /// Whether `project` still describes its sources — nothing its compiler
    /// reads changed since it was compiled. A file placed against a compile
    /// that is not current would be placed against positions and declarations
    /// that moved. The sources are looked at once per [`LOOK_AGAIN`] per
    /// project, however many file tasks ask.
    pub async fn is_current(
        &self,
        engine: &'static dyn CompilerEngine,
        project: &Project,
    ) -> Result<bool, String> {
        let key = (engine.manifest(), project.root.clone());
        let recent = self
            .looked
            .lock()
            .map_err(|_| poisoned())?
            .get(&key)
            .filter(|(at, _)| at.elapsed() < LOOK_AGAIN)
            .map(|(_, seen)| seen.clone());
        let now = match recent {
            Some(seen) => seen,
            None => {
                let seen = fingerprint(&project.root, engine.sources()).await?;
                self.looked
                    .lock()
                    .map_err(|_| poisoned())?
                    .insert(key, (std::time::Instant::now(), seen.clone()));
                seen
            }
        };
        Ok(now == project.fingerprint)
    }

    /// [`Self::is_current`], looking at the sources now.
    #[cfg(test)]
    pub async fn is_current_now(
        &self,
        engine: &'static dyn CompilerEngine,
        project: &Project,
    ) -> Result<bool, String> {
        self.looked.lock().map_err(|_| poisoned())?.clear();
        self.is_current(engine, project).await
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// The compiled project the package at `package_dir` belongs to. An `Err`
    /// carries why — the build tool or the compiler failed, or is missing — and
    /// the caller's calls then say `engine_not_run`.
    #[cfg(test)]
    pub async fn project_for(
        &self,
        engine: &'static dyn CompilerEngine,
        package_dir: &Path,
    ) -> Result<Arc<Project>, String> {
        let root = self.locate_root(engine, package_dir).await?;
        self.compile_root(engine, &root).await
    }

    /// Compile the projects `packages` belong to: every package located first,
    /// then each project they name compiled ONCE — however many of its
    /// packages ask, and whatever changes on disk between them. One answer per
    /// project (by root), or per package that could not be located (by its
    /// directory).
    pub async fn compile_packages(
        &self,
        packages: Vec<(&'static dyn CompilerEngine, PathBuf)>,
    ) -> Vec<(PathBuf, Result<Arc<Project>, String>)> {
        let mut out = Vec::new();
        let mut projects: std::collections::BTreeMap<Key, &'static dyn CompilerEngine> =
            std::collections::BTreeMap::new();
        for (engine, dir) in packages {
            match self.locate_root(engine, &dir).await {
                Ok(root) => {
                    projects.entry((engine.manifest(), (*root).clone())).or_insert(engine);
                }
                Err(e) => out.push((dir, Err(e))),
            }
        }
        for ((_, root), engine) in projects {
            let compiled = self.compile_root(engine, &root).await;
            out.push((root, compiled));
        }
        out
    }

    /// The project root the package at `package_dir` belongs to, as its build
    /// tool names it — and remembered for the package's file tasks.
    async fn locate_root(
        &self,
        engine: &'static dyn CompilerEngine,
        package_dir: &Path,
    ) -> Result<Arc<PathBuf>, String> {
        // Asked afresh each time: a package can move into another workspace.
        let located = async {
            let out = run(&engine.locate(&package_dir.to_string_lossy()), LOCATE_TIMEOUT).await?;
            engine
                .project_root(&out)
                .map(|r| Arc::new(canonical(Path::new(&r))))
                .ok_or_else(|| format!("no project root in {out:?}"))
        }
        .await;
        self.located
            .lock()
            .map_err(|_| "compiler session cache poisoned".to_string())?
            .insert((engine.manifest(), package_dir.to_path_buf()), located.clone());
        located
    }

    /// The compile of the project at `root` as its sources are now.
    async fn compile_root(
        &self,
        engine: &'static dyn CompilerEngine,
        root: &Path,
    ) -> Result<Arc<Project>, String> {
        let fingerprint = fingerprint(root, engine.sources()).await?;
        let key = (engine.manifest(), root.to_path_buf());
        let (cell, turn) = self.compile_cell(key.clone(), &fingerprint)?;
        let project = cell
            .get_or_init(|| async {
                let _turn = turn.lock().await;
                compile(engine, root, &fingerprint).await.map(Arc::new)
            })
            .await
            .clone()?;
        if let Some(compiles) = self.compiled.lock().map_err(|_| poisoned())?.get_mut(&key) {
            compiles.finished = Some(project.clone());
        }
        Ok(project)
    }

    /// The compile of this state of the project's sources — the one already
    /// made or under way when nothing changed, a fresh one when something did —
    /// and the project's turn to compile.
    fn compile_cell(
        &self,
        key: Key,
        fingerprint: &str,
    ) -> Result<(Shared<Project>, Arc<tokio::sync::Mutex<()>>), String> {
        let mut compiled = self.compiled.lock().map_err(|_| poisoned())?;
        let compiles = compiled.entry(key).or_default();
        if compiles.fingerprint != fingerprint {
            compiles.fingerprint = fingerprint.to_string();
            compiles.latest = Shared::default();
            // A definition file read before the change may have changed with it.
            self.declared.lock().map_err(|_| poisoned())?.clear();
        }
        Ok((compiles.latest.clone(), compiles.turn.clone()))
    }

    /// The project `package_dir` belongs to, as its latest finished compile
    /// left it. Never compiles: that is `CompileProjects`' job, ahead of the
    /// files, so a file task never waits on — or, when abandoned, cancels — a
    /// compile.
    pub async fn compiled_for(
        &self,
        engine: &'static dyn CompilerEngine,
        package_dir: &Path,
    ) -> Result<Arc<Project>, String> {
        let located = self
            .located
            .lock()
            .map_err(|_| "compiler session cache poisoned".to_string())?
            .get(&(engine.manifest(), package_dir.to_path_buf()))
            .cloned();
        let root = match located {
            Some(found) => found?,
            None => return Err("this package's project was not compiled by this scan".to_string()),
        };
        let compiled = self.compiled.lock().map_err(|_| poisoned())?;
        let compiles = compiled.get(&(engine.manifest(), (*root).clone()));
        // The latest state's compile once it is done; while it runs, the last
        // one that finished.
        match compiles.map(|c| (c.latest.get(), &c.finished)) {
            Some((Some(latest), _)) => latest.clone(),
            Some((None, Some(finished))) => Ok(finished.clone()),
            _ => Err("this package's project was not compiled by this scan".to_string()),
        }
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
    /// The sessions of this repository, shared by all its file tasks and kept
    /// across its scans: a gate that fires again finds the compile it started.
    pub fn shared(folder_id: &uuid::Uuid, repo_root: &Path) -> Result<Arc<Self>, String> {
        let mut scans =
            scans().lock().map_err(|_| "compiler session cache poisoned".to_string())?;
        Ok(scans.entry(*folder_id).or_insert_with(|| Arc::new(Self::new(repo_root))).clone())
    }
}

/// How long a look at a project's sources stands for its current state.
const LOOK_AGAIN: std::time::Duration = std::time::Duration::from_secs(2);

/// The state of the sources under `root`: every file the repository does not
/// ignore whose extension is one of `sources`, by path, length and
/// modification time.
async fn fingerprint(root: &Path, sources: &'static [&'static str]) -> Result<Fingerprint, String> {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        use sha2::Digest;
        let mut files = Vec::new();
        for entry in ignore::WalkBuilder::new(&root).build() {
            let entry = entry.map_err(|e| format!("cannot walk {root:?}: {e}"))?;
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let read = entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| sources.contains(&e));
            if !read {
                continue;
            }
            let meta = entry.metadata().map_err(|e| format!("cannot read {entry:?}: {e}"))?;
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos());
            files.push(format!("{}\0{}\0{modified}\n", entry.path().display(), meta.len()));
        }
        files.sort();
        let mut hash = sha2::Sha256::new();
        for file in files {
            hash.update(file.as_bytes());
        }
        Ok(format!("{:x}", hash.finalize()))
    })
    .await
    .map_err(|e| format!("the fingerprint of the sources failed: {e}"))?
}

fn poisoned() -> String {
    "compiler session cache poisoned".to_string()
}

/// A path with its symlinks resolved, or as given when it cannot be (a path
/// that does not exist resolves to nothing, and is compared as written).
pub fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Compile `root` into a SCIP index and read it.
async fn compile(
    engine: &'static dyn CompilerEngine,
    root: &Path,
    fingerprint: &str,
) -> Result<Project, String> {
    let out_dir = std::env::temp_dir().join("sensei-compiler");
    tokio::fs::create_dir_all(&out_dir)
        .await
        .map_err(|e| format!("cannot create {out_dir:?}: {e}"))?;
    let name: String = root
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .chain(".".chars())
        .chain(fingerprint.chars().take(16))
        .collect();
    let root_str = root.to_string_lossy();
    let compiler = run(&engine.locate_compiler(&root_str), LOCATE_TIMEOUT).await?;
    let compiler = compiler.trim();
    if compiler.is_empty() {
        return Err("the compiler's binary could not be located".to_string());
    }
    // One compile per configuration (per platform), merged: code gated to a
    // platform is answered by that platform's compile.
    let mut merged: Option<CompilerIndex> = None;
    for (n, text) in engine.configs().iter().enumerate() {
        let config = out_dir.join(format!("{name}.{n}.config.json"));
        tokio::fs::write(&config, text)
            .await
            .map_err(|e| format!("cannot write the compiler's configuration {config:?}: {e}"))?;
        let output = out_dir.join(format!("{name}.{n}.scip"));
        let invocation = engine.compile(
            compiler,
            &root_str,
            &output.to_string_lossy(),
            &config.to_string_lossy(),
        );
        run(&invocation, COMPILE_TIMEOUT).await?;
        let bytes = tokio::fs::read(&output)
            .await
            .map_err(|e| format!("compiler wrote no index at {output:?}: {e}"))?;
        let index = CompilerIndex::from_bytes(&bytes)?;
        match merged.as_mut() {
            Some(all) => all.merge(index),
            None => merged = Some(index),
        }
    }
    let index = merged.ok_or_else(|| "the engine declares no configuration".to_string())?;
    Ok(Project { root: root.to_path_buf(), index, fingerprint: fingerprint.to_string() })
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
        let holds =
            |span: &crate::indexer::facts::Span| span.start_line <= line && line <= span.end_line;
        let declared = facts
            .symbols
            .iter()
            .filter(|s| s.name == name && holds(&s.span))
            .max_by_key(|s| s.span.start_line)
            .map(|s| s.fqn.clone());
        match declared {
            Some(fqn) => Ok(fqn),
            // Defined inside a macro invocation: a declaration the macro
            // generates, which the walk reads as text and never expands.
            None if facts.references.iter().any(|r| {
                r.kind == crate::indexer::facts::RefKind::MacroInvokes && holds(&r.at)
            }) =>
            {
                Err(Reason::MacroExpansion)
            }
            None => Err(Reason::DeclarationNotFound),
        }
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
            fingerprint: String::new(),
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

    /// A declaration a MACRO generates (`schema_vocabulary! { Kind { … } }`)
    /// is defined, the compiler says, inside the invocation — which the walk
    /// reads as text and does not expand. That is `macro_expansion`, the
    /// cause, not a declaration the walk failed to find.
    #[test]
    fn a_declaration_a_macro_generates_is_macro_expansion() {
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
            "macro_rules! make {\n    ($n:ident) => {\n        pub struct $n;\n    };\n}\n\nmake! {\n    Thing\n}\n",
        )
        .unwrap();
        let sessions = CompilerSessions::new(root);
        let project = Project {
            root: root.to_path_buf(),
            index: CompilerIndex::from_index(&scip::types::Index::new()),
            fingerprint: String::new(),
        };
        let decl = sessions.declarations(&project);
        assert_eq!(decl.declared_at("src/lib.rs", 8, "Thing"), Err(Reason::MacroExpansion));
        assert_eq!(decl.declared_at("src/lib.rs", 1, "Gone"), Err(Reason::DeclarationNotFound));
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
            fingerprint: String::new(),
        };
        let got = sessions.declarations(&project).declared_at("src/lib.rs", 2, "listener").unwrap();
        assert!(got.as_str().ends_with("listener·listener·item"), "got {got:?}");
    }

    /// An engine that counts its compiles: `sh` writes an empty index and
    /// appends a line to a counter file.
    /// Logs `start` and `end` of each compile to `counter`, `pause` seconds apart.
    struct Counting {
        counter: std::path::PathBuf,
        pause: &'static str,
    }
    impl CompilerEngine for Counting {
        fn manifest(&self) -> &'static str {
            "Cargo.toml"
        }
        fn locate(&self, package_dir: &str) -> Invocation {
            Invocation {
                program: "sh".into(),
                args: vec!["-c".into(), format!("echo {package_dir}/Cargo.toml")],
                cwd: package_dir.into(),
            }
        }
        fn project_root(&self, located: &str) -> Option<String> {
            located.trim().strip_suffix("/Cargo.toml").map(String::from)
        }
        fn locate_compiler(&self, root: &str) -> Invocation {
            Invocation {
                program: "sh".into(),
                args: vec!["-c".into(), "echo sh".into()],
                cwd: root.into(),
            }
        }
        fn sources(&self) -> &'static [&'static str] {
            &["rs", "toml"]
        }
        fn configs(&self) -> &'static [&'static str] {
            &["{}"]
        }
        fn compile(&self, _compiler: &str, root: &str, output: &str, _config: &str) -> Invocation {
            Invocation {
                program: "sh".into(),
                args: vec![
                    "-c".into(),
                    format!(
                        "echo start >> '{c}'; sleep {p}; : > '{output}'; echo end >> '{c}'",
                        c = self.counter.display(),
                        p = self.pause,
                    ),
                ],
                cwd: root.into(),
            }
        }
        fn target(&self, _: &str, _: &CompilerIndex, _: &dyn Declarations) -> Result<Fqn, Reason> {
            Err(Reason::SymbolUnnamed)
        }
    }

    /// **A COMPILE IS KEYED BY WHAT IT COMPILES, NOT BY WHO ASKS.** Gates fire
    /// several times a scan; each used to wipe the sessions and compile again,
    /// concurrently. An unchanged project now reuses its compile; a changed one
    /// compiles again; and a file task reads the latest.
    #[tokio::test]
    async fn an_unchanged_project_is_compiled_once_and_a_changed_one_again() {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        std::fs::write(root.join("lib.rs"), "pub fn a() {}\n").unwrap();
        // Outside the project: a write inside it is a change to its sources.
        let elsewhere = tempfile::tempdir().unwrap();
        let counter = elsewhere.path().join("compiles.txt");
        let engine: &'static Counting =
            Box::leak(Box::new(Counting { counter: counter.clone(), pause: "0" }));
        let sessions = CompilerSessions::new(&root);
        let runs = || {
            std::fs::read_to_string(&counter)
                .map(|t| t.lines().filter(|l| *l == "end").count())
                .unwrap_or(0)
        };

        sessions.project_for(engine, &root).await.expect("compiles");
        sessions.project_for(engine, &root).await.expect("reuses");
        assert_eq!(runs(), 1, "an unchanged project is compiled once");
        assert!(sessions.compiled_for(engine, &root).await.is_ok(), "a file task reads it");

        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(root.join("lib.rs"), "pub fn a() {}\npub fn b() {}\n").unwrap();
        sessions.project_for(engine, &root).await.expect("recompiles");
        assert_eq!(runs(), 2, "a changed project is compiled again");
    }

    /// **A COMPILE DESCRIBES THE SOURCES IT SAW, AND KNOWS WHEN THEY MOVED ON.**
    /// A file edited after its project was compiled was placed against the old
    /// answers and marked parsed — 226 calls of an edited file left
    /// `declaration_not_found`, for good. `is_current` says whether the
    /// project still is what was compiled; only the sources its compiler reads
    /// count, so a note beside the code does not make it stale.
    #[tokio::test]
    async fn a_compile_is_current_until_a_source_it_read_changes() {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        std::fs::write(root.join("lib.rs"), "pub fn a() {}\n").unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let engine: &'static Counting = Box::leak(Box::new(Counting {
            counter: elsewhere.path().join("compiles.txt"),
            pause: "0",
        }));
        let sessions = CompilerSessions::new(&root);
        let project = sessions.project_for(engine, &root).await.expect("compiles");
        assert_eq!(sessions.is_current(engine, &project).await, Ok(true));

        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(root.join("NOTES.md"), "not a source\n").unwrap();
        assert_eq!(sessions.is_current_now(engine, &project).await, Ok(true), "not a source");

        std::fs::write(root.join("lib.rs"), "pub fn a() {}\npub fn b() {}\n").unwrap();
        assert_eq!(sessions.is_current_now(engine, &project).await, Ok(false), "a source moved on");
    }

    /// **A NEWER COMPILE NEVER BLANKS AN OLDER ONE, NOR RUNS BESIDE IT.** While
    /// a changed project compiles again, a file task reads the compile that
    /// finished last — not "not compiled"; and two compiles of one project run
    /// one after the other, never together (each is a whole compiler's memory).
    #[tokio::test]
    async fn a_recompile_waits_its_turn_and_files_read_the_last_one_finished() {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        std::fs::write(root.join("lib.rs"), "pub fn a() {}\n").unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let counter = elsewhere.path().join("compiles.txt");
        let engine: &'static Counting =
            Box::leak(Box::new(Counting { counter: counter.clone(), pause: "0.6" }));
        let sessions = Arc::new(CompilerSessions::new(&root));
        sessions.project_for(engine, &root).await.expect("first compile");

        let change = |text: &str| {
            std::thread::sleep(std::time::Duration::from_millis(20));
            std::fs::write(root.join("lib.rs"), text).unwrap();
        };
        change("pub fn a() {}\npub fn b() {}\n");
        let (s, r) = (sessions.clone(), root.clone());
        let second = tokio::spawn(async move { s.project_for(engine, &r).await.map(|_| ()) });
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert!(
            sessions.compiled_for(engine, &root).await.is_ok(),
            "a file task reads the compile that finished while the next one runs"
        );
        change("pub fn a() {}\npub fn b() {}\npub fn c() {}\n");
        let (s, r) = (sessions.clone(), root.clone());
        let third = tokio::spawn(async move { s.project_for(engine, &r).await.map(|_| ()) });
        second.await.unwrap().expect("second compile");
        third.await.unwrap().expect("third compile");

        let log = std::fs::read_to_string(&counter).unwrap();
        let log: Vec<&str> = log.lines().collect();
        assert_eq!(log, ["start", "end", "start", "end", "start", "end"], "one at a time");
    }

    /// **WHAT TYPESCRIPT WRITES AROUND A CALL DOES NOT HIDE ITS TARGET.**
    /// `super(…)` calls the base class; a name destructured out of a module
    /// (`const { bind } = await import('./net')`) is that module's
    /// declaration; a Svelte component's prop is its parameter, so a call
    /// through it is a local's and no edge.
    #[tokio::test]
    async fn the_checker_sees_through_super_destructuring_and_props() {
        use crate::indexer::facts::{RefKind, Resolution, Rung};
        use crate::indexer::lang::compiler::{Compiled, place_with};
        use crate::indexer::lang::ts_compiler::TS_ENGINE;

        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("package.json", r#"{"name": "web", "type": "module"}"#);
        write(
            "tsconfig.json",
            r#"{"compilerOptions": {"strict": true, "module": "esnext", "moduleResolution": "bundler", "target": "es2022"}}"#,
        );
        write("src/net.ts", "export function bind(port: number) {\n  return port;\n}\n");
        write("src/base.ts", "export class Base {\n  constructor(public n: number) {}\n}\n");
        write(
            "src/kid.ts",
            "import { Base } from './base';\n\nexport class Kid extends Base {\n  constructor() {\n    super(1);\n  }\n}\n\nexport class Oops extends Error {\n  constructor() {\n    super('x');\n  }\n}\n",
        );
        write(
            "src/dyn.ts",
            "export async function run() {\n  const { bind } = await import('./net');\n  return bind(2);\n}\n",
        );
        write(
            "src/Button.svelte",
            "<script lang=\"ts\">\n  let { onSave }: { onSave: () => void } = $props();\n  function save() {\n    onSave();\n  }\n</script>\n\n<button onclick={save}>save</button>\n",
        );

        let sessions = CompilerSessions::new(&root);
        let project = sessions.project_for(&TS_ENGINE, &root).await.expect("the driver compiles");
        let declarations = sessions.declarations(&project);
        let mut placed = Vec::new();
        for rel in ["src/kid.ts", "src/dyn.ts", "src/Button.svelte"] {
            let facts = read_declarations(&root, &root.join(rel)).expect(rel);
            let text = std::fs::read_to_string(root.join(rel)).unwrap();
            let compiled =
                Compiled { index: &project.index, path: rel, declarations: &declarations };
            let calls: Vec<_> =
                facts.references.iter().filter(|r| r.kind == RefKind::Calls).cloned().collect();
            for call in place_with(&TS_ENGINE, Some(&compiled), calls, &text) {
                placed.push((rel, call.at.start_line, call.target));
            }
        }
        let to = |rel: &str, line: u32| {
            placed
                .iter()
                .filter(|(r, l, _)| *r == rel && *l == line)
                .map(|(_, _, t)| t.clone())
                .collect::<Vec<_>>()
        };
        let resolved = |fqn: Fqn| vec![Resolution::Resolved { fqn, via: Rung::ResolvedByCompiler }];
        let base = declarations.declared_at("src/base.ts", 1, "Base").expect("Base is declared");
        let bind = declarations.declared_at("src/net.ts", 1, "bind").expect("bind is declared");
        let error = crate::indexer::fqn::refer(&crate::indexer::fqn::Form::Lib {
            package: "ecmascript",
            member: "Error",
        })
        .unwrap();
        assert_eq!(to("src/kid.ts", 5), resolved(base), "super is the base class");
        assert_eq!(to("src/kid.ts", 11), resolved(error), "a library base class is its lib node");
        assert_eq!(to("src/dyn.ts", 3), resolved(bind), "a destructured import is its declaration");
        assert_eq!(to("src/Button.svelte", 4), vec![], "a prop is a parameter: no edge");
    }

    /// **WHAT THE CHECKER KNOWS WITHOUT A DECLARATION IS STILL AN ANSWER.**
    /// A member of a mapped type (`fireEvent.click`) has no declaration of its
    /// own — it is named by its type. A shorthand property (`{ loadCatalog }`)
    /// is the function it names. A Svelte snippet is declared in the markup, a
    /// template's local: no edge. And a receiver typed `any` names nothing —
    /// which is said, as `untyped_receiver`.
    #[tokio::test]
    async fn the_checker_names_members_shorthands_snippets_and_untyped_receivers() {
        use crate::indexer::facts::{RefKind, Resolution, Rung};
        use crate::indexer::lang::compiler::{Compiled, place_with};
        use crate::indexer::lang::ts_compiler::TS_ENGINE;

        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("package.json", r#"{"name": "web", "type": "module"}"#);
        write(
            "tsconfig.json",
            r#"{"compilerOptions": {"strict": true, "module": "esnext", "moduleResolution": "bundler", "target": "es2022", "allowJs": true}}"#,
        );
        write("node_modules/firelib/package.json", r#"{"name": "firelib", "types": "index.d.ts"}"#);
        write(
            "node_modules/firelib/index.d.ts",
            "export type FireObject = { [K in 'click' | 'input']: (el: number) => Promise<boolean> };\nexport type FireFunction = (el: number) => boolean;\nexport declare const fireEvent: FireFunction & FireObject;\n",
        );
        write(
            "src/fire.ts",
            "import { fireEvent } from 'firelib';\n\nexport async function press() {\n  await fireEvent.click(1);\n}\n",
        );
        write(
            "src/store.ts",
            "function loadCatalog() {\n  return 1;\n}\n\nexport const mcp = { loadCatalog };\n",
        );
        write(
            "src/use.ts",
            "import { mcp } from './store';\n\nexport function go() {\n  return mcp.loadCatalog();\n}\n",
        );
        write(
            "src/Card.svelte",
            "<script lang=\"ts\">\n  let n = 1;\n</script>\n\n{#snippet stat(x: number)}\n  <b>{x}</b>\n{/snippet}\n\n{@render stat(n)}\n",
        );
        write("src/raw.js", "export function go(x) {\n  return x.whatever();\n}\n");
        write(
            "src/missing.ts",
            "import { make } from 'not-installed';\n\nexport function go() {\n  return make();\n}\n",
        );

        let sessions = CompilerSessions::new(&root);
        let project = sessions.project_for(&TS_ENGINE, &root).await.expect("the driver compiles");
        let declarations = sessions.declarations(&project);
        let mut seen = Vec::new();
        let mut placed = Vec::new();
        for rel in ["src/fire.ts", "src/use.ts", "src/Card.svelte", "src/raw.js", "src/missing.ts"]
        {
            let facts = read_declarations(&root, &root.join(rel)).expect(rel);
            let text = std::fs::read_to_string(root.join(rel)).unwrap();
            let compiled =
                Compiled { index: &project.index, path: rel, declarations: &declarations };
            let calls: Vec<_> =
                facts.references.iter().filter(|r| r.kind == RefKind::Calls).cloned().collect();
            seen.extend(calls.iter().map(|c| (rel, c.at.start_line)));
            for call in place_with(&TS_ENGINE, Some(&compiled), calls, &text) {
                placed.push((rel, call.at.start_line, call.target));
            }
        }
        let to = |rel: &str, line: u32| {
            placed
                .iter()
                .filter(|(r, l, _)| *r == rel && *l == line)
                .map(|(_, _, t)| t.clone())
                .collect::<Vec<_>>()
        };
        let resolved = |fqn: Fqn| vec![Resolution::Resolved { fqn, via: Rung::ResolvedByCompiler }];
        let click = crate::indexer::fqn::refer(&crate::indexer::fqn::Form::Lib {
            package: "firelib",
            member: "FireObject.click",
        })
        .unwrap();
        let load = declarations.declared_at("src/store.ts", 1, "loadCatalog").expect("declared");
        assert_eq!(to("src/fire.ts", 4), resolved(click), "a mapped member is named by its type");
        assert_eq!(to("src/use.ts", 4), resolved(load), "a shorthand is the function it names");
        assert!(
            seen.contains(&("src/Card.svelte", 9)),
            "the walk saw the snippet's call: {seen:?}"
        );
        assert_eq!(to("src/Card.svelte", 9), vec![], "a snippet is a template's local");
        assert!(
            matches!(
                to("src/raw.js", 2).as_slice(),
                [Resolution::Unresolved { reason: Reason::UntypedReceiver, .. }]
            ),
            "{:?}",
            to("src/raw.js", 2)
        );
        assert!(
            matches!(
                to("src/missing.ts", 4).as_slice(),
                [Resolution::Unresolved { reason: Reason::UntypedReceiver, .. }]
            ),
            "a name imported from a module the checker cannot find is untyped: {:?}",
            to("src/missing.ts", 4)
        );
    }

    /// **WHAT A CALL REACHES THROUGH A VALUE IS NAMED BY WHAT DECLARES IT.**
    /// A `declare module 'x'` in the project declares a LIBRARY's shape (a
    /// virtual module, a shim), so its members are `lib·x·…` (R5). A property
    /// whose value is computed (`{ publish: fn() }`) is called through the
    /// value's type. And a member of an anonymous object type (`Array<{ run:
    /// () => … }>`) is a value, not a declaration — calling it calls whatever
    /// function the value holds, as calling a parameter does: no edge.
    #[tokio::test]
    async fn the_checker_names_ambient_modules_values_and_anonymous_members() {
        use crate::indexer::facts::{RefKind, Resolution, Rung};
        use crate::indexer::lang::compiler::{Compiled, place_with};
        use crate::indexer::lang::ts_compiler::TS_ENGINE;

        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("package.json", r#"{"name": "web", "type": "module"}"#);
        write(
            "tsconfig.json",
            r#"{"compilerOptions": {"strict": true, "module": "esnext", "moduleResolution": "bundler", "target": "es2022"}}"#,
        );
        write("node_modules/spylib/package.json", r#"{"name": "spylib", "types": "index.d.ts"}"#);
        write(
            "node_modules/spylib/index.d.ts",
            "export interface Mock {\n  (...a: unknown[]): unknown;\n}\nexport declare function fn(): Mock;\n",
        );
        write(
            "src/shim.d.ts",
            "declare module 'kitlib' {\n  export const kit: { configure: (o?: {}) => void };\n  export class Item {\n    get(k: string): unknown;\n  }\n  // eslint-disable-next-line\n  export const loose: any;\n}\n",
        );
        write(
            "src/use-loose.ts",
            "import { loose } from 'kitlib';\n\nexport function go() {\n  return loose.registry.add(1);\n}\n",
        );
        write(
            "src/use-kit.ts",
            "import { kit, Item } from 'kitlib';\n\nexport function go(i: Item) {\n  kit.configure();\n  return i.get('x');\n}\n",
        );
        write(
            "src/routes.ts",
            "const routes: Array<{ name: string; run: () => number }> = [{ name: 'a', run: () => 1 }];\n\nexport function all() {\n  return routes.map((r) => r.run());\n}\n",
        );
        write(
            "src/spies.ts",
            "import { fn } from 'spylib';\n\nconst spies = { publish: fn() };\n\nexport function go() {\n  return spies.publish(1);\n}\n",
        );
        write(
            "src/store.ts",
            "export const store = {\n  load() {\n    return 1;\n  },\n};\n\nexport function go() {\n  return store.load();\n}\n",
        );

        let sessions = CompilerSessions::new(&root);
        let project = sessions.project_for(&TS_ENGINE, &root).await.expect("the driver compiles");
        let declarations = sessions.declarations(&project);
        let mut placed = Vec::new();
        for rel in
            ["src/use-kit.ts", "src/routes.ts", "src/spies.ts", "src/store.ts", "src/use-loose.ts"]
        {
            let facts = read_declarations(&root, &root.join(rel)).expect(rel);
            let text = std::fs::read_to_string(root.join(rel)).unwrap();
            let compiled =
                Compiled { index: &project.index, path: rel, declarations: &declarations };
            let calls: Vec<_> =
                facts.references.iter().filter(|r| r.kind == RefKind::Calls).cloned().collect();
            for call in place_with(&TS_ENGINE, Some(&compiled), calls, &text) {
                placed.push((rel, call.at.start_line, call.target));
            }
        }
        let lib = |package: &str, member: &str| {
            vec![Resolution::Resolved {
                fqn: crate::indexer::fqn::refer(&crate::indexer::fqn::Form::Lib {
                    package,
                    member,
                })
                .unwrap(),
                via: Rung::ResolvedByCompiler,
            }]
        };
        let to = |rel: &str, line: u32, name: &str| {
            placed
                .iter()
                .filter(|(r, l, t)| {
                    *r == rel
                        && *l == line
                        && match t {
                            Resolution::Resolved { fqn, .. } => fqn.as_str().contains(name),
                            Resolution::Unresolved { evidence, .. } => {
                                evidence.name.ends_with(name)
                            }
                        }
                })
                .map(|(_, _, t)| t.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            to("src/use-kit.ts", 4, "configure"),
            lib("kitlib", "kit.configure"),
            "an ambient module is a library"
        );
        assert_eq!(to("src/use-kit.ts", 5, "get"), lib("kitlib", "Item.get"));
        assert_eq!(
            to("src/use-loose.ts", 4, "add"),
            lib("kitlib", "loose.registry.add"),
            "a library binding typed any is still that library's member, named by its path"
        );
        assert_eq!(
            to("src/spies.ts", 6, "Mock"),
            lib("spylib", "Mock"),
            "a computed value is called through its type"
        );
        let load = declarations.declared_at("src/store.ts", 2, "load").expect("a walked member");
        assert_eq!(
            to("src/store.ts", 8, "load"),
            vec![Resolution::Resolved { fqn: load, via: Rung::ResolvedByCompiler }],
            "a function of a module-level object is its member, not a dynamic call"
        );
        assert_eq!(to("src/routes.ts", 4, "run"), vec![], "a value's member is a local's: no edge");
    }

    /// **RUST'S REFERENCES, BY RUST-ANALYZER.** A field read and a type use
    /// reach their declarations; a library type is its `lib·` node; a generic
    /// parameter is the declaration's own local — no edge.
    #[tokio::test]
    async fn rust_references_are_placed_by_rust_analyzer() {
        use crate::indexer::facts::{RefKind, Resolution, Rung};
        use crate::indexer::lang::compiler::{Compiled, place_with};
        use crate::indexer::lang::rust::compiler::RUST_ENGINE;

        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n[workspace]\n",
        )
        .unwrap();
        let text = "pub struct Point {\n    pub x: u32,\n}\n\npub fn get<T>(p: &Point, _t: T) -> Vec<Point> {\n    let _n = p.x;\n    Vec::new()\n}\n";
        std::fs::write(root.join("src/lib.rs"), text).unwrap();

        let sessions = CompilerSessions::new(&root);
        let project =
            sessions.project_for(&RUST_ENGINE, &root).await.expect("rust-analyzer compiles");
        let declarations = sessions.declarations(&project);
        let facts = read_declarations(&root, &root.join("src/lib.rs")).expect("reads");
        let compiled =
            Compiled { index: &project.index, path: "src/lib.rs", declarations: &declarations };
        let references: Vec<_> = facts
            .references
            .iter()
            .filter(|r| matches!(r.kind, RefKind::Reads | RefKind::TypeUse))
            .cloned()
            .collect();
        let placed = place_with(&RUST_ENGINE, Some(&compiled), references, text);
        let unplaced: Vec<_> =
            placed.iter().filter(|r| matches!(r.target, Resolution::Unresolved { .. })).collect();
        assert!(unplaced.is_empty(), "every reference placed: {unplaced:#?}");
        let targets: Vec<String> = placed
            .iter()
            .filter_map(|r| match &r.target {
                Resolution::Resolved { fqn, via: Rung::ResolvedByCompiler } => {
                    Some(fqn.as_str().to_string())
                }
                _ => None,
            })
            .collect();
        let point = declarations.declared_at("src/lib.rs", 1, "Point").unwrap();
        let x = declarations.declared_at("src/lib.rs", 2, "x").unwrap();
        assert!(targets.contains(&point.as_str().to_string()), "{targets:?}");
        assert!(targets.contains(&x.as_str().to_string()), "{targets:?}");
        assert!(
            targets.iter().any(|t| t.starts_with("lib·alloc·") && t.contains("Vec")),
            "{targets:?}"
        );
        assert!(
            !targets.iter().any(|t| t.ends_with("·T·item") || t.contains("·T·")),
            "a generic parameter is no edge: {targets:?}"
        );
    }

    /// **WHAT A REFERENCE READS THROUGH DECIDES WHETHER IT IS AN EDGE.** An
    /// index signature's entry is a value, not a declaration: no edge. A
    /// member declared in `declare global` (SvelteKit's `App.Platform`) is a
    /// declaration of this project. A template tag the walk records whole
    /// names every symbol in it.
    #[tokio::test]
    async fn references_through_index_signatures_globals_and_template_tags() {
        use crate::indexer::facts::{RefKind, Resolution, Rung};
        use crate::indexer::lang::compiler::{Compiled, place_with};
        use crate::indexer::lang::ts_compiler::TS_ENGINE;

        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("package.json", r#"{"name": "web", "type": "module"}"#);
        write(
            "tsconfig.json",
            r#"{"compilerOptions": {"strict": true, "module": "esnext", "moduleResolution": "bundler", "target": "es2022"}}"#,
        );
        write(
            "src/db.ts",
            "export interface Row {\n  [column: string]: unknown;\n}\n\nexport function first(r: Row) {\n  return r.claimed_at;\n}\n",
        );
        write(
            "src/app.d.ts",
            "declare global {\n  namespace App {\n    interface Platform {\n      context?: { waitUntil(p: Promise<unknown>): void };\n    }\n  }\n}\n\nexport {};\n",
        );
        write(
            "src/use-platform.ts",
            "export function wait(p: App.Platform) {\n  return p.context;\n}\n",
        );
        write(
            "src/Trend.svelte",
            "<script lang=\"ts\">\n  import Chart from './Chart.svelte';\n  const TREND_WINDOW = 7;\n</script>\n\n<Chart options={{ type: 'ma', window: TREND_WINDOW }} />\n",
        );
        write(
            "src/Chart.svelte",
            "<script lang=\"ts\">\n  let { options }: { options: { type: string; window: number } } = $props();\n</script>\n\n<p>{options.type}</p>\n",
        );

        let sessions = CompilerSessions::new(&root);
        let project = sessions.project_for(&TS_ENGINE, &root).await.expect("the driver compiles");
        let declarations = sessions.declarations(&project);
        let mut placed = Vec::new();
        for rel in ["src/db.ts", "src/use-platform.ts", "src/Trend.svelte"] {
            let facts = read_declarations(&root, &root.join(rel)).expect(rel);
            let text = std::fs::read_to_string(root.join(rel)).unwrap();
            let compiled =
                Compiled { index: &project.index, path: rel, declarations: &declarations };
            let references: Vec<_> = facts
                .references
                .iter()
                .filter(|r| {
                    matches!(
                        r.kind,
                        RefKind::Reads | RefKind::Writes | RefKind::TypeUse | RefKind::Constructs
                    )
                })
                .cloned()
                .collect();
            for r in place_with(&TS_ENGINE, Some(&compiled), references, &text) {
                placed.push((rel, r.at.start_line, r.target));
            }
        }
        let unplaced: Vec<_> =
            placed.iter().filter(|(_, _, t)| matches!(t, Resolution::Unresolved { .. })).collect();
        assert!(unplaced.is_empty(), "every reference placed or no edge: {unplaced:#?}");
        let at = |rel: &str, line: u32| {
            placed
                .iter()
                .filter(|(r, l, _)| *r == rel && *l == line)
                .filter_map(|(_, _, t)| match t {
                    Resolution::Resolved { fqn, via: Rung::ResolvedByCompiler } => {
                        Some(fqn.as_str().to_string())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            at("src/db.ts", 6),
            Vec::<String>::new(),
            "an index signature's entry is no edge"
        );
        let context = declarations
            .declared_at("src/app.d.ts", 4, "context")
            .expect("a global's member is a node");
        assert!(
            at("src/use-platform.ts", 2).contains(&context.as_str().to_string()),
            "{:?}",
            at("src/use-platform.ts", 2)
        );
        let window =
            declarations.declared_at("src/Trend.svelte", 3, "TREND_WINDOW").expect("declared");
        assert!(
            at("src/Trend.svelte", 6).contains(&window.as_str().to_string()),
            "{:?}",
            at("src/Trend.svelte", 6)
        );
    }

    /// **A REFERENCE IS PLACED BY THE CHECKER AS A CALL IS.** A type use, a
    /// construction and field reads reach their declarations — through a
    /// parameter's type and through a local's.
    #[tokio::test]
    async fn references_are_placed_by_the_checker() {
        use crate::indexer::facts::{RefKind, Resolution, Rung};
        use crate::indexer::lang::compiler::{Compiled, place_with};
        use crate::indexer::lang::ts_compiler::TS_ENGINE;

        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("package.json", r#"{"name": "web", "type": "module"}"#);
        write(
            "tsconfig.json",
            r#"{"compilerOptions": {"strict": true, "module": "esnext", "moduleResolution": "bundler", "target": "es2022"}}"#,
        );
        write(
            "src/conf.ts",
            "export class Box {\n  size = 1;\n}\n\nexport interface Opts {\n  n: number;\n  'x-key': string;\n}\n",
        );
        write(
            "src/use.ts",
            "import { Box, type Opts } from './conf';\n\nexport type Pair = Opts[];\n\nexport function pick(o: Opts, all: number[]) {\n  const b = new Box();\n  return o.n + b.size + o['x-key'].length + all[0];\n}\n",
        );

        let sessions = CompilerSessions::new(&root);
        let project = sessions.project_for(&TS_ENGINE, &root).await.expect("the driver compiles");
        let declarations = sessions.declarations(&project);
        let facts = read_declarations(&root, &root.join("src/use.ts")).expect("reads");
        let text = std::fs::read_to_string(root.join("src/use.ts")).unwrap();
        let compiled =
            Compiled { index: &project.index, path: "src/use.ts", declarations: &declarations };
        let references: Vec<_> = facts
            .references
            .iter()
            .filter(|r| {
                !matches!(r.kind, RefKind::Calls | RefKind::MacroInvokes | RefKind::Imports)
            })
            .cloned()
            .collect();
        assert!(!references.is_empty(), "the walk saw references");
        let placed = place_with(&TS_ENGINE, Some(&compiled), references, &text);
        for r in &placed {
            assert!(
                matches!(r.target, Resolution::Resolved { via: Rung::ResolvedByCompiler, .. }),
                "every reference the checker answers is placed: {r:?}"
            );
        }
        let targets: Vec<String> = placed
            .iter()
            .filter_map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => Some(fqn.as_str().to_string()),
                Resolution::Unresolved { .. } => None,
            })
            .collect();
        let mut wanted: Vec<String> =
            [("Box", 1), ("size", 2), ("Opts", 5), ("n", 6), ("x-key", 7)]
                .iter()
                .map(|(name, line)| {
                    declarations
                        .declared_at("src/conf.ts", *line, name)
                        .unwrap()
                        .as_str()
                        .to_string()
                })
                .collect();
        wanted.push(
            crate::indexer::fqn::refer(&crate::indexer::fqn::Form::Lib {
                package: "ecmascript",
                member: "String.length",
            })
            .unwrap()
            .as_str()
            .to_string(),
        );
        let mut got = targets.clone();
        wanted.sort();
        got.sort();
        got.dedup();
        assert_eq!(
            got, wanted,
            "the type use, the construction, the field reads (a literal key's too) and the \
             language's own `length`; `all[0]` is a value's element, no edge"
        );
    }

    /// **A SVELTEKIT PROJECT IS COMPILED AS ITS OWN TOOLING SEES IT.** Its
    /// tsconfig extends what `svelte-kit sync` generates (`$lib`, `$app`,
    /// `./$types`); a clean checkout, or a `make clean`, has none of it, and
    /// every name imported through `$lib` vanished. The driver runs the
    /// project's own `svelte-kit sync` first. Here the kit is a stand-in whose
    /// sync writes only the `$lib` path.
    #[tokio::test]
    async fn a_sveltekit_project_is_synced_before_it_is_compiled() {
        use crate::indexer::facts::{RefKind, Resolution, Rung};
        use crate::indexer::lang::compiler::{Compiled, place_with};
        use crate::indexer::lang::ts_compiler::TS_ENGINE;

        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("package.json", r#"{"name": "kitapp", "type": "module"}"#);
        write("svelte.config.js", "export default {};\n");
        write("tsconfig.json", r#"{"extends": "./.svelte-kit/tsconfig.json"}"#);
        write(
            "node_modules/@sveltejs/kit/package.json",
            r#"{"name": "@sveltejs/kit", "version": "0.0.0"}"#,
        );
        write(
            "node_modules/@sveltejs/kit/svelte-kit.js",
            "import fs from 'node:fs';\nif (process.argv[2] !== 'sync') process.exit(9);\nfs.mkdirSync('.svelte-kit', { recursive: true });\nfs.writeFileSync('.svelte-kit/tsconfig.json', JSON.stringify({ compilerOptions: { strict: true, module: 'esnext', moduleResolution: 'bundler', target: 'es2022', paths: { '$lib': ['../src/lib'], '$lib/*': ['../src/lib/*'] } }, include: ['../src/**/*.ts'] }));\n",
        );
        write("src/lib/net.ts", "export function bind(port: number) {\n  return port;\n}\n");
        write(
            "src/main.ts",
            "import { bind } from '$lib/net';\n\nexport function start() {\n  return bind(80);\n}\n",
        );

        let sessions = CompilerSessions::new(&root);
        let project = sessions.project_for(&TS_ENGINE, &root).await.expect("the driver compiles");
        let declarations = sessions.declarations(&project);
        let bind = declarations.declared_at("src/lib/net.ts", 1, "bind").expect("bind is declared");
        let facts = read_declarations(&root, &root.join("src/main.ts")).expect("main reads");
        let text = std::fs::read_to_string(root.join("src/main.ts")).unwrap();
        let compiled =
            Compiled { index: &project.index, path: "src/main.ts", declarations: &declarations };
        let calls: Vec<_> =
            facts.references.iter().filter(|r| r.kind == RefKind::Calls).cloned().collect();
        let placed = place_with(&TS_ENGINE, Some(&compiled), calls, &text);
        assert_eq!(
            placed.iter().map(|r| r.target.clone()).collect::<Vec<_>>(),
            vec![Resolution::Resolved { fqn: bind, via: Rung::ResolvedByCompiler }],
            "a call through `$lib` is placed once the project is synced"
        );
    }

    /// An engine whose packages all belong to the project at `root`, and whose
    /// compile touches that project — the way a developer's editor does while
    /// a scan runs.
    struct OneProject {
        root: std::path::PathBuf,
        counter: std::path::PathBuf,
    }
    impl CompilerEngine for OneProject {
        fn manifest(&self) -> &'static str {
            "Cargo.toml"
        }
        fn locate(&self, package_dir: &str) -> Invocation {
            Invocation {
                program: "sh".into(),
                args: vec!["-c".into(), format!("echo {}/Cargo.toml", self.root.display())],
                cwd: package_dir.into(),
            }
        }
        fn project_root(&self, located: &str) -> Option<String> {
            located.trim().strip_suffix("/Cargo.toml").map(String::from)
        }
        fn locate_compiler(&self, root: &str) -> Invocation {
            Invocation {
                program: "sh".into(),
                args: vec!["-c".into(), "echo sh".into()],
                cwd: root.into(),
            }
        }
        fn sources(&self) -> &'static [&'static str] {
            &["rs", "toml"]
        }
        fn configs(&self) -> &'static [&'static str] {
            &["{}"]
        }
        fn compile(&self, _compiler: &str, root: &str, output: &str, _config: &str) -> Invocation {
            Invocation {
                program: "sh".into(),
                args: vec![
                    "-c".into(),
                    format!(
                        ": > '{output}'; echo run >> '{}'; date +%N >> '{root}/edited.rs'",
                        self.counter.display()
                    ),
                ],
                cwd: root.into(),
            }
        }
        fn target(&self, _: &str, _: &CompilerIndex, _: &dyn Declarations) -> Result<Fqn, Reason> {
            Err(Reason::SymbolUnnamed)
        }
    }

    /// **ONE COMPILE OF A PROJECT PER TASK, HOWEVER MANY OF ITS PACKAGES ASK.**
    /// A workspace of nine crates was compiled up to nine times by one task:
    /// each package re-read the sources, and an edit between two of them made
    /// a "new" project. The task's packages are located first and each project
    /// they name is compiled once — that compile is the task's snapshot.
    #[tokio::test]
    async fn the_packages_of_one_project_share_one_compile() {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        std::fs::create_dir_all(root.join("a")).unwrap();
        std::fs::create_dir_all(root.join("b")).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let counter = elsewhere.path().join("compiles.txt");
        let engine: &'static OneProject =
            Box::leak(Box::new(OneProject { root: root.clone(), counter: counter.clone() }));
        let sessions = CompilerSessions::new(&root);

        let compiled = sessions
            .compile_packages(vec![
                (engine as &dyn CompilerEngine, root.join("a")),
                (engine, root.join("b")),
            ])
            .await;
        let roots: Vec<_> = compiled.iter().map(|(root, r)| (root, r.as_ref().err())).collect();
        assert_eq!(roots, [(&root, None)], "one project, compiled");
        let runs = std::fs::read_to_string(&counter).unwrap().lines().count();
        assert_eq!(runs, 1, "the project is compiled once for both of its packages");
        assert!(sessions.compiled_for(engine, &root.join("a")).await.is_ok());
        assert!(sessions.compiled_for(engine, &root.join("b")).await.is_ok());
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

    /// **THE CHECKER, END TO END.** A real TypeScript project with a Svelte
    /// component, compiled by the embedded driver, each file read by its own
    /// adapter and placed: every call the walk saw is placed by the checker —
    /// through a renamed import, into a component's template, onto the
    /// language's own `Array.prototype.map`.
    #[tokio::test]
    async fn a_typescript_and_svelte_project_is_placed_by_its_checker() {
        use crate::indexer::facts::{RefKind, Resolution, Rung};
        use crate::indexer::lang::compiler::{Compiled, place_with};
        use crate::indexer::lang::ts_compiler::TS_ENGINE;

        let dir = tempfile::tempdir().unwrap();
        let root = canonical(dir.path());
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("package.json", r#"{"name": "web", "type": "module"}"#);
        write(
            "tsconfig.json",
            r#"{"compilerOptions": {"strict": true, "module": "esnext", "moduleResolution": "bundler", "target": "es2022"}}"#,
        );
        write(
            "src/net.ts",
            "export function bind(port: number) {\n  return [port].map((p) => p + 1);\n}\n",
        );
        write(
            "src/main.ts",
            "import { bind as open } from './net';\n\nexport function start() {\n  return open(80);\n}\n",
        );
        write(
            "src/App.svelte",
            "<script lang=\"ts\">\n  import { bind } from './net';\n  function go() {\n    return bind(1);\n  }\n</script>\n\n<button onclick={() => bind(2)}>{go()}</button>\n",
        );

        let sessions = CompilerSessions::new(&root);
        let project = sessions.project_for(&TS_ENGINE, &root).await.expect("the driver compiles");
        let declarations = sessions.declarations(&project);
        let bind = declarations.declared_at("src/net.ts", 1, "bind").expect("bind is declared");
        let lib_map = crate::indexer::fqn::refer(&crate::indexer::fqn::Form::Lib {
            package: "ecmascript",
            member: "Array.map",
        })
        .unwrap();

        let mut placed = Vec::new();
        for rel in ["src/net.ts", "src/main.ts", "src/App.svelte"] {
            let facts = read_declarations(&root, &root.join(rel)).expect(rel);
            let text = std::fs::read_to_string(root.join(rel)).unwrap();
            let compiled =
                Compiled { index: &project.index, path: rel, declarations: &declarations };
            let calls: Vec<_> =
                facts.references.iter().filter(|r| r.kind == RefKind::Calls).cloned().collect();
            assert!(!calls.is_empty(), "{rel}: the walk saw no call");
            for call in place_with(&TS_ENGINE, Some(&compiled), calls, &text) {
                placed.push((rel, call.at.start_line, call.target));
            }
        }
        for (rel, line, target) in &placed {
            assert!(
                matches!(target, Resolution::Resolved { via: Rung::ResolvedByCompiler, .. }),
                "{rel}:{line} was not placed by the checker: {target:?}"
            );
        }
        let to = |rel: &str, line: u32| {
            placed.iter().find(|(r, l, _)| *r == rel && *l == line).map(|(_, _, t)| t.clone())
        };
        let resolved = |fqn: &Fqn| {
            Some(Resolution::Resolved { fqn: fqn.clone(), via: Rung::ResolvedByCompiler })
        };
        assert_eq!(to("src/main.ts", 4), resolved(&bind), "through the renamed import");
        assert_eq!(to("src/net.ts", 2), resolved(&lib_map), "onto the language's own method");
        assert_eq!(to("src/App.svelte", 4), resolved(&bind), "from a component's script");
        assert_eq!(to("src/App.svelte", 8), resolved(&bind), "from a component's template");
        // A component's own function is its top level, not a local of the
        // function svelte2tsx wraps the script in.
        let go = declarations.declared_at("src/App.svelte", 3, "go").expect("go is declared");
        let on_line_8: Vec<_> =
            placed.iter().filter(|(r, l, _)| *r == "src/App.svelte" && *l == 8).collect();
        assert!(
            on_line_8.iter().any(|(_, _, t)| Some(t.clone()) == resolved(&go)),
            "{on_line_8:?}"
        );
        assert_eq!(placed.len(), 5, "map, open, bind ×2, go: {placed:?}");
    }
}
