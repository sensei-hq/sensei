//! The indexer's compiler engines: what places call edges per language, and
//! whether this machine has them.
//!
//! * **Rust** — rust-analyzer, found through `rustup which rust-analyzer`, with
//!   the standard library's sources (`rust-src`) beside it.
//! * **TypeScript / JavaScript / Svelte** — `node`, running the daemon's
//!   embedded driver, which loads its packages from the project or else from
//!   [`engines_dir`]. The versions it loads are pinned in [`ENGINES_MANIFEST`],
//!   the one copy of that list.
//!
//! [`check`] reports each engine as ready or missing with every cause named;
//! [`install`] installs what it can (rustup components, the npm packages) and
//! names what it cannot (rustup or node absent — those it never installs).
//! [`ensure`] is check → install what is missing → check again.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::config::engines_dir;
use crate::health::process_util::{TimedOutcome, output_with_timeout};

/// The npm packages the TypeScript engine loads, pinned: written as
/// `package.json` into [`engines_dir`] and installed there.
pub const ENGINES_MANIFEST: &str = include_str!("../assets/ts_engines.json");

/// Budget for a probe (`rustup which`, `node -p`).
const PROBE_TIMEOUT: Duration = Duration::from_secs(20);

/// Budget for an install: a component or package download on a slow link.
const INSTALL_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// One compiler engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Rust,
    TypeScript,
}

impl Engine {
    pub const ALL: [Engine; 2] = [Engine::Rust, Engine::TypeScript];

    pub fn label(self) -> &'static str {
        match self {
            Engine::Rust => "rust",
            Engine::TypeScript => "typescript",
        }
    }
}

/// Why a probe produced no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeError {
    /// The program is not on PATH.
    NotFound,
    /// It ran and exited unsuccessfully; what it said.
    Exited(String),
    /// It did not finish within its budget and was killed.
    TimedOut,
    /// Spawning or waiting failed at the OS level.
    Failed(String),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbeError::NotFound => write!(f, "not found on PATH"),
            ProbeError::Exited(said) => write!(f, "failed: {said}"),
            ProbeError::TimedOut => write!(f, "timed out"),
            ProbeError::Failed(e) => write!(f, "could not run: {e}"),
        }
    }
}

/// One reason an engine is not ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cause {
    RustupMissing,
    /// `rustup which rust-analyzer` failed; what rustup said.
    RustAnalyzerMissing(String),
    RustSrcMissing,
    NodeMissing,
    ManifestMissing(PathBuf),
    /// The engines directory's `package.json` is not the pinned one.
    ManifestStale(PathBuf),
    ManifestUnreadable {
        path: PathBuf,
        cause: String,
    },
    PackageMissing(String),
    PackageVersion {
        name: String,
        pinned: String,
        installed: String,
    },
    PackageUnreadable {
        name: String,
        cause: String,
    },
    /// A probe that could not answer at all.
    Probe {
        command: String,
        error: ProbeError,
    },
}

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cause::RustupMissing => {
                write!(f, "rustup not found on PATH (install it from https://rustup.rs)")
            }
            Cause::RustAnalyzerMissing(said) => write!(f, "rust-analyzer not installed: {said}"),
            Cause::RustSrcMissing => write!(f, "rustup component rust-src not installed"),
            Cause::NodeMissing => write!(f, "node not found on PATH (install Node.js)"),
            Cause::ManifestMissing(p) => write!(f, "{} does not exist", p.display()),
            Cause::ManifestStale(p) => {
                write!(f, "{} is not the pinned engines manifest", p.display())
            }
            Cause::ManifestUnreadable { path, cause } => {
                write!(f, "{} unreadable: {cause}", path.display())
            }
            Cause::PackageMissing(name) => write!(f, "npm package {name} not installed"),
            Cause::PackageVersion { name, pinned, installed } => {
                write!(f, "npm package {name} is {installed}, pinned {pinned}")
            }
            Cause::PackageUnreadable { name, cause } => {
                write!(f, "npm package {name} unreadable: {cause}")
            }
            Cause::Probe { command, error } => write!(f, "`{command}` {error}"),
        }
    }
}

/// An engine and everything keeping it from being ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineStatus {
    pub engine: Engine,
    pub missing: Vec<Cause>,
}

impl EngineStatus {
    pub fn is_ready(&self) -> bool {
        self.missing.is_empty()
    }
}

/// An install step that did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallFailure {
    pub engine: Engine,
    pub cause: String,
}

impl fmt::Display for InstallFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.engine.label(), self.cause)
    }
}

/// What [`ensure`] found, did, and found afterwards.
#[derive(Debug, Clone)]
pub struct Ensured {
    pub before: Vec<EngineStatus>,
    pub installed: Vec<Engine>,
    pub failures: Vec<InstallFailure>,
    pub after: Vec<EngineStatus>,
}

// ── Pure decisions ──────────────────────────────────────────────────────────

/// The pinned `(package, version)` pairs of a manifest's `dependencies`.
pub fn pinned_dependencies(manifest: &str) -> Result<Vec<(String, String)>, String> {
    let value: serde_json::Value = serde_json::from_str(manifest).map_err(|e| e.to_string())?;
    let deps =
        value.get("dependencies").and_then(|d| d.as_object()).ok_or("no `dependencies` object")?;
    let mut pinned = deps
        .iter()
        .map(|(name, v)| {
            v.as_str()
                .map(|v| (name.clone(), v.to_string()))
                .ok_or_else(|| format!("{name}: version is not a string"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    pinned.sort();
    Ok(pinned)
}

/// What keeps the Rust engine from being ready, given what
/// `rustup component list --installed` and `rustup which rust-analyzer` said.
pub fn rust_causes(
    components: Result<String, ProbeError>,
    analyzer: Result<String, ProbeError>,
) -> Vec<Cause> {
    if matches!(components, Err(ProbeError::NotFound))
        || matches!(analyzer, Err(ProbeError::NotFound))
    {
        return vec![Cause::RustupMissing];
    }
    let mut causes = vec![];
    match components {
        Ok(list) => {
            if !list.lines().map(str::trim).any(|c| c == "rust-src" || c.starts_with("rust-src-")) {
                causes.push(Cause::RustSrcMissing);
            }
        }
        Err(error) => {
            causes.push(Cause::Probe { command: "rustup component list --installed".into(), error })
        }
    }
    match analyzer {
        Ok(_) => {}
        Err(ProbeError::Exited(said)) => causes.push(Cause::RustAnalyzerMissing(said)),
        Err(error) => {
            causes.push(Cause::Probe { command: "rustup which rust-analyzer".into(), error })
        }
    }
    causes
}

/// What keeps a pinned package from being ready, given its installed
/// `package.json` text.
pub fn package_cause(name: &str, pinned: &str, installed_manifest: &str) -> Option<Cause> {
    let unreadable = |cause: String| Cause::PackageUnreadable { name: name.into(), cause };
    let value: serde_json::Value = match serde_json::from_str(installed_manifest) {
        Ok(v) => v,
        Err(e) => return Some(unreadable(e.to_string())),
    };
    let Some(installed) = value.get("version").and_then(|v| v.as_str()) else {
        return Some(unreadable("package.json has no version".into()));
    };
    (installed != pinned).then(|| Cause::PackageVersion {
        name: name.into(),
        pinned: pinned.into(),
        installed: installed.into(),
    })
}

/// The engines that are not ready, in order.
pub fn needing_install(statuses: &[EngineStatus]) -> Vec<Engine> {
    statuses.iter().filter(|s| !s.is_ready()).map(|s| s.engine).collect()
}

// ── IO ──────────────────────────────────────────────────────────────────────

/// Run a probe within its budget; its trimmed stdout, or why there is none.
fn run(program: &str, args: &[&str], timeout: Duration) -> Result<String, ProbeError> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    match output_with_timeout(cmd, timeout) {
        TimedOutcome::Done(out) if out.status.success() => {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
        TimedOutcome::Done(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let said = if stderr.is_empty() {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else {
                stderr
            };
            Err(ProbeError::Exited(format!("{} ({})", said, out.status)))
        }
        TimedOutcome::TimedOut => Err(ProbeError::TimedOut),
        TimedOutcome::Failed(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(ProbeError::NotFound)
        }
        TimedOutcome::Failed(e) => Err(ProbeError::Failed(e.to_string())),
    }
}

/// What keeps the npm packages in `dir` from being the pinned ones.
pub fn package_causes(dir: &Path) -> Vec<Cause> {
    let mut causes = vec![];
    let path = dir.join("package.json");
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let on_disk = serde_json::from_str::<serde_json::Value>(&text);
            let pinned = serde_json::from_str::<serde_json::Value>(ENGINES_MANIFEST)
                .expect("the embedded engines manifest is JSON");
            match on_disk {
                Ok(v) if v == pinned => {}
                Ok(_) => causes.push(Cause::ManifestStale(path)),
                Err(e) => causes.push(Cause::ManifestUnreadable { path, cause: e.to_string() }),
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            causes.push(Cause::ManifestMissing(path))
        }
        Err(e) => causes.push(Cause::ManifestUnreadable { path, cause: e.to_string() }),
    }
    let pinned =
        pinned_dependencies(ENGINES_MANIFEST).expect("the embedded engines manifest pins packages");
    for (name, version) in pinned {
        let installed = dir.join("node_modules").join(&name).join("package.json");
        match std::fs::read_to_string(&installed) {
            Ok(text) => causes.extend(package_cause(&name, &version, &text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                causes.push(Cause::PackageMissing(name))
            }
            Err(e) => causes.push(Cause::PackageUnreadable { name, cause: e.to_string() }),
        }
    }
    causes
}

fn rust_status() -> EngineStatus {
    let components = run("rustup", &["component", "list", "--installed"], PROBE_TIMEOUT);
    let analyzer = run("rustup", &["which", "rust-analyzer"], PROBE_TIMEOUT);
    EngineStatus { engine: Engine::Rust, missing: rust_causes(components, analyzer) }
}

fn typescript_status(dir: &Path) -> EngineStatus {
    let mut missing = match run("node", &["-p", "process.execPath"], PROBE_TIMEOUT) {
        Ok(_) => vec![],
        Err(ProbeError::NotFound) => vec![Cause::NodeMissing],
        Err(error) => vec![Cause::Probe { command: "node -p process.execPath".into(), error }],
    };
    missing.extend(package_causes(dir));
    EngineStatus { engine: Engine::TypeScript, missing }
}

/// Every engine's status on this machine.
pub fn check() -> Vec<EngineStatus> {
    vec![rust_status(), typescript_status(&engines_dir())]
}

fn install_rust() -> Result<(), String> {
    match run("rustup", &["component", "add", "rust-analyzer", "rust-src"], INSTALL_TIMEOUT) {
        Ok(_) => Ok(()),
        Err(ProbeError::NotFound) => Err(Cause::RustupMissing.to_string()),
        Err(e) => Err(format!("`rustup component add rust-analyzer rust-src` {e}")),
    }
}

/// Write the pinned manifest into `dir` and have npm install it there.
fn install_typescript(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let manifest = dir.join("package.json");
    std::fs::write(&manifest, ENGINES_MANIFEST)
        .map_err(|e| format!("write {}: {e}", manifest.display()))?;
    let prefix = dir.to_string_lossy();
    match run("npm", &["install", "--prefix", &prefix, "--no-audit", "--no-fund"], INSTALL_TIMEOUT)
    {
        Ok(_) => Ok(()),
        Err(ProbeError::NotFound) => Err("npm not found on PATH (install Node.js)".into()),
        Err(e) => Err(format!("`npm install --prefix {prefix}` {e}")),
    }
}

fn install_engine(engine: Engine) -> Result<(), InstallFailure> {
    let done = match engine {
        Engine::Rust => install_rust(),
        Engine::TypeScript => install_typescript(&engines_dir()),
    };
    done.map_err(|cause| InstallFailure { engine, cause })
}

/// Install every engine: the rustup components and the pinned npm packages.
/// Each is attempted; every failure is returned.
pub fn install() -> Result<(), Vec<InstallFailure>> {
    let failures: Vec<_> =
        Engine::ALL.into_iter().filter_map(|e| install_engine(e).err()).collect();
    if failures.is_empty() { Ok(()) } else { Err(failures) }
}

/// Check; install the engines that are not ready; check again.
pub fn ensure() -> Ensured {
    let before = check();
    let installed = needing_install(&before);
    if installed.is_empty() {
        let after = before.clone();
        return Ensured { before, installed, failures: vec![], after };
    }
    let failures = installed.iter().filter_map(|e| install_engine(*e).err()).collect();
    Ensured { before, installed, failures, after: check() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exited(said: &str) -> Result<String, ProbeError> {
        Err(ProbeError::Exited(said.into()))
    }

    const INSTALLED: &str = "cargo-aarch64-apple-darwin\nrust-analyzer-aarch64-apple-darwin\nrust-src\nrustc-aarch64-apple-darwin\n";

    #[test]
    fn the_embedded_manifest_pins_every_package_the_driver_loads() {
        let pinned = pinned_dependencies(ENGINES_MANIFEST).expect("embedded manifest parses");
        let names: Vec<&str> = pinned.iter().map(|(n, _)| n.as_str()).collect();
        for loaded in ["typescript", "svelte2tsx", "@jridgewell/trace-mapping", "svelte"] {
            assert!(names.contains(&loaded), "{loaded} is not pinned: {names:?}");
        }
        assert!(pinned.iter().all(|(_, v)| !v.is_empty()));
    }

    #[test]
    fn pinned_dependencies_reads_each_name_and_version() {
        let pinned =
            pinned_dependencies(r#"{"dependencies":{"a":"1.0.0","@s/b":"2.3.4"}}"#).unwrap();
        assert_eq!(
            pinned,
            vec![("@s/b".to_string(), "2.3.4".to_string()), ("a".to_string(), "1.0.0".to_string())]
        );
    }

    #[test]
    fn a_manifest_without_dependencies_is_an_error_not_an_empty_list() {
        assert!(pinned_dependencies(r#"{"name":"x"}"#).is_err());
        assert!(pinned_dependencies("not json").is_err());
    }

    #[test]
    fn rust_is_ready_when_both_components_are_there() {
        assert_eq!(rust_causes(Ok(INSTALLED.into()), Ok("/x/rust-analyzer".into())), vec![]);
    }

    #[test]
    fn rust_src_absent_from_the_installed_list_is_named() {
        let list = "rust-analyzer-aarch64-apple-darwin\nrust-std-aarch64-apple-darwin\n";
        assert_eq!(
            rust_causes(Ok(list.into()), Ok("/x/rust-analyzer".into())),
            vec![Cause::RustSrcMissing]
        );
    }

    #[test]
    fn rust_analyzer_unresolved_carries_what_rustup_said() {
        assert_eq!(
            rust_causes(Ok(INSTALLED.into()), exited("'rust-analyzer' is not installed")),
            vec![Cause::RustAnalyzerMissing("'rust-analyzer' is not installed".into())]
        );
    }

    #[test]
    fn without_rustup_the_one_cause_is_rustup() {
        assert_eq!(
            rust_causes(Err(ProbeError::NotFound), Err(ProbeError::NotFound)),
            vec![Cause::RustupMissing]
        );
    }

    #[test]
    fn a_probe_that_timed_out_is_its_own_cause_not_ready() {
        assert_eq!(
            rust_causes(Err(ProbeError::TimedOut), Ok("/x/rust-analyzer".into())),
            vec![Cause::Probe {
                command: "rustup component list --installed".into(),
                error: ProbeError::TimedOut,
            }]
        );
    }

    #[test]
    fn a_package_at_its_pinned_version_is_ready() {
        assert_eq!(package_cause("typescript", "5.9.3", r#"{"version":"5.9.3"}"#), None);
    }

    #[test]
    fn a_package_at_another_version_names_both() {
        assert_eq!(
            package_cause("typescript", "5.9.3", r#"{"version":"5.8.0"}"#),
            Some(Cause::PackageVersion {
                name: "typescript".into(),
                pinned: "5.9.3".into(),
                installed: "5.8.0".into(),
            })
        );
    }

    #[test]
    fn a_package_manifest_without_a_version_is_unreadable() {
        assert!(matches!(
            package_cause("typescript", "5.9.3", r#"{"name":"typescript"}"#),
            Some(Cause::PackageUnreadable { .. })
        ));
        assert!(matches!(
            package_cause("typescript", "5.9.3", "{"),
            Some(Cause::PackageUnreadable { .. })
        ));
    }

    /// An engines directory laid out as npm leaves it, every package at `version`.
    fn engines_at(dir: &Path, version: impl Fn(&str) -> String) {
        std::fs::write(dir.join("package.json"), ENGINES_MANIFEST).unwrap();
        for (name, pinned) in pinned_dependencies(ENGINES_MANIFEST).unwrap() {
            let pkg = dir.join("node_modules").join(&name);
            std::fs::create_dir_all(&pkg).unwrap();
            std::fs::write(
                pkg.join("package.json"),
                format!(r#"{{"name":"{name}","version":"{}"}}"#, version(&pinned)),
            )
            .unwrap();
        }
    }

    #[test]
    fn an_installed_engines_directory_has_no_causes() {
        let dir = tempfile::tempdir().unwrap();
        engines_at(dir.path(), |pinned| pinned.to_string());
        assert_eq!(package_causes(dir.path()), vec![]);
    }

    #[test]
    fn an_empty_engines_directory_names_the_manifest_and_every_package() {
        let dir = tempfile::tempdir().unwrap();
        let causes = package_causes(dir.path());
        let pinned = pinned_dependencies(ENGINES_MANIFEST).unwrap();
        assert_eq!(causes[0], Cause::ManifestMissing(dir.path().join("package.json")));
        assert_eq!(causes.len(), 1 + pinned.len());
        for (name, _) in pinned {
            assert!(causes.contains(&Cause::PackageMissing(name.clone())), "{name}: {causes:?}");
        }
    }

    #[test]
    fn a_manifest_other_than_the_pinned_one_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        engines_at(dir.path(), |pinned| pinned.to_string());
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"typescript":"5.0.0"}}"#,
        )
        .unwrap();
        assert_eq!(
            package_causes(dir.path()),
            vec![Cause::ManifestStale(dir.path().join("package.json"))]
        );
    }

    #[test]
    fn the_manifest_is_compared_as_json_not_as_bytes() {
        let dir = tempfile::tempdir().unwrap();
        engines_at(dir.path(), |pinned| pinned.to_string());
        let value: serde_json::Value = serde_json::from_str(ENGINES_MANIFEST).unwrap();
        std::fs::write(dir.path().join("package.json"), value.to_string()).unwrap();
        assert_eq!(package_causes(dir.path()), vec![]);
    }

    #[test]
    fn a_package_at_another_version_is_named_in_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        engines_at(dir.path(), |pinned| format!("{pinned}-old"));
        let causes = package_causes(dir.path());
        assert_eq!(causes.len(), pinned_dependencies(ENGINES_MANIFEST).unwrap().len());
        assert!(causes.iter().all(|c| matches!(c, Cause::PackageVersion { .. })), "{causes:?}");
    }

    #[test]
    fn only_engines_that_are_not_ready_are_installed() {
        let statuses = vec![
            EngineStatus { engine: Engine::Rust, missing: vec![] },
            EngineStatus { engine: Engine::TypeScript, missing: vec![Cause::NodeMissing] },
        ];
        assert_eq!(needing_install(&statuses), vec![Engine::TypeScript]);
        assert_eq!(needing_install(&statuses[..1]), vec![]);
    }

    #[test]
    fn a_probe_names_a_missing_program_and_a_failing_one_apart() {
        assert_eq!(
            run("definitely_not_a_real_binary_xyz_77", &[], PROBE_TIMEOUT),
            Err(ProbeError::NotFound)
        );
        assert!(matches!(run("false", &[], PROBE_TIMEOUT), Err(ProbeError::Exited(_))));
        assert_eq!(run("echo", &[" hi "], PROBE_TIMEOUT), Ok("hi".into()));
    }
}
