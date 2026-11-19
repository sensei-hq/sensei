//! The supported tools' names and kinds, as the seeded catalogue gives them (#455).
//!
//! `sensei.assistant_catalogue` owns every tool's display name and kind; the
//! `sensei.assistant_catalogue_words` view resolves any spelling the system writes
//! (an id or an alias) to its row. [`CatalogueNames`] holds that view, read once per
//! request, and answers "what is this word called, and what is it" for every surface
//! that names a tool. A word with no row is unknown: it is named from its own words
//! and never classified as an assistant or an editor.
use crate::db::pg_store::CatalogueWordRow;
use std::collections::HashMap;

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
}
