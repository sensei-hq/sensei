//! Reading a Svelte component: the script block, and the markup expressions
//! around it (`docs/spec/indexer/04b-walk-js.md` §3, §4).
//!
//! A `.svelte` file holds three languages. The style block is CSS and declares
//! nothing this graph names. The script block is JavaScript or TypeScript and
//! goes straight to `super::javascript`, at an offset, so a span reported from
//! it is one a reader can click in the original file.
//!
//! # Markup expressions are use sites, and this is the decision 04b §4 asked for
//!
//! `{items.length}` and `{store.load()}` are calls and reads exactly as they
//! would be inside the script — the same names, resolved through the same
//! imports — and a component that does its work in markup would otherwise show
//! up as a file with no use sites at all. In this repository's three SvelteKit
//! apps that is most components.
//!
//! So they are READ, not skipped. Each interpolation is parsed as an
//! expression, in the script's own dialect, and walked with the script's
//! bindings in scope. What is NOT read is stated just as explicitly: a closing
//! tag and `{:else}` name nothing, so they emit nothing; a block tag whose
//! expression will not parse emits an `UnhandledForm` miss, so the histogram
//! says what was not understood rather than saying nothing (R2, S8).
//
// These modules have no caller on purpose — see the note in `indexer/mod.rs`.
#![allow(dead_code)]

use super::javascript;
use super::{LanguageAdapter, ReadError, Source, TypeHomes};
use crate::indexer::facts::{FileFacts, Fqn, Language};
use crate::indexer::fqn::FqnError;
use crate::indexer::resolve::Grammar;

/// Svelte, as the registry sees it. It DELEGATES to TypeScript rather than
/// inheriting from it — see [`LanguageAdapter::host`] — which is why a
/// `.svelte` file's symbols are filed under the TypeScript language.
pub struct SvelteAdapter;

impl LanguageAdapter for SvelteAdapter {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn name(&self) -> &'static str {
        "svelte"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[".svelte"]
    }

    fn grammar(&self) -> &'static Grammar {
        &javascript::GRAMMAR
    }

    fn read(&self, source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError> {
        read(source, types)
    }

    fn file_fqn(&self, package: &str, module: &str, path: &str) -> Result<Fqn, FqnError> {
        javascript::file_fqn(package, module, path)
    }

    fn module_path(&self, file: &str, package_root: &str) -> String {
        javascript::module_path(file, package_root)
    }

    fn type_segment(&self, raw: &str) -> Result<String, FqnError> {
        javascript::type_segment(raw)
    }

    fn host(&self) -> Option<&'static str> {
        Some("typescript")
    }
}

/// Parse one component once (R1) and return everything that parse saw.
pub fn read(source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError> {
    let from = javascript::file_fqn(source.package, source.module, source.path)
        .map_err(ReadError::NoFileIdentity)?;

    let blocks = script_blocks(source.text);
    // ONE walk over the blocks and the markup — see `read_component` for why
    // two would leave the markup typing nothing.
    let found = javascript::read_component(source, types, &blocks, from)?;

    Ok(FileFacts {
        language: Language::TypeScript,
        package: source.package.to_string(),
        module: source.module.to_string(),
        path: source.path.to_string(),
        symbols: found.symbols,
        references: found.references,
        relations: found.relations,
        imports: found.imports,
    })
}

/// A `<script>` block's CONTENT, as a byte range in the whole file.
///
/// Found by scanning rather than by parsing, because there is no Svelte
/// grammar in this build and the two things needed from the markup — where the
/// script is, and where the interpolations are — are both lexical. A scanner
/// that is wrong about a block is wrong in one direction only: it reads less,
/// never something that is not there.
pub(super) struct ScriptBlock {
    pub start: u32,
    pub end: u32,
    /// Whether the opening tag said `lang="ts"`.
    pub typescript: bool,
}

pub(super) fn script_blocks(text: &str) -> Vec<ScriptBlock> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut at = 0usize;
    while let Some(open) = text[at..].find("<script").map(|i| i + at) {
        // `<scriptish` is not a script tag. The character after the name has to
        // end it.
        let after = open + "<script".len();
        if bytes.get(after).is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'-') {
            at = after;
            continue;
        }
        let Some(tag_end) = text[open..].find('>').map(|i| i + open) else {
            break;
        };
        let tag = &text[open..tag_end];
        let typescript = tag.contains("lang=\"ts\"")
            || tag.contains("lang='ts'")
            || tag.contains("lang=ts")
            || tag.contains("lang=\"typescript\"");
        let body = tag_end + 1;
        let Some(close) = text[body..].find("</script").map(|i| i + body) else {
            break;
        };
        out.push(ScriptBlock { start: body as u32, end: close as u32, typescript });
        at = close + "</script".len();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::facts::{Observation, RefKind, Resolution};

    fn facts(text: &str) -> FileFacts {
        read(
            &Source { package: "pkg", module: "lib/Comp", path: "src/lib/Comp.svelte", text },
            &TypeHomes::unknown(),
        )
        .expect("the component reads")
    }

    fn named(facts: &FileFacts, name: &str) -> Vec<String> {
        facts
            .references
            .iter()
            .map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => ("?".to_string(), fqn.to_string()),
                Resolution::Unresolved { reason, evidence } => (
                    evidence.name.clone(),
                    evidence
                        .saw
                        .iter()
                        .find_map(|o| match o {
                            Observation::Candidate(fqn) => Some(fqn.to_string()),
                            _ => None,
                        })
                        .unwrap_or_else(|| format!("{reason:?}")),
                ),
            })
            .filter(|(n, _)| n == name)
            .map(|(_, t)| t)
            .collect()
    }

    /// The script block is read at an OFFSET, so a declaration inside it
    /// reports the line it is on in the FILE — not the line it is on in the
    /// block. MUTATION: parse the block with no offset; every span shifts up by
    /// the number of markup lines above it.
    #[test]
    fn a_declaration_in_the_script_reports_its_line_in_the_whole_file() {
        let facts = facts("<p>markup</p>\n<p>more</p>\n<script>\nfunction go() {}\n</script>\n");
        let go = facts.symbols.iter().find(|s| s.name == "go").expect("the function is a symbol");
        assert_eq!(go.span.start_line, 4, "line 4 of the file, not line 2 of the block");
    }

    /// 04b §3: a component with `lang="ts"` has annotations, and one without
    /// has none. MUTATION: read every block as TypeScript — a plain `<script>`
    /// then parses `x as y` as an assertion instead of failing, which is a
    /// dialect the file is not written in.
    #[test]
    fn the_script_dialect_is_the_one_the_block_states() {
        let ts = facts(
            "<script lang=\"ts\">\nclass T { m() {} }\nfunction go(x: T) { x.m(); }\n</script>\n",
        );
        assert_eq!(
            named(&ts, "m").len(),
            1,
            "the annotated parameter types the receiver in a ts block"
        );
        assert!(named(&ts, "m")[0].ends_with("T·m·item"), "{:?}", named(&ts, "m"));

        let js = facts("<script>\nclass T { m() {} }\nconst x = new T();\nx.m();\n</script>\n");
        assert!(named(&js, "m")[0].ends_with("T·m·item"), "new T() works with no annotations");
    }

    /// **The decision 04b §4 asked for, made and tested.** A markup
    /// interpolation is a use site. MUTATION: skip the markup — the call
    /// disappears from the graph entirely and the component looks inert.
    #[test]
    fn a_markup_interpolation_is_a_use_site_and_sees_the_scripts_bindings() {
        let facts = facts(
            "<script>\nclass Store { load() {} }\nconst store = new Store();\n</script>\n\
             <button onclick={() => store.load()}>go</button>\n<p>{store.load()}</p>\n",
        );
        let calls = named(&facts, "load");
        assert_eq!(calls.len(), 2, "both interpolations are use sites: {calls:?}");
        for call in &calls {
            assert!(call.ends_with("Store·load·item"), "the script's binding types it: {call:?}");
        }
    }

    /// A block tag's EXPRESSION is a use site; the tag's keyword is not, and a
    /// closing tag names nothing at all. MUTATION: emit a reference for every
    /// `{...}` — `{/if}` and `{:else}` then flood the histogram with misses
    /// that are not misses.
    #[test]
    fn a_block_tags_expression_is_read_and_its_keyword_and_closer_are_not() {
        let facts = facts(
            "<script>\nclass Store { load() {} }\nconst store = new Store();\n</script>\n\
             {#if store.load()}\n<p>yes</p>\n{:else}\n<p>no</p>\n{/if}\n",
        );
        let calls = named(&facts, "load");
        assert_eq!(calls.len(), 1, "the `#if` test is the one use site here: {calls:?}");
        assert!(calls[0].ends_with("Store·load·item"));
        assert!(
            !facts.references.iter().any(|r| matches!(
                &r.target,
                Resolution::Unresolved { evidence, .. } if evidence.name.contains("/if")
                    || evidence.name.contains(":else")
            )),
            "a closer and an else name nothing, so they emit nothing"
        );
    }

    /// A component with no script is a component, not a failed read: its markup
    /// is still use sites, and `Ok` with no symbols is the honest answer.
    #[test]
    fn a_component_with_no_script_reads_its_markup_and_declares_nothing() {
        let facts = facts("<p>{title}</p>\n");
        assert!(facts.symbols.is_empty(), "nothing is declared here");
        assert_eq!(facts.language, Language::TypeScript);
    }

    /// The style block is CSS. Reading it as script would produce a tree full
    /// of errors and facts from none of it.
    #[test]
    fn the_style_block_is_not_script_and_a_scriptish_tag_is_not_either() {
        let blocks =
            script_blocks("<style>\np { color: red; }\n</style>\n<script>let a = 1;</script>\n");
        assert_eq!(blocks.len(), 1, "one script block, and the style is not it");
        assert!(!blocks[0].typescript, "a plain `<script>` states no dialect");
        assert!(script_blocks("<scriptish>x</scriptish>").is_empty(), "the tag name must end");
    }

    /// Both blocks, and both at their own offsets. A `context="module"` block
    /// declares module-level bindings the instance block uses.
    #[test]
    fn a_component_with_two_script_blocks_reads_both() {
        let facts = facts(
            "<script context=\"module\">\nexport function shared() {}\n</script>\n\
             <script>\nfunction local() {}\n</script>\n",
        );
        let names: Vec<&str> = facts.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"shared"), "the module block declares: {names:?}");
        assert!(names.contains(&"local"), "the instance block declares: {names:?}");
    }

    /// Markup use sites are filed under the FILE, because there is no enclosing
    /// declaration in markup for them to belong to.
    #[test]
    fn a_markup_use_site_is_filed_under_the_component_itself() {
        let facts = facts("<script>\nconst n = 1;\n</script>\n<p>{n.toFixed()}</p>\n");
        let call = facts
            .references
            .iter()
            .find(|r| r.kind == RefKind::Calls)
            .expect("the markup call is a reference");
        assert_eq!(call.from.to_string(), "typescript·pkg·lib·Comp·mod");
    }
}
