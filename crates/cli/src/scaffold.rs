//! `sensei scaffold` — create the canonical "spine of record" doc structure.
//!
//! Split into a pure `canonical_layout` (the spec) and an IO `materialize`
//! (writes it), so the structure is unit-testable without touching disk.

use std::fs;
use std::path::Path;

/// One entry in the canonical layout: a directory or a file with contents.
pub enum Entry {
    Dir(String),
    File { path: String, contents: String },
}

impl Entry {
    /// The entry's path relative to the scaffold base (handy for tests + logging).
    pub fn path(&self) -> &str {
        match self {
            Entry::Dir(p) => p,
            Entry::File { path, .. } => path,
        }
    }
}

/// The full set of directories + files the scaffold creates.
pub struct Layout {
    pub entries: Vec<Entry>,
}

/// The canonical doc structure (spec §5). Pure — takes the project name + date
/// and returns the layout; no IO, no clock. Identical for code and non-code
/// projects (the structure is the invariant).
pub fn canonical_layout(project: &str, date: &str) -> Layout {
    let dir = |p: &str| Entry::Dir(p.to_string());
    let file = |p: &str, c: String| Entry::File { path: p.to_string(), contents: c };
    Layout {
        entries: vec![
            file("docs/README.md", readme_md(project, date)),
            file("docs/vision.md", vision_md(project, date)),
            dir("docs/personas"),
            dir("docs/journeys"),
            dir("docs/roadmap"),
            dir("docs/design"),
            dir("docs/mockups"),
            dir("docs/features"),
            file("docs/features/README.md", features_md(project, date)),
            file("docs/decisions.md", decisions_md(project, date)),
        ],
    }
}

fn vision_md(project: &str, date: &str) -> String {
    format!(
        "---\nname: Vision — {project}\nupdated: {date}\n---\n\n# Vision\n\n\
         > The why. What this project is for, who it serves, and what \"good\" looks like.\n\
         > Keep it living — sharpen it as the objective clarifies.\n\n\
         ## Objective\n\n<!-- One paragraph: the problem and the outcome. -->\n\n\
         ## Audience\n\n<!-- Who it's for. See personas/. -->\n\n\
         ## Outcomes\n\n<!-- Experiences/outcomes it must deliver. See journeys/. -->\n"
    )
}

fn decisions_md(project: &str, date: &str) -> String {
    format!(
        "---\nname: Decisions log — {project}\nupdated: {date}\n---\n\n# Decisions\n\n\
         > Append-only. One entry per decision: date · decision · why · alternatives.\n\
         > The anti-rework memory — never re-derive a settled choice.\n"
    )
}

fn readme_md(project: &str, date: &str) -> String {
    format!(
        "---\nname: Docs structure — {project}\nupdated: {date}\n---\n\n\
         # Project docs — the spine of record\n\n\
         Canonical structure (scaffolded by `sensei scaffold`). The *shape* never\n\
         changes; the method that fills it does.\n\n\
         | Path | Slot | Holds |\n|---|---|---|\n\
         | vision.md | Intent | the why / objective |\n\
         | personas/ | Audience | who it's for |\n\
         | journeys/ | Outcomes | user flows |\n\
         | roadmap/ | — | phases + value releases (Planner output) |\n\
         | design/ | Structure | architecture + design-system reference |\n\
         | mockups/ | — | system-wide mockup (one cohesive artifact) |\n\
         | features/ | — | the FR/NFR registry; one dossier per feature |\n\
         | decisions.md | Decisions | append-only log |\n\n\
         **Not folders — live surfaces:** metrics (quality / coverage / churn /\n\
         velocity trends) and governance (rules) are dynamic surfaces in\n\
         Sensei / Dōjō, not docs here.\n"
    )
}

fn features_md(project: &str, date: &str) -> String {
    format!(
        "---\nname: Features — {project}\nupdated: {date}\n---\n\n# Features\n\n\
         The FR/NFR registry — the \"what\". Each feature is a dossier that mirrors\n\
         the project shape:\n\n\
         ```\n\
         <feature>/\n\
         \x20 brief.md        # intent-level: user objective + data (not layout)\n\
         \x20 design.md       # depth dialed by risk; cross-layer contract\n\
         \x20 mockup-ref.md   # link to a section of the system mockup (optional; added later)\n\
         \x20 plan.md         # tasks\n\
         \x20 tests/          # acceptance\n\
         \x20 decisions.md    # learnings\n\
         ```\n"
    )
}

/// What a scaffold run did — created vs already-present vs failed, in layout order.
#[derive(Default)]
pub struct ScaffoldReport {
    pub created: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: Vec<(String, String)>,
}

/// Write `layout` under `base`, idempotently: existing paths are skipped, never
/// overwritten. Returns the per-path report.
pub fn materialize(base: &Path, layout: &Layout) -> ScaffoldReport {
    let mut report = ScaffoldReport::default();
    for entry in &layout.entries {
        let rel = entry.path().to_string();
        let target = base.join(&rel);
        if target.exists() {
            report.skipped.push(rel);
            continue;
        }
        let result = match entry {
            Entry::Dir(_) => fs::create_dir_all(&target),
            Entry::File { contents, .. } => match target.parent() {
                Some(parent) => fs::create_dir_all(parent).and_then(|_| fs::write(&target, contents)),
                None => fs::write(&target, contents),
            },
        };
        match result {
            Ok(()) => report.created.push(rel),
            Err(e) => report.failed.push((rel, e.to_string())),
        }
    }
    report
}

/// Scaffold the canonical structure into `target`. Derives the project name from
/// the directory name and stamps today's date (via the crate's `format_date`).
pub fn run(target: &Path) -> ScaffoldReport {
    let name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");
    let date = crate::format_date();
    let layout = canonical_layout(name, &date);
    materialize(target, &layout)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(l: &Layout) -> Vec<&str> {
        l.entries.iter().map(|e| e.path()).collect()
    }

    #[test]
    fn layout_has_every_canonical_slot() {
        let l = canonical_layout("demo", "2026-07-17");
        let p = paths(&l);
        for expected in [
            "docs/README.md",
            "docs/vision.md",
            "docs/personas",
            "docs/journeys",
            "docs/roadmap",
            "docs/design",
            "docs/mockups",
            "docs/features",
            "docs/features/README.md",
            "docs/decisions.md",
        ] {
            assert!(p.contains(&expected), "layout missing {expected}");
        }
    }

    #[test]
    fn metrics_and_governance_are_not_folders() {
        // Spec §3.2: these are live surfaces (Sensei/Dōjō), never doc folders.
        let l = canonical_layout("demo", "2026-07-17");
        for banned in ["docs/metrics", "docs/governance"] {
            assert!(
                !paths(&l).iter().any(|p| *p == banned || p.starts_with(&format!("{banned}/"))),
                "{banned} must not be scaffolded as a folder"
            );
        }
    }

    #[test]
    fn vision_interpolates_project_and_date() {
        let l = canonical_layout("acme", "2026-07-17");
        let vision = l
            .entries
            .iter()
            .find_map(|e| match e {
                Entry::File { path, contents } if path == "docs/vision.md" => Some(contents),
                _ => None,
            })
            .expect("vision.md present");
        assert!(vision.contains("acme"), "vision names the project");
        assert!(vision.contains("2026-07-17"), "vision carries the date");
    }

    #[test]
    fn materialize_creates_every_entry_once() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = canonical_layout("demo", "2026-07-17");
        let report = materialize(tmp.path(), &layout);

        assert_eq!(report.created.len(), layout.entries.len(), "all created");
        assert!(report.skipped.is_empty(), "nothing skipped on a clean dir");
        assert!(tmp.path().join("docs/vision.md").is_file());
        assert!(tmp.path().join("docs/features").is_dir());
        assert!(tmp.path().join("docs/features/README.md").is_file());
        assert!(tmp.path().join("docs/decisions.md").is_file());
    }

    #[test]
    fn materialize_is_idempotent_and_never_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = canonical_layout("demo", "2026-07-17");
        materialize(tmp.path(), &layout);

        // Hand-edit a file, then re-run: it must be skipped, not clobbered.
        let vision = tmp.path().join("docs/vision.md");
        fs::write(&vision, "EDITED").unwrap();

        let second = materialize(tmp.path(), &layout);
        assert!(second.created.is_empty(), "second run creates nothing");
        assert_eq!(second.skipped.len(), layout.entries.len(), "all skipped");
        assert_eq!(fs::read_to_string(&vision).unwrap(), "EDITED", "not overwritten");
    }

    #[test]
    fn materialize_records_failures_instead_of_swallowing() {
        // A read-only base dir → writes inside it fail; failures must be recorded,
        // not silently dropped (house "no silent errors" rule). Tests run as a
        // normal user; root would ignore the perms.
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        let ro = {
            let mut p = fs::metadata(base).unwrap().permissions();
            p.set_readonly(true);
            p
        };
        fs::set_permissions(base, ro).unwrap();

        let layout = canonical_layout("demo", "2026-07-17");
        let report = materialize(base, &layout);

        // Restore write perms BEFORE asserting so tempdir cleanup always succeeds.
        // The world-writable concern of `set_readonly(false)` is moot: this is a
        // throwaway tempdir deleted immediately after the test.
        #[allow(clippy::permissions_set_readonly_false)]
        let rw = {
            let mut p = fs::metadata(base).unwrap().permissions();
            p.set_readonly(false);
            p
        };
        fs::set_permissions(base, rw).unwrap();

        assert!(!report.failed.is_empty(), "failures recorded, not swallowed");
        assert!(report.created.is_empty(), "nothing created under a read-only base");
    }
}
