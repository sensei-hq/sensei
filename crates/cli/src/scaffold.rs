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
                Some(parent) => {
                    fs::create_dir_all(parent).and_then(|_| fs::write(&target, contents))
                }
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
    let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("project");
    let date = crate::format_date();
    let layout = canonical_layout(name, &date);
    materialize(target, &layout)
}

/// The per-feature dossier (spec §3.2, feature-scope column). Pure — takes the
/// feature name + date, returns the layout rooted at `docs/features/<feature>/`.
/// Mirrors the project spine, lighter: Intent(brief) · Structure(design) ·
/// Outcomes/Signals(tests) · Decisions(decisions) + plan + optional mockup-ref.
pub fn feature_layout(feature: &str, date: &str) -> Layout {
    let base = format!("docs/features/{feature}");
    let dir = |p: String| Entry::Dir(p);
    let file = |p: String, c: String| Entry::File { path: p, contents: c };
    Layout {
        entries: vec![
            file(format!("{base}/brief.md"), feature_brief_md(feature, date)),
            file(format!("{base}/design.md"), feature_design_md(feature, date)),
            file(format!("{base}/plan.md"), feature_plan_md(feature, date)),
            dir(format!("{base}/tests")),
            file(format!("{base}/tests/README.md"), feature_tests_md(feature, date)),
            file(format!("{base}/decisions.md"), feature_decisions_md(feature, date)),
            file(format!("{base}/mockup-ref.md"), feature_mockup_ref_md(feature, date)),
        ],
    }
}

fn feature_brief_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — brief\nupdated: {date}\n---\n\n# {feature} — brief\n\n\
         > Intent. The user objective + the data it moves — not the layout.\n\
         > Audience inherits the project personas. Keep it short; sharpen as it clarifies.\n\n\
         ## Goal\n\n<!-- One paragraph: the user objective this chunk delivers. -->\n\n\
         ## Acceptance\n\n<!-- Observable outcomes that mean done. Mirror into tests/. -->\n\n\
         ## Data\n\n<!-- The entities/fields this touches (not screens). -->\n"
    )
}

fn feature_design_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — design\nupdated: {date}\n---\n\n# {feature} — design\n\n\
         > Structure. Depth dialed by risk: shallow for low-risk/known; deep with a\n\
         > fixed cross-layer contract for high blast-radius — settled BEFORE code.\n\n\
         ## Cross-layer contract\n\n\
         <!-- db → api → ui interface the sub-agents build against. Fix it here first. -->\n\n\
         ## Gates that apply\n\n\
         <!-- Rules relevant here — resolved live via get_rules; note the ones that bind. -->\n\n\
         ## Approach\n\n<!-- How it's built, proportional to risk. -->\n"
    )
}

fn feature_plan_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — plan\nupdated: {date}\n---\n\n# {feature} — plan\n\n\
         > Tasks. The build steps for this chunk — bite-sized, TDD, frequent commits.\n"
    )
}

fn feature_tests_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — acceptance\nupdated: {date}\n---\n\n# {feature} — acceptance\n\n\
         > Outcomes / signals. The acceptance criteria for this chunk and the tests that\n\
         > prove them. Coverage for {feature} lives here.\n"
    )
}

fn feature_decisions_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — decisions\nupdated: {date}\n---\n\n# {feature} — decisions\n\n\
         > Append-only. One entry per decision: date · decision · why · alternatives.\n\
         > The chunk's anti-rework memory — never re-derive a settled choice.\n"
    )
}

fn feature_mockup_ref_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — mockup ref\nupdated: {date}\n---\n\n# {feature} — mockup reference\n\n\
         > Optional. Link to the section of the system-wide mockup (docs/mockups/) this\n\
         > feature realizes. The mockup is one cohesive artifact — point here, don't fork it.\n"
    )
}

/// A feature name must be a single, safe path segment — no separators, no `..`,
/// non-empty. Guards against scaffolding outside `docs/features/` (path traversal).
pub fn is_safe_feature_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
}

/// Scaffold a feature dossier under `target/docs/features/<feature>/`. Validates the
/// name, stamps today's date, and reuses the idempotent `materialize`. Returns an
/// Err with a message on an unsafe name (nothing is written).
pub fn run_feature(target: &Path, feature: &str) -> Result<ScaffoldReport, String> {
    if !is_safe_feature_name(feature) {
        return Err(format!(
            "invalid feature name {feature:?} — use a single path segment (no '/', '\\', or '..')"
        ));
    }
    let date = crate::format_date();
    let layout = feature_layout(feature, &date);
    Ok(materialize(target, &layout))
}

/// The kind of project a baseline contract targets — selects the adapter column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
#[value(rename_all = "lower")]
pub enum BaselineKind {
    Code,
    Content,
}

impl BaselineKind {
    /// Stable slug used in the contract's frontmatter.
    pub fn slug(self) -> &'static str {
        match self {
            BaselineKind::Code => "code",
            BaselineKind::Content => "content",
        }
    }
}

/// The baseline capability-contract layout (spec §3.6). Pure — one file,
/// `docs/baseline.md`, rendered for the given kind.
pub fn baseline_layout(kind: BaselineKind, date: &str) -> Layout {
    Layout {
        entries: vec![Entry::File {
            path: "docs/baseline.md".to_string(),
            contents: baseline_md(kind, date),
        }],
    }
}

fn baseline_md(kind: BaselineKind, date: &str) -> String {
    // Capability × adapter, per §3.6. The adapter column switches on kind.
    let rows: &[(&str, &str)] = match kind {
        BaselineKind::Code => &[
            ("Format", "prettier / rustfmt"),
            ("Lint", "eslint / clippy"),
            ("Unit test", "vitest / cargo test"),
            ("Flow test", "Playwright e2e"),
            ("Coverage", "coverage %"),
            ("Quality", "qlty.sh score"),
            ("Security", "semgrep / deps scan"),
            ("Churn + velocity", "git signal"),
            ("Design system", "rokkit tokens + component catalog (no hand-rolled primitives)"),
        ],
        BaselineKind::Content => &[
            ("Format", "style-guide conformance"),
            ("Lint", "grammar / tone"),
            ("Unit test", "fact / continuity check"),
            ("Integration", "chapter-to-chapter coherence"),
            ("Flow test", "full read-through / arc check"),
            ("Coverage", "outline coverage"),
            ("Quality", "readability / pacing"),
            ("Churn + velocity", "draft-revision signal"),
            ("Design system", "template / layout system"),
        ],
    };
    let mut table = String::from("| Capability | Adapter |\n|---|---|\n");
    for (cap, adapter) in rows {
        table.push_str(&format!("| {cap} | {adapter} |\n"));
    }
    format!(
        "---\nname: Baseline — capability contract\nkind: {kind}\nupdated: {date}\n---\n\n\
         # Baseline — the capability contract\n\n\
         > The capabilities every change must satisfy — a *contract*, not a fixed toolset.\n\
         > Installed once at project start (recommend-and-confirm); after that it's just\n\
         > `bun run x` / `make x`. Sensei detects the stack and fills concrete tools\n\
         > (rides the manifest-adapter); conformance streams into the Signals slot as the\n\
         > project health score.\n\n\
         {table}\n\
         ## Gates\n\n\
         - **Security scan — block.**\n\
         - **Test coverage ≥ 80% — block** (org-tunable via Dōjō).\n\
         - **Quality floor — block.**\n\n\
         Everything else is installed + guided, ratcheting up over time.\n\n\
         ## Governance\n\n\
         The strictness layer is **live**, not recorded here — resolved at the point of\n\
         work via `get_rules` (org / Dōjō top-down + contributed bottom-up); mandatory\n\
         rules are non-overridable. This file records the *contract*; governance sets the\n\
         *strictness*.\n",
        kind = kind.slug()
    )
}

/// Scaffold the baseline contract into `target/docs/baseline.md`. Stamps today's
/// date and reuses the idempotent `materialize`.
pub fn run_baseline(target: &Path, kind: BaselineKind) -> ScaffoldReport {
    let date = crate::format_date();
    let layout = baseline_layout(kind, &date);
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

    // ── feature-dossier scaffolder (`sensei scaffold feature <name>`) ───────

    #[test]
    fn feature_layout_has_every_dossier_slot() {
        let l = feature_layout("auth", "2026-07-18");
        let p = paths(&l);
        for expected in [
            "docs/features/auth/brief.md",
            "docs/features/auth/design.md",
            "docs/features/auth/plan.md",
            "docs/features/auth/tests",
            "docs/features/auth/tests/README.md",
            "docs/features/auth/decisions.md",
            "docs/features/auth/mockup-ref.md",
        ] {
            assert!(p.contains(&expected), "dossier missing {expected}");
        }
    }

    #[test]
    fn feature_layout_is_rooted_at_the_named_feature() {
        let l = feature_layout("billing", "2026-07-18");
        assert!(
            l.entries.iter().all(|e| e.path().starts_with("docs/features/billing/")),
            "every entry is under docs/features/<name>/"
        );
    }

    #[test]
    fn feature_layout_governance_is_not_a_slot() {
        // §3.2: constraints/governance are LIVE (get_rules), never a dossier file/folder.
        let l = feature_layout("auth", "2026-07-18");
        assert!(
            !paths(&l).iter().any(|p| p.contains("governance") || p.ends_with("/rules.md")),
            "governance must not be scaffolded into the dossier"
        );
    }

    #[test]
    fn feature_brief_interpolates_name_and_date() {
        let l = feature_layout("checkout", "2026-07-18");
        let brief = l
            .entries
            .iter()
            .find_map(|e| match e {
                Entry::File { path, contents } if path == "docs/features/checkout/brief.md" => {
                    Some(contents)
                }
                _ => None,
            })
            .expect("brief.md present");
        assert!(brief.contains("checkout"), "brief names the feature");
        assert!(brief.contains("2026-07-18"), "brief carries the date");
    }

    #[test]
    fn run_feature_creates_the_dossier_once() {
        let tmp = tempfile::tempdir().unwrap();
        let report = run_feature(tmp.path(), "auth").expect("safe name");
        assert!(report.created.contains(&"docs/features/auth/brief.md".to_string()));
        assert!(tmp.path().join("docs/features/auth/brief.md").is_file());
        assert!(tmp.path().join("docs/features/auth/tests").is_dir());
        assert!(tmp.path().join("docs/features/auth/tests/README.md").is_file());
        assert!(tmp.path().join("docs/features/auth/decisions.md").is_file());
        assert!(report.skipped.is_empty() && report.failed.is_empty());
    }

    #[test]
    fn run_feature_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        run_feature(tmp.path(), "auth").unwrap();
        let brief = tmp.path().join("docs/features/auth/brief.md");
        fs::write(&brief, "EDITED").unwrap();
        let second = run_feature(tmp.path(), "auth").unwrap();
        assert!(second.created.is_empty(), "second run creates nothing");
        assert_eq!(fs::read_to_string(&brief).unwrap(), "EDITED", "not overwritten");
    }

    #[test]
    fn run_feature_rejects_unsafe_names() {
        let tmp = tempfile::tempdir().unwrap();
        for bad in ["../evil", "a/b", "..", "", "."] {
            assert!(run_feature(tmp.path(), bad).is_err(), "must reject {bad:?}");
        }
        assert!(!tmp.path().join("../evil").exists());
    }

    // ── baseline capability-contract scaffolder (`sensei scaffold baseline`) ──

    #[test]
    fn baseline_layout_is_a_single_contract_file() {
        let l = baseline_layout(BaselineKind::Code, "2026-07-18");
        assert_eq!(paths(&l), vec!["docs/baseline.md"]);
    }

    #[test]
    fn baseline_code_names_code_tools_and_the_gate() {
        let l = baseline_layout(BaselineKind::Code, "2026-07-18");
        let md = match &l.entries[0] {
            Entry::File { contents, .. } => contents,
            _ => panic!("baseline.md is a file"),
        };
        assert!(md.contains("kind: code"), "frontmatter carries the kind");
        assert!(md.contains("2026-07-18"), "carries the date");
        assert!(md.contains("eslint") || md.contains("clippy"), "code lint adapter");
        assert!(md.contains("80%"), "the ≥80% coverage gate");
        assert!(md.contains("get_rules"), "governance is live, not scaffolded");
    }

    #[test]
    fn baseline_content_uses_non_code_adapters() {
        let l = baseline_layout(BaselineKind::Content, "2026-07-18");
        let md = match &l.entries[0] {
            Entry::File { contents, .. } => contents,
            _ => panic!("baseline.md is a file"),
        };
        assert!(md.contains("kind: content"), "frontmatter carries the kind");
        assert!(md.contains("grammar") || md.contains("tone"), "content lint adapter");
        assert!(!md.contains("eslint"), "no code tools in the content contract");
    }

    #[test]
    fn run_baseline_writes_the_contract() {
        let tmp = tempfile::tempdir().unwrap();
        let report = run_baseline(tmp.path(), BaselineKind::Code);
        assert_eq!(report.created, vec!["docs/baseline.md".to_string()]);
        assert!(tmp.path().join("docs/baseline.md").is_file());
        assert!(report.failed.is_empty());
    }

    #[test]
    fn run_baseline_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        run_baseline(tmp.path(), BaselineKind::Code);
        let f = tmp.path().join("docs/baseline.md");
        fs::write(&f, "EDITED").unwrap();
        let second = run_baseline(tmp.path(), BaselineKind::Code);
        assert!(second.created.is_empty(), "second run creates nothing");
        assert_eq!(fs::read_to_string(&f).unwrap(), "EDITED", "not overwritten");
    }
}
