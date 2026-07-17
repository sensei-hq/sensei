//! `sensei scaffold` — create the canonical "spine of record" doc structure.
//!
//! Split into a pure `canonical_layout` (the spec) and an IO `materialize`
//! (writes it), so the structure is unit-testable without touching disk.
//!
//! Not yet wired to a CLI subcommand — `materialize` (IO) and the `Commands`
//! variant land in follow-up tasks. Until then this module's public API is
//! exercised only by its own tests.
#![allow(dead_code)]

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
}
