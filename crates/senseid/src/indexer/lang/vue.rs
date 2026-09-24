//! Reading a Vue single-file component: the script block, and the template
//! expressions around it.
//!
//! The sibling of [`super::svelte`], and deliberately the same shape. A `.vue`
//! file holds three languages: the style block is CSS and declares nothing this
//! graph names, and the script block is JavaScript or TypeScript and goes
//! straight to [`super::javascript`] at an offset, so a span reported from it is
//! one a reader can click in the original file.
//!
//! # What differs from Svelte, and it is only the markup
//!
//! The script is the same language read the same way — `<script setup>` and
//! `<script>` are both just script blocks, and `lang="ts"` states the dialect
//! exactly as it does in a Svelte component, so the SAME scanner finds them.
//!
//! The TEMPLATE is where the two part. Vue spells an interpolation `{{ expr }}`
//! and puts its other expressions in QUOTED ATTRIBUTE VALUES — `:prop="expr"`,
//! `@click="expr"`, `v-if="expr"` — where Svelte uses a single brace throughout.
//! Reading a Vue template with Svelte's rule would take `{ expr }` out of
//! `{{ expr }}` and then fail to parse it, filing a miss for every
//! interpolation in the file. So the region finder is chosen by dialect
//! (`javascript::Markup`) and everything after it — parse, walk, resolve — is
//! shared.
//!
//! # Why this exists at all
//!
//! Vue was the last language the legacy indexer still owned that this one did
//! not, and `languages/typescript.rs` could not be deleted while
//! `languages/vue.rs` parsed its `<script>` through it. One producer per
//! language is the invariant that makes the graph joinable; a Vue file read by
//! the legacy producer would mint identities in a scheme nothing here can match.

use super::javascript::{self, Markup};
use super::{LanguageAdapter, ReadError, Source, TypeHomes};
use crate::indexer::facts::{FileFacts, Fqn, Language};
use crate::indexer::fqn::FqnError;
use crate::indexer::resolve::Grammar;

/// Vue, as the registry sees it. It DELEGATES to TypeScript rather than
/// inheriting from it — see [`LanguageAdapter::host`] — which is why a `.vue`
/// file's symbols are filed under the TypeScript language.
pub struct VueAdapter;

impl LanguageAdapter for VueAdapter {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn name(&self) -> &'static str {
        "vue"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[".vue"]
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

    // The SAME scanner Svelte uses: a `<script>` tag is a `<script>` tag, and
    // `lang="ts"` states the dialect the same way. A second copy here would be
    // a second thing to keep in step with it.
    let blocks = super::svelte::script_blocks(source.text);
    let mut found = javascript::read_component(source, types, &blocks, from.clone(), Markup::Vue)?;
    // A component is a module like any other file. See `common::file_module`.
    let its_own = from.clone();
    found.symbols.insert(
        0,
        super::common::file_module(from, javascript::module_name_of(source.module), source.text),
    );
    let entered = super::common::import_references(Language::TypeScript, &found.imports, &its_own);
    found.references.extend(entered);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::facts::{Observation, RefKind, Resolution};

    fn facts(text: &str) -> FileFacts {
        read(
            &Source { package: "pkg", module: "lib/Comp", path: "src/lib/Comp.vue", text },
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
                            Observation::Candidate(fqn) | Observation::Named(fqn) => {
                                Some(fqn.to_string())
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| format!("{reason:?}")),
                ),
            })
            .filter(|(n, _)| n == name)
            .map(|(_, t)| t)
            .collect()
    }

    /// The script block is read at an OFFSET, so a declaration inside it reports
    /// the line it is on in the FILE.
    ///
    /// MUTATION: parse the block with no offset; every span shifts up by the
    /// number of template lines above it.
    #[test]
    fn a_declaration_in_the_script_reports_its_line_in_the_whole_file() {
        let facts =
            facts("<template>\n<p>hi</p>\n</template>\n<script>\nfunction go() {}\n</script>\n");
        let go = facts.symbols.iter().find(|s| s.name == "go").expect("the function is a symbol");
        assert_eq!(go.span.start_line, 5, "line 5 of the file, not line 2 of the block");
    }

    /// `<script setup lang="ts">` is a script block stating TypeScript. Vue's
    /// `setup` attribute changes what the block MEANS at runtime, not what
    /// language it is written in.
    ///
    /// MUTATION: read every block as JavaScript — the annotation then fails to
    /// parse and the component reads as `NotParsed`.
    #[test]
    fn a_script_setup_block_states_its_dialect_like_any_other() {
        let ts = facts(
            "<script setup lang=\"ts\">\nclass T { m() {} }\nfunction go(x: T) { x.m(); }\n</script>\n",
        );
        assert_eq!(named(&ts, "m").len(), 1, "the annotated parameter types the receiver");
        assert!(named(&ts, "m")[0].ends_with("T·m·item"), "{:?}", named(&ts, "m"));
    }

    /// **The Vue half of 04b §4.** A mustache is a use site and sees the
    /// script's bindings — and it is spelled with TWO braces, which is the whole
    /// reason this dialect needed its own region finder.
    ///
    /// MUTATION: read the region with `markup_expression` instead of
    /// `vue_expression` — Svelte's reader strips ONE brace, leaving
    /// `{ store.load() }`, which parses as a block rather than an expression, so
    /// the call becomes an `UnhandledForm` miss.
    ///
    /// Note which half that names. The two REGION finders agree here — both
    /// return `{{ store.load() }}`, verified by probing them side by side — so
    /// swapping the region finder proves nothing about this test and the
    /// expression reader is the whole of what it covers.
    #[test]
    fn a_mustache_interpolation_is_a_use_site_and_sees_the_scripts_bindings() {
        let facts = facts(
            "<script>\nclass Store { load() {} }\nconst store = new Store();\n</script>\n\
             <template>\n<p>{{ store.load() }}</p>\n</template>\n",
        );
        let calls = named(&facts, "load");
        assert_eq!(calls.len(), 1, "the mustache is a use site: {calls:?}");
        assert!(calls[0].ends_with("Store·load·item"), "the script's binding types it: {calls:?}");
    }

    /// A DIRECTIVE's expression is its quoted attribute value. A component whose
    /// work happens in `@click` would otherwise show up inert.
    ///
    /// MUTATION: drop the directive arm of `vue_interpolations` — all three
    /// calls disappear and the component has no use sites at all. This is the
    /// one Vue test the REGION finder owns: a directive value is not between
    /// braces, so nothing but that arm can find it.
    #[test]
    fn a_directive_value_is_a_use_site() {
        let facts = facts(
            "<script>\nclass Store { load() {} save() {} drop() {} }\n\
             const store = new Store();\n</script>\n\
             <template>\n\
             <button @click=\"store.load()\">go</button>\n\
             <p v-if=\"store.save()\">yes</p>\n\
             <x :value=\"store.drop()\" />\n\
             </template>\n",
        );
        for method in ["load", "save", "drop"] {
            let calls = named(&facts, method);
            assert_eq!(calls.len(), 1, "`{method}` is named by its directive: {calls:?}");
            assert!(calls[0].ends_with(&format!("Store·{method}·item")), "{calls:?}");
        }
    }

    /// A PLAIN attribute is a string, not an expression. Reading one would put a
    /// miss in the histogram for every attribute in the file.
    ///
    /// MUTATION: treat any quoted attribute value as a region — `class="row"`
    /// becomes a reference to a name nothing declares.
    #[test]
    fn a_plain_attribute_is_not_an_expression() {
        let facts = facts("<template>\n<p class=\"row\" id=\"main\">hi</p>\n</template>\n");
        assert!(
            facts.references.is_empty(),
            "a plain attribute names nothing: {:?}",
            facts.references
        );
    }

    /// A component with no script is a component, not a failed read.
    #[test]
    fn a_component_with_no_script_declares_only_itself() {
        let facts = facts("<template>\n<p>hi</p>\n</template>\n");
        assert_eq!(
            facts.symbols.iter().map(|s| s.fqn.as_str()).collect::<Vec<&str>>(),
            vec!["typescript·pkg·lib·Comp·mod"],
            "a component still declares its own module"
        );
        assert_eq!(facts.language, Language::TypeScript);
    }

    /// Template use sites are filed under the FILE, because there is no
    /// enclosing declaration in a template for them to belong to.
    #[test]
    fn a_template_use_site_is_filed_under_the_component_itself() {
        let facts =
            facts("<script>\nconst n = 1;\n</script>\n<template>{{ n.toFixed() }}</template>\n");
        let call = facts
            .references
            .iter()
            .find(|r| r.kind == RefKind::Calls)
            .expect("the template call is a reference");
        assert_eq!(call.from.to_string(), "typescript·pkg·lib·Comp·mod");
    }
}
