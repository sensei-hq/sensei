//! Library intelligence (workstream D) — a library declares the skills/agents/tools
//! it provides via a `sensei.library.json` manifest committed in its OWN repo; sensei
//! ingests that manifest into `sensei.library_skills` / `sensei.library_agents` and
//! associates the capabilities to any project that depends on the library.
//!
//! This is `crate::libraries` — distinct from `crate::api::handlers::libraries` (the
//! HTTP handlers) and `crate::adapters::manifest` (per-ecosystem dependency parsing).

pub mod manifest;
pub mod version;
pub mod registry;
pub mod advisory;

use std::path::Path;

/// Read + resolve a library's `sensei.library.json` from its local source root.
/// Returns `(version, skills, agents)` with each entry's `body` filled from its
/// `path` (read relative to `root`). Entries whose file can't be read — or whose
/// path escapes the root (`..`) — are left body-less (dropped downstream; never a
/// fabricated body). `None` when there is no manifest or it's malformed.
pub fn load_manifest_from_root(
    root: &Path,
) -> Option<(String, Vec<manifest::ProvidedSkill>, Vec<manifest::ProvidedAgent>)> {
    let text = std::fs::read_to_string(root.join(manifest::MANIFEST_FILENAME)).ok()?;
    let m = match manifest::parse_library_manifest(&text) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, root = %root.display(), "load_manifest_from_root: invalid manifest, skipping");
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
    Some((m.version, skills, agents))
}

/// Fill `body` from `path` (relative to `root`) if `body` is empty. A `..` in the
/// path or an unreadable file leaves `body` as `None` (a warning is logged).
fn resolve_body(root: &Path, body: &mut Option<String>, path: Option<&str>, name: &str) {
    if body.is_some() {
        return;
    }
    let Some(p) = path else { return };
    if p.contains("..") {
        tracing::warn!(entry = name, path = p, "load_manifest: refusing a manifest path that escapes the library root");
        return;
    }
    match std::fs::read_to_string(root.join(p)) {
        Ok(text) => *body = Some(text),
        Err(e) => tracing::warn!(entry = name, path = p, error = %e, "load_manifest: could not read manifest entry file, skipping it"),
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
        ).unwrap();

        let (version, skills, _agents) = load_manifest_from_root(root).unwrap();
        assert_eq!(version, ">=1.3");
        let styling = skills.iter().find(|s| s.focus == "styling").unwrap();
        assert_eq!(styling.body.as_deref(), Some("# styling body"), "path resolved to file body");
        assert!(skills.iter().find(|s| s.name == "missing").unwrap().body.is_none(), "unreadable path → no body");
        assert!(skills.iter().find(|s| s.name == "escape").unwrap().body.is_none(), "`..` path is refused");
    }

    #[test]
    fn no_manifest_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_manifest_from_root(dir.path()).is_none());
    }
}

