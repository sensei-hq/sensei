//! Library intelligence (workstream D) — a library declares the skills/agents/tools
//! it provides via a `sensei.library.json` manifest committed in its OWN repo; sensei
//! ingests that manifest into `sensei.library_skills` / `sensei.library_agents` and
//! associates the capabilities to any project that depends on the library.
//!
//! This is `crate::libraries` — distinct from `crate::api::handlers::libraries` (the
//! HTTP handlers) and `crate::adapters::manifest` (per-ecosystem dependency parsing).

pub mod advisory;
pub mod manifest;
pub mod registry;
pub mod version;

use std::path::Path;

/// A library's `sensei.library.json`, resolved: capability bodies read from disk
/// and the declared package names carried through.
///
/// A named type rather than a tuple because there are four fields and clippy is
/// right that nobody can read the tuple. It also replaced
/// `load_manifest_from_root`, which returned three of them — two readers over one
/// file is how a caller silently stops seeing a field, and the packages field is
/// exactly the sort of thing that would have been missed.
#[derive(Debug, Clone)]
pub struct ResolvedManifest {
    /// The library's own name — what its capabilities are addressed by.
    pub library: String,
    /// Semver RANGE the capabilities apply to (free text in v1).
    pub version: String,
    pub skills: Vec<manifest::ProvidedSkill>,
    pub agents: Vec<manifest::ProvidedAgent>,
    /// Packages this library publishes under other names. Empty is the common case.
    pub packages: Vec<String>,
}

/// How deep to look for `sensei.library.json` under a scanned root.
///
/// 2 covers `<root>/<repo>/sensei.library.json`, which is where a sibling-repo
/// checkout puts it. Deeper would start walking `node_modules` and vendored trees
/// for a file that belongs at a repository root.
pub const MANIFEST_SCAN_DEPTH: usize = 2;

/// Every `sensei.library.json` under `root`, to `MANIFEST_SCAN_DEPTH`.
///
/// Bounded and non-recursive-by-default on purpose: a manifest belongs at a
/// repository root, and an unbounded walk here would repeat the mistake that put
/// 1,211 vendored folders in the graph (#129).
///
/// Skips a directory it cannot read rather than failing the whole scan — one
/// unreadable sibling should not stop the others being registered.
pub fn find_manifests(root: &Path, max_depth: usize) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut frontier = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = frontier.pop() {
        if dir.join(manifest::MANIFEST_FILENAME).is_file() {
            out.push(dir.clone());
            // A manifest marks a library root; do not descend into it looking for
            // more. A nested one would belong to a vendored copy, not to this tree.
            continue;
        }
        if depth >= max_depth {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            // Skip dotted and dependency directories — a manifest under
            // node_modules describes a vendored copy, not this checkout.
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            if p.is_dir() {
                frontier.push((p, depth + 1));
            }
        }
    }
    out.sort();
    out
}

/// Ingest one library's `sensei.library.json` from a local root: capabilities,
/// declared packages, and — crucially — the PATH it was read from.
///
/// The single place manifest ingestion happens. It was previously inline in
/// `index_library`, which coupled it to doc indexing and to a transient
/// `LocalDir` source: a library with a manifest but no local docs never got its
/// skills, and nothing recorded where the manifest lived, so nothing could
/// re-read it. Measured before this: rokkit had 4 of 5 skills and 2 of 3 agents
/// (both missing files present on disk), dbd and kavach had none at all, and
/// `libraries.local_path` was empty on all 1,121 rows.
///
/// Storing `local_path` FIRST is deliberate: if a later step fails, the location
/// is still recorded and a refresh can retry. Losing the location is the failure
/// that made this unfixable without being handed the path again.
///
/// Returns `(skills, agents, packages)` counts. `None` when there is no manifest
/// or it is malformed — never a partial claim of success.
pub async fn ingest_manifest_at(
    pg: &crate::db::pg_store::PgStore,
    library_id: &uuid::Uuid,
    root: &Path,
) -> Option<(u32, u32, u64)> {
    let m = read_manifest(root)?;

    // The location first — see above.
    if let Err(e) = pg.set_library_local_path(library_id, &root.to_string_lossy()).await {
        tracing::warn!(error = %e, root = %root.display(), "ingest_manifest: set_library_local_path failed");
    }

    let (ns, na) = match pg
        .replace_library_capabilities(
            library_id,
            "manifest",
            Some(&m.version),
            &m.skills,
            &m.agents,
        )
        .await
    {
        Ok(counts) => counts,
        Err(e) => {
            tracing::warn!(error = %e, root = %root.display(), "ingest_manifest: replace_library_capabilities failed");
            (0, 0)
        }
    };

    let np = match pg.replace_library_packages(library_id, "manifest", &m.packages).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, root = %root.display(), "ingest_manifest: replace_library_packages failed");
            0
        }
    };

    Some((ns, na, np))
}

/// Read and resolve a library's `sensei.library.json` from its local source root.
///
/// Capability bodies are filled from their `path` (relative to `root`). An entry
/// whose file cannot be read — or whose path escapes the root via `..` — is left
/// body-less and dropped downstream, never given a fabricated body.
///
/// `None` when there is no manifest or it is malformed.
pub fn read_manifest(root: &Path) -> Option<ResolvedManifest> {
    let text = std::fs::read_to_string(root.join(manifest::MANIFEST_FILENAME)).ok()?;
    let m = match manifest::parse_library_manifest(&text) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, root = %root.display(), "read_manifest: invalid manifest, skipping");
            return None;
        }
    };
    let mut skills = m.skills;
    for s in &mut skills {
        resolve_body(root, &mut s.body, s.path.as_deref(), &s.name);
    }
    let mut agents = m.agents;
    for a in &mut agents {
        resolve_body(root, &mut a.body, a.path.as_deref(), &a.name);
    }
    Some(ResolvedManifest {
        library: m.library,
        version: m.version,
        skills,
        agents,
        packages: m.packages,
    })
}

/// Fill `body` from `path` (relative to `root`) if `body` is empty. A `..` in the
/// path or an unreadable file leaves `body` as `None` (a warning is logged).
fn resolve_body(root: &Path, body: &mut Option<String>, path: Option<&str>, name: &str) {
    if body.is_some() {
        return;
    }
    let Some(p) = path else { return };
    if p.contains("..") {
        tracing::warn!(
            entry = name,
            path = p,
            "load_manifest: refusing a manifest path that escapes the library root"
        );
        return;
    }
    match std::fs::read_to_string(root.join(p)) {
        Ok(text) => *body = Some(text),
        Err(e) => {
            tracing::warn!(entry = name, path = p, error = %e, "load_manifest: could not read manifest entry file, skipping it")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_manifest_resolves_paths_and_drops_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("skills")).unwrap();
        std::fs::write(root.join("skills/styling.md"), "# styling body").unwrap();
        std::fs::write(
            root.join(manifest::MANIFEST_FILENAME),
            r#"{"library":"rokkit","version":">=1.3",
                "skills":[
                  {"name":"semantic-styles-rokkit","focus":"styling","path":"skills/styling.md"},
                  {"name":"missing","focus":"x","path":"skills/nope.md"},
                  {"name":"escape","focus":"y","path":"../secret"}
                ]}"#,
        )
        .unwrap();

        let m = read_manifest(root).unwrap();
        let (version, skills) = (m.version, m.skills);
        assert_eq!(version, ">=1.3");
        let styling = skills.iter().find(|s| s.focus == "styling").unwrap();
        assert_eq!(styling.body.as_deref(), Some("# styling body"), "path resolved to file body");
        assert!(
            skills.iter().find(|s| s.name == "missing").unwrap().body.is_none(),
            "unreadable path → no body"
        );
        assert!(
            skills.iter().find(|s| s.name == "escape").unwrap().body.is_none(),
            "`..` path is refused"
        );
    }

    #[test]
    fn find_manifests_finds_sibling_repos_and_stops_at_a_library_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Two sibling library repos, one plain repo, and a vendored copy that must
        // NOT be picked up.
        for r in ["rokkit", "kavach"] {
            std::fs::create_dir_all(root.join(r)).unwrap();
            std::fs::write(
                root.join(r).join(manifest::MANIFEST_FILENAME),
                format!(r#"{{"library":"{r}","version":"1"}}"#),
            )
            .unwrap();
        }
        std::fs::create_dir_all(root.join("plain/src")).unwrap();
        std::fs::create_dir_all(root.join("app/node_modules/rokkit")).unwrap();
        std::fs::write(
            root.join("app/node_modules/rokkit").join(manifest::MANIFEST_FILENAME),
            r#"{"library":"rokkit","version":"1"}"#,
        )
        .unwrap();

        let found = find_manifests(root, MANIFEST_SCAN_DEPTH);
        assert_eq!(found.len(), 2, "the two sibling library repos: {found:?}");
        assert!(found.contains(&root.join("kavach")));
        assert!(found.contains(&root.join("rokkit")));
        assert!(
            !found.iter().any(|p| p.to_string_lossy().contains("node_modules")),
            "a vendored manifest describes someone else's copy, not this checkout",
        );
    }

    #[test]
    fn find_manifests_does_not_descend_into_a_library_root() {
        // A manifest marks a library root. Descending further would pick up a
        // nested/vendored manifest and register it as a second library.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let lib = root.join("lib");
        std::fs::create_dir_all(lib.join("packages/inner")).unwrap();
        std::fs::write(lib.join(manifest::MANIFEST_FILENAME), r#"{"library":"l","version":"1"}"#)
            .unwrap();
        std::fs::write(
            lib.join("packages/inner").join(manifest::MANIFEST_FILENAME),
            r#"{"library":"inner","version":"1"}"#,
        )
        .unwrap();

        let found = find_manifests(root, 4);
        assert_eq!(found, vec![lib], "stops at the library root");
    }

    #[test]
    fn read_manifest_carries_the_declared_packages() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(
            root.join(manifest::MANIFEST_FILENAME),
            r#"{"library":"rokkit","version":">=1.3","packages":["@rokkit/ui","@rokkit/actions"]}"#,
        )
        .unwrap();
        let m = read_manifest(root).unwrap();
        assert_eq!(m.library, "rokkit");
        assert_eq!(m.version, ">=1.3");
        assert_eq!(m.packages, vec!["@rokkit/ui", "@rokkit/actions"]);
    }

    #[test]
    fn no_manifest_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_manifest(dir.path()).is_none());
    }
}
