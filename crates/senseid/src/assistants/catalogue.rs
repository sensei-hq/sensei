//! The supported tools' names and kinds, as the seeded catalogue gives them (#455).
//!
//! `sensei.assistant_catalogue` owns every tool's display name and kind; the
//! `sensei.assistant_catalogue_words` view resolves any spelling the system writes
//! (an id or an alias) to its row. [`CatalogueNames`] holds that view, read once per
//! request, and answers "what is this word called, and what is it" for every surface
//! that names a tool. A word with no row is unknown: it is named from its own words
//! and never classified as an assistant or an editor.
use std::collections::HashMap;

/// One spelling of a supported tool, as `sensei.assistant_catalogue_words`
/// serves it: the word, the tool it names, its display name and kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueWordRow {
    pub word: String,
    pub id: String,
    pub name: String,
    /// A `sensei.tool_kind` label: `assistant` or `editor`.
    pub kind: String,
    pub joins_crew: bool,
    /// One command string that starts this tool, with a `{repo}` placeholder
    /// (#513). `None` when the invocation is not known.
    pub launch_template: Option<String>,
}

/// The kind a word with no catalogue row reads as.
pub const UNKNOWN_KIND: &str = "unknown";

/// The catalogue's words, keyed by spelling.
#[derive(Debug, Clone, Default)]
pub struct CatalogueNames {
    by_word: HashMap<String, CatalogueWordRow>,
}

impl CatalogueNames {
    pub fn from_rows(rows: Vec<CatalogueWordRow>) -> Self {
        Self { by_word: rows.into_iter().map(|r| (r.word.clone(), r)).collect() }
    }

    /// The catalogue row a word resolves to, if any.
    pub fn entry(&self, word: &str) -> Option<&CatalogueWordRow> {
        self.by_word.get(word)
    }

    /// The name people know the tool by; an unknown word made readable.
    pub fn name(&self, word: &str) -> String {
        self.entry(word).map_or_else(|| readable(word), |e| e.name.clone())
    }

    /// `assistant`, `editor`, or `unknown` when the catalogue has no row.
    pub fn kind(&self, word: &str) -> &str {
        self.entry(word).map_or(UNKNOWN_KIND, |e| e.kind.as_str())
    }
}

/// A word made readable: separators to spaces, the first letter capitalised
/// ("gemini-cli" → "Gemini cli", "muse" → "Muse"). Names without classifying.
pub fn readable(word: &str) -> String {
    let words = word.split(['-', '_']).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// How often sessions named a word (an `acp_id` or a `host`).
#[derive(Debug, Clone, PartialEq)]
pub struct WordUsage {
    pub word: String,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sessions_week: i64,
}

/// One stored install, as the catalogue read needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolInstall {
    pub id: String,
    pub installed: bool,
    pub configured: bool,
    pub config_path: Option<String>,
}

#[derive(Default)]
struct Seen {
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    sessions_week: i64,
    models: Vec<String>,
}

/// `GET /relay/catalogue` (#455): the crew-capable catalogue tools this machine
/// has seen (installed, or named by a session), split by kind, and the words
/// sessions named that the catalogue does not know. Pure: the store reads, this
/// shapes. `models` is `(word, model)` newest first. `repo` is the local path
/// `{repo}` resolves against (#513); `None` leaves every `launch` null.
pub fn assemble_catalogue(
    names: &CatalogueNames,
    installs: &[ToolInstall],
    usage: &[WordUsage],
    models: &[(String, String)],
    repo: Option<&str>,
) -> serde_json::Value {
    use std::collections::BTreeMap;
    let mut seen: BTreeMap<String, Seen> = BTreeMap::new();
    let mut unknown: BTreeMap<String, Seen> = BTreeMap::new();
    for u in usage {
        let slot = match names.entry(&u.word) {
            Some(e) => seen.entry(e.id.clone()).or_default(),
            None => unknown.entry(u.word.clone()).or_default(),
        };
        slot.last_used_at = slot.last_used_at.max(u.last_used_at);
        slot.sessions_week += u.sessions_week;
    }
    for (word, model) in models {
        if let Some(e) = names.entry(word)
            && let Some(slot) = seen.get_mut(&e.id)
            && !slot.models.contains(model)
        {
            slot.models.push(model.clone());
        }
    }
    let install_of = |id: &str| installs.iter().find(|i| i.id == id);
    for i in installs.iter().filter(|i| i.installed) {
        seen.entry(i.id.clone()).or_default();
    }
    let iso = |t: Option<chrono::DateTime<chrono::Utc>>| t.map(|t| t.to_rfc3339());
    let mut assistants = Vec::new();
    let mut editors = Vec::new();
    for (id, s) in &seen {
        let Some(e) = names.entry(id).filter(|e| e.joins_crew) else { continue };
        let install = install_of(id);
        let installed = install.map(|i| i.installed);
        // The launch resolves only when both halves exist: a template the seed
        // vouches for, and a repo the request resolved (#513).
        let launch = match (repo, e.launch_template.as_deref()) {
            (Some(path), Some(tpl)) => Some(tpl.replace("{repo}", path)),
            _ => None,
        };
        match e.kind.as_str() {
            "assistant" => assistants.push((
                s.sessions_week,
                s.last_used_at,
                serde_json::json!({
                    "id": id, "name": e.name, "kind": e.kind, "installed": installed,
                    "last_used_at": iso(s.last_used_at), "sessions_week": s.sessions_week,
                    "launch": launch,
                }),
            )),
            "editor" => editors.push((
                s.last_used_at,
                serde_json::json!({
                    "id": id, "name": e.name, "kind": e.kind, "installed": installed,
                    "last_used_at": iso(s.last_used_at), "models": s.models,
                    "mcp_config": install.filter(|i| i.configured).and_then(|i| i.config_path.clone()),
                    "launch": launch,
                }),
            )),
            other => tracing::warn!(id, kind = other, "relay catalogue: a tool of an unexpected kind"),
        }
    }
    // Most used first; a tool never used last (None sorts below Some).
    assistants.sort_by_key(|a| std::cmp::Reverse((a.0, a.1)));
    editors.sort_by_key(|e| std::cmp::Reverse(e.0));
    let unknown: Vec<serde_json::Value> = unknown
        .iter()
        .map(|(word, s)| {
            serde_json::json!({
                "id": word, "name": readable(word), "kind": UNKNOWN_KIND,
                "last_used_at": iso(s.last_used_at),
            })
        })
        .collect();
    serde_json::json!({
        "assistants": assistants.into_iter().map(|a| a.2).collect::<Vec<_>>(),
        "editors": editors.into_iter().map(|e| e.1).collect::<Vec<_>>(),
        "unknown": unknown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(word: &str, id: &str, name: &str, kind: &str, joins_crew: bool) -> CatalogueWordRow {
        CatalogueWordRow {
            word: word.into(),
            id: id.into(),
            name: name.into(),
            kind: kind.into(),
            joins_crew,
            launch_template: None,
        }
    }

    fn names() -> CatalogueNames {
        CatalogueNames::from_rows(vec![
            row("claude", "claude", "Claude Code", "assistant", true),
            row("claude-code", "claude", "Claude Code", "assistant", true),
            row("cursor", "cursor", "Cursor", "editor", true),
            row("zed", "zed", "Zed", "editor", true),
        ])
    }

    #[test]
    fn an_alias_resolves_to_its_tool() {
        let n = names();
        assert_eq!(n.name("claude-code"), "Claude Code");
        assert_eq!(n.kind("claude-code"), "assistant");
        assert_eq!(n.entry("claude-code").map(|e| e.id.as_str()), Some("claude"));
    }

    #[test]
    fn an_editor_is_an_editor() {
        let n = names();
        assert_eq!(n.name("zed"), "Zed");
        assert_eq!(n.kind("cursor"), "editor");
    }

    #[test]
    fn an_unknown_word_is_named_from_its_words_and_never_classified() {
        let n = names();
        assert_eq!(n.name("muse"), "Muse");
        assert_eq!(n.kind("muse"), UNKNOWN_KIND);
        assert!(n.entry("muse").is_none());
        assert_eq!(n.name("some-new_tool"), "Some new tool");
        assert_eq!(n.name("cowork"), "Cowork");
    }

    #[test]
    fn readable_handles_blank_and_separators() {
        assert_eq!(readable(""), "");
        assert_eq!(readable("--"), "");
        assert_eq!(readable("gemini-cli"), "Gemini cli");
    }

    fn templated_names() -> CatalogueNames {
        CatalogueNames::from_rows(vec![
            CatalogueWordRow {
                word: "claude".into(),
                id: "claude".into(),
                name: "Claude Code".into(),
                kind: "assistant".into(),
                joins_crew: true,
                launch_template: Some("cd \"{repo}\" && claude".into()),
            },
            CatalogueWordRow {
                word: "zed".into(),
                id: "zed".into(),
                name: "Zed".into(),
                kind: "editor".into(),
                joins_crew: true,
                launch_template: None,
            },
        ])
    }

    fn usage_of(word: &str) -> WordUsage {
        WordUsage { word: word.into(), last_used_at: None, sessions_week: 1 }
    }

    #[test]
    fn launch_template_resolves_against_the_repo() {
        let usage = vec![usage_of("claude"), usage_of("zed")];
        let out = assemble_catalogue(&templated_names(), &[], &usage, &[], Some("/w/sensei"));
        let assistants = out["assistants"].as_array().unwrap();
        assert_eq!(assistants.len(), 1);
        assert_eq!(assistants[0]["launch"], serde_json::json!("cd \"/w/sensei\" && claude"));
        // No template, no launch — a wrong command is worse than none (#513).
        let editors = out["editors"].as_array().unwrap();
        assert_eq!(editors.len(), 1);
        assert!(editors[0].get("launch").is_none_or(|v| v.is_null()));
    }

    #[test]
    fn launch_is_null_with_no_repo() {
        let usage = vec![usage_of("claude")];
        let out = assemble_catalogue(&templated_names(), &[], &usage, &[], None);
        let assistants = out["assistants"].as_array().unwrap();
        assert!(assistants[0].get("launch").is_none_or(|v| v.is_null()));
    }
}
