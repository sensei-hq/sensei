//! Deterministic project-icon inference — fills the generic 場 fallback so
//! every project gets a meaningful icon at a glance. See
//! [[pipeline/project-icon]] for the design.
//!
//! The scanner writes `sensei.projects.icon` (jsonb `{kind, value, source}`).
//! The app reads `kind` + `value` only (`kind:"image"` → `<img src=value>`,
//! `kind:"kanji"` → glyph, else → 場); `source` is provenance the app ignores.
//!
//! The *choice* is pure and unit-testable: [`infer_icon`] takes the project
//! name, its stack, the current icon, and the set of logo/asset paths already
//! found on disk (fed by the existing `scan_icons` filesystem walk). No disk,
//! no clock — deterministic.
//!
//! Voice: sentence case, lowercase "sensei", no filler.

use serde::Serialize;
use serde_json::Value;

/// The last-resort glyph — 場 (place). Reused from the project-overview
/// read-path so the write and read sides agree on the single fallback.
use crate::project_overview::DEFAULT_KANJI;

// Provenance tags (also serialized into `icon.source`). Ordered by priority —
// lower rank wins. Kept in one place so the guard and the chain agree.
const SRC_README: &str = "readme";
const SRC_LOGO: &str = "logo_file";
const SRC_FAVICON: &str = "favicon";
const SRC_PACKAGE: &str = "package_branding";
const SRC_KANJI: &str = "kanji_map";
const SRC_LETTER: &str = "letter_fallback";
const SRC_DEFAULT: &str = "default";

/// Priority rank of a machine-inference `source` (lower = higher priority).
/// `None` marks a source sensei never produces — i.e. an author/About-form
/// choice, which the guard must never clobber.
fn source_rank(source: &str) -> Option<u8> {
    match source {
        SRC_README => Some(0),
        SRC_LOGO => Some(1),
        SRC_FAVICON => Some(2),
        SRC_PACKAGE => Some(3),
        SRC_KANJI => Some(4),
        SRC_LETTER => Some(5),
        SRC_DEFAULT => Some(6),
        _ => None,
    }
}

/// An inferred icon, serialized verbatim into `sensei.projects.icon`.
///
/// `kind` is always one of the two the app renders (`"image"` or `"kanji"`);
/// the letter fallback rides on `kind:"kanji"` so a Latin initial renders as a
/// glyph today (the app has no `letter` arm — see module notes / spec deviation).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Icon {
    pub kind: &'static str,
    pub value: String,
    pub source: &'static str,
}

/// The write decision for a project's icon.
///
/// `Keep` means leave the stored icon untouched — it is either an author choice
/// or an equal-or-higher-priority machine icon (re-writing would churn
/// `modified_at` or flip-flop between a project's repos). `Set` carries the icon
/// to persist.
#[derive(Debug, Clone, PartialEq)]
pub enum IconDecision {
    Keep,
    Set(Icon),
}

/// Whether the stored icon is an author/human choice (About form or a
/// create endpoint) that inference must never override. A choice is "author-set"
/// when it has a non-empty `value` and its `source` is absent or is not one of
/// sensei's inference tags.
fn is_author_set(existing: &Value) -> bool {
    let has_value = existing
        .get("value")
        .and_then(Value::as_str)
        .is_some_and(|v| !v.trim().is_empty());
    if !has_value {
        return false;
    }
    match existing.get("source").and_then(Value::as_str) {
        Some(s) => source_rank(s).is_none(),
        None => true,
    }
}

/// The priority rank of a prior *machine* icon, or `None` when the stored icon
/// is empty (no value) — an empty slot is always writable.
fn machine_rank(existing: &Value) -> Option<u8> {
    let has_value = existing
        .get("value")
        .and_then(Value::as_str)
        .is_some_and(|v| !v.trim().is_empty());
    if !has_value {
        return None;
    }
    existing.get("source").and_then(Value::as_str).and_then(source_rank)
}

/// Decide the icon to persist for a project. Deterministic — the same inputs
/// always yield the same decision.
///
/// Priority (first hit wins):
/// 1. **explicit** — an author/About-form icon is kept as-is ([`IconDecision::Keep`]).
/// 2. **logo / image file** — any asset path from the filesystem walk
///    (`scan_icons`), stored as `kind:"image"` with the repo-relative path.
/// 3. **kanji from stack** — a domain kanji for the dominant mapped stack.
/// 4. **letter** — the project name's first alphabetic character, uppercased.
/// 5. **default** — 場, only for a nameless, stackless, logoless project.
///
/// A prior machine icon is only replaced by a strictly higher-priority source,
/// so re-scans neither churn nor flip-flop between a project's member repos.
pub fn infer_icon(
    name: &str,
    stack: &[String],
    existing_icon: &Value,
    present_asset_paths: &[String],
) -> IconDecision {
    // 1. Never override a human choice.
    if is_author_set(existing_icon) {
        return IconDecision::Keep;
    }

    let candidate = choose_icon(name, stack, present_asset_paths);

    // Only upgrade a prior machine icon — equal or lower priority is a no-op.
    if let Some(existing) = machine_rank(existing_icon)
        && source_rank(candidate.source).unwrap_or(u8::MAX) >= existing
    {
        return IconDecision::Keep;
    }
    IconDecision::Set(candidate)
}

/// The best fresh icon for the current repo state, ignoring what is stored.
fn choose_icon(name: &str, stack: &[String], present_asset_paths: &[String]) -> Icon {
    // 2. Logo / image file — the first non-empty path the walk surfaced.
    if let Some(path) = present_asset_paths.iter().find(|p| !p.trim().is_empty()) {
        return Icon { kind: "image", value: path.clone(), source: SRC_LOGO };
    }
    // 3. Kanji from the dominant mapped stack.
    if let Some(k) = kanji_from_stack(stack) {
        return Icon { kind: "kanji", value: k.to_string(), source: SRC_KANJI };
    }
    // 4. Letter from the name (rides on kind:"kanji" so it renders today).
    if let Some(letter) = letter_from_name(name) {
        return Icon { kind: "kanji", value: letter, source: SRC_LETTER };
    }
    // 5. Documented last resort.
    Icon { kind: "kanji", value: DEFAULT_KANJI.to_string(), source: SRC_DEFAULT }
}

/// The first alphabetic character of `name` uppercased, falling back to the
/// first alphanumeric character. `None` for an empty / symbol-only name.
fn letter_from_name(name: &str) -> Option<String> {
    name.chars()
        .find(|c| c.is_alphabetic())
        .or_else(|| name.chars().find(|c| c.is_alphanumeric()))
        .map(|c| c.to_uppercase().to_string())
}

/// A domain kanji for the dominant stack — the first stack label (in order)
/// that has a mapping. `None` when no label is mapped (→ letter fallback).
pub fn kanji_from_stack(stack: &[String]) -> Option<&'static str> {
    stack.iter().find_map(|s| kanji_for(s))
}

/// The kanji for a single stack label, or `None` if unmapped. Labels arrive
/// lowercase from the detector and frontmatter; normalized here defensively.
fn kanji_for(label: &str) -> Option<&'static str> {
    let l = label.trim().to_ascii_lowercase();
    KANJI_MAP.iter().find(|(k, _)| *k == l).map(|(_, v)| *v)
}

/// Stack label → single domain kanji. Covers every label the scanner's
/// `detect_stack` can emit, plus common frontmatter-supplied stacks.
///
/// Motivated where possible: 鉄 iron (rust), 蛇 snake (python), 象 elephant
/// (php), 燕 swift-bird (swift), 雪 snow (svelte), 碁 the game go, 玉 gem (ruby),
/// 次 next (nextjs), 網 net (dotnet), 珈 coffee (java), 築 build (gradle/maven),
/// 型 type / 文 script, 景 view (vue), 応 reaction (react).
const KANJI_MAP: &[(&str, &str)] = &[
    // Languages / runtimes the detector emits.
    ("rust", "鉄"),
    ("go", "碁"),
    ("python", "蛇"),
    ("ruby", "玉"),
    ("php", "象"),
    ("swift", "燕"),
    ("java", "珈"),
    ("dotnet", "網"),
    ("typescript", "型"),
    // Build tools (emitted alongside "java", so rarely dominant).
    ("gradle", "築"),
    ("maven", "築"),
    // Frontend frameworks the npm adapter emits.
    ("svelte", "雪"),
    ("react", "応"),
    ("vue", "景"),
    ("nextjs", "次"),
    // Common frontmatter-supplied labels (spec-listed).
    ("javascript", "文"),
    ("sql", "庫"),
    ("docs", "書"),
    ("tauri", "匠"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn paths(p: &[&str]) -> Vec<String> {
        p.iter().map(|s| s.to_string()).collect()
    }
    fn stack(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    // ── priority order ────────────────────────────────────────────────

    #[test]
    fn explicit_author_icon_wins() {
        // An About-form kanji (no source) is kept even when a logo exists.
        let existing = json!({ "kind": "kanji", "value": "禅" });
        assert_eq!(
            infer_icon("proj", &stack(&["rust"]), &existing, &paths(&["logo.svg"])),
            IconDecision::Keep
        );
        // A non-machine source is also an author choice.
        let picked = json!({ "kind": "image", "value": "x.png", "source": "user_upload" });
        assert_eq!(
            infer_icon("proj", &stack(&["rust"]), &picked, &paths(&["logo.svg"])),
            IconDecision::Keep
        );
    }

    #[test]
    fn logo_file_beats_kanji() {
        let d = infer_icon("proj", &stack(&["rust"]), &json!({}), &paths(&["assets/logo.svg"]));
        assert_eq!(
            d,
            IconDecision::Set(Icon {
                kind: "image",
                value: "assets/logo.svg".into(),
                source: SRC_LOGO,
            })
        );
    }

    #[test]
    fn logo_picks_first_present_path() {
        // The pure chooser takes the first non-empty candidate (the walk ranks).
        let d = infer_icon("proj", &[], &json!({}), &paths(&["", "logo.png", "icon.svg"]));
        match d {
            IconDecision::Set(Icon { kind: "image", value, .. }) => assert_eq!(value, "logo.png"),
            other => panic!("expected image, got {other:?}"),
        }
    }

    #[test]
    fn kanji_from_stack_for_each_mapped_stack() {
        let cases = [
            ("rust", "鉄"),
            ("go", "碁"),
            ("python", "蛇"),
            ("ruby", "玉"),
            ("php", "象"),
            ("swift", "燕"),
            ("java", "珈"),
            ("dotnet", "網"),
            ("typescript", "型"),
            ("svelte", "雪"),
            ("react", "応"),
            ("vue", "景"),
            ("nextjs", "次"),
        ];
        for (label, glyph) in cases {
            let d = infer_icon("proj", &stack(&[label]), &json!({}), &[]);
            assert_eq!(
                d,
                IconDecision::Set(Icon { kind: "kanji", value: glyph.into(), source: SRC_KANJI }),
                "stack {label} should map to {glyph}"
            );
        }
    }

    #[test]
    fn dominant_stack_is_first_mapped_label() {
        // sensei's monorepo frontmatter stack — rust wins over sveltekit.
        let d = infer_icon("sensei", &stack(&["rust", "sveltekit"]), &json!({}), &[]);
        assert_eq!(
            d,
            IconDecision::Set(Icon { kind: "kanji", value: "鉄".into(), source: SRC_KANJI })
        );
    }

    #[test]
    fn unmapped_stack_falls_through_to_letter() {
        // "cobol" has no kanji → letter from the name.
        let d = infer_icon("cobolapp", &stack(&["cobol"]), &json!({}), &[]);
        assert_eq!(
            d,
            IconDecision::Set(Icon { kind: "kanji", value: "C".into(), source: SRC_LETTER })
        );
    }

    #[test]
    fn letter_fallback_uppercases_first_alpha() {
        let d = infer_icon("sensei", &[], &json!({}), &[]);
        assert_eq!(
            d,
            IconDecision::Set(Icon { kind: "kanji", value: "S".into(), source: SRC_LETTER })
        );
        // Leading non-alpha is skipped; first alphabetic char is used.
        assert_eq!(letter_from_name("42tools").as_deref(), Some("T"));
        // A CJK name yields its first glyph.
        assert_eq!(letter_from_name("禅道").as_deref(), Some("禅"));
        // No alpha → first alphanumeric.
        assert_eq!(letter_from_name("42").as_deref(), Some("4"));
    }

    #[test]
    fn empty_or_symbol_name_with_no_stack_uses_documented_default() {
        for name in ["", "   ", "@#$"] {
            let d = infer_icon(name, &[], &json!({}), &[]);
            assert_eq!(
                d,
                IconDecision::Set(Icon {
                    kind: "kanji",
                    value: DEFAULT_KANJI.into(),
                    source: SRC_DEFAULT,
                }),
                "name {name:?} should fall to the documented default"
            );
        }
    }

    // ── re-scan / idempotence guard ───────────────────────────────────

    #[test]
    fn same_machine_icon_is_kept_no_churn() {
        // A prior kanji_map icon, re-inferred identically → Keep (no re-write).
        let existing = json!({ "kind": "kanji", "value": "鉄", "source": SRC_KANJI });
        assert_eq!(infer_icon("proj", &stack(&["rust"]), &existing, &[]), IconDecision::Keep);
    }

    #[test]
    fn higher_priority_source_upgrades_a_machine_icon() {
        // A kanji_map icon is upgraded when a logo appears.
        let existing = json!({ "kind": "kanji", "value": "鉄", "source": SRC_KANJI });
        let d = infer_icon("proj", &stack(&["rust"]), &existing, &paths(&["logo.svg"]));
        assert_eq!(
            d,
            IconDecision::Set(Icon { kind: "image", value: "logo.svg".into(), source: SRC_LOGO })
        );
    }

    #[test]
    fn lower_priority_source_never_downgrades() {
        // An image icon is not replaced by a kanji when the logo path is gone
        // this pass (multi-repo flip-flop / transient-miss guard).
        let existing = json!({ "kind": "image", "value": "logo.svg", "source": SRC_LOGO });
        assert_eq!(infer_icon("proj", &stack(&["rust"]), &existing, &[]), IconDecision::Keep);
    }

    #[test]
    fn empty_icon_is_always_written() {
        // The default `{}` slot is writable.
        assert!(matches!(
            infer_icon("proj", &stack(&["rust"]), &json!({}), &[]),
            IconDecision::Set(_)
        ));
        assert!(matches!(
            infer_icon("proj", &stack(&["rust"]), &Value::Null, &[]),
            IconDecision::Set(_)
        ));
    }

    // ── kanji map totality ────────────────────────────────────────────

    /// Every stack label `scan_logic::detect_stack` + the manifest adapters can
    /// emit. If a new detector label lands without a kanji, this test fails —
    /// forcing a mapping or a documented fallthrough decision.
    const DETECTOR_STACK_LABELS: &[&str] = &[
        "rust", "ruby", "php", "svelte", "react", "vue", "nextjs", "typescript",
        "python", "dotnet", "java", "gradle", "maven", "go", "swift",
    ];

    #[test]
    fn kanji_map_is_total_for_detector_labels() {
        for label in DETECTOR_STACK_LABELS {
            assert!(
                kanji_for(label).is_some(),
                "detector stack label {label} has no kanji mapping"
            );
        }
    }

    #[test]
    fn icon_serializes_to_the_wire_shape() {
        let icon = Icon { kind: "kanji", value: "鉄".into(), source: SRC_KANJI };
        assert_eq!(
            serde_json::to_value(&icon).unwrap(),
            json!({ "kind": "kanji", "value": "鉄", "source": "kanji_map" })
        );
    }
}
