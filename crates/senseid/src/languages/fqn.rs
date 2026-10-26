//! What every fqn READER shares: the separator, the external (`lib·`) marker and
//! its SQL spelling, and the receiver hint an edge carries to the resolver.
//!
//! It builds no fqn. `indexer::fqn` is the one encoder (`define` / `refer` over a
//! typed `Form`); this module's v1 joiners minted a second, four-segment scheme
//! that no v2 node carries, and were deleted (#201) rather than deprecated — a
//! second encoder that still compiles is one an author reaches for.

/// FQN segment separator — U+00B7 MIDDLE DOT.
pub const SEP: char = '·';

/// What every external node's FQN starts with: the `lib` segment plus the
/// separator.
///
/// D12 removed `lib_symbol` and `lib_package` from `sensei.node_kind`. Kind says
/// WHAT a node is; this prefix says WHERE it came from. Collapsing the two
/// destroyed the real kind on 18,240 rows to re-state a fact the FQN already
/// carried, so the marker moved here and the kind went back to meaning itself.
///
/// One definition because the alternative is a hand-copied `lib·` literal in
/// every query that asks the question, and a marker with eight copies is a
/// marker that drifts.
pub const LIB_PREFIX: &str = "lib·";

/// The SQL test for "this node came from outside the repo", for the queries
/// that used to ask `kind IN ('lib_symbol','lib_package')`.
///
/// A function rather than a constant because every call site qualifies the
/// column differently (`n.fqn`, `s.fqn`, `p.fqn`) — a bare constant would have
/// been unusable at most of them, which is how nine hand-written copies of the
/// pattern appeared while the constant meant to prevent them sat unused.
pub fn sql_is_external(column: &str) -> String {
    format!("{column} LIKE '{LIB_PREFIX}%'")
}

/// The negation, NULL-safe.
///
/// Its own function because `NOT LIKE` on a NULL yields NULL, not true, so the
/// naive negation silently drops every legacy node that carries no fqn at all.
/// Two call sites had to spell the `IS NULL OR` guard by hand to get this right;
/// one that forgot would filter out rows nobody meant to exclude and report a
/// smaller answer with no error.
pub fn sql_is_not_external(column: &str) -> String {
    format!("({column} IS NULL OR {column} NOT LIKE '{LIB_PREFIX}%')")
}

/// Does this FQN name something outside the repo?
pub fn is_external(fqn: &str) -> bool {
    fqn.starts_with(LIB_PREFIX)
}

/// What a call site SAW of its receiver, expressed as a graph KEY.
///
/// A method call is the one reference shape whose target no import can name:
/// measured on the live graph, all 28,069 unresolved rust `calls` edges have a
/// lowercase `target_name`. The producer had the receiver in hand at the moment
/// it gave up and threw it away, so nothing downstream could finish the job.
/// This carries it instead.
///
/// The payload is an FQN on purpose: a later resolver needs the node table and
/// nothing else — no file path, no re-parse, no second pass over source.
///
/// There is deliberately no "probably" variant. A receiver whose type the file
/// cannot name carries NO hint and the reference stays unresolved, because a
/// plausible key the resolver cannot tell apart from a real one is exactly how
/// ghost nodes get minted.
///
/// ONE variant, because only one shape is ever storable. Props are stamped only
/// on an UNRESOLVED call (see `process.rs`), and the two producer arms that know
/// the receiver's type outright — `self` inside an `impl`, a receiver bound to a
/// first-party type — are the same two arms that already mint a target. A
/// "receiver type" hint therefore attached only to calls that carried an fqn,
/// and every one was computed and dropped on the floor.
///
/// Deleted rather than wired up. What it would buy is real — 2,142 rust `calls`
/// edges in the live `sensei` folder point at a STUB (a target node with no
/// file path), and the receiver's type is a second way to reach the definition
/// those stubs stand in for. What it would cost is re-pointing calls that are
/// ALREADY correctly resolved, which is a strictly worse failure than a stub:
/// the edge would move only when the second lookup disagreed with the first,
/// and nothing here can say which of the two was right. Reaching those 2,142
/// wants a lookup that cannot wrong-merge, not a hint that can.
///
/// Reach, measured by replaying the rust producer over this repo's own 364 rust
/// files with each file's real package and module (30,051 references emitted,
/// 17,067 resolved / 12,984 not): 599 references gain a hint, and every one of
/// them is unresolved — that is the new reach, references that had nothing but a
/// bare method name. 12,385 unresolved references still carry none.
///
/// Every unresolved rust reference is a member call — the `field_expression`
/// arm is the only one that yields an unresolved target — so that last bucket
/// is entirely receivers this file cannot name: a chain deeper than one link
/// (`a.b().c().d()`, where the inner hop is itself unresolved and so has no key
/// to hand on) or an identifier the binding map never learned a type for
/// (`let x = ctx.pg();`). Both are the same defect one level down and they
/// unlock together: once a resolver can turn a hint into a concrete type,
/// feeding that type back into the binding map is what reaches them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiverHint {
    /// The receiver is the value another call returned — this is THAT call's
    /// target FQN (`rust·senseid·tasks::executor·TaskContext·pg`). The resolver
    /// reads the declared return type off that node ([`FqnDefinition::return_type`])
    /// to learn the receiver's type, then finds the member under it.
    ReturnOf(String),
}

impl ReceiverHint {
    /// The `edges.props` key this variant is stored under.
    ///
    /// A call is emitted in one place and completed in another — the producer
    /// has the receiver, the resolver has the graph — so the key name lives
    /// HERE, next to the variant, rather than as string literals that can drift
    /// apart across `process.rs` and `pg_store::graph`.
    const PROPS_KEY: &'static str = "receiver_return_of";

    /// Encode for `edges.props`. Only ever stamped on an UNRESOLVED call — a
    /// resolved edge already names its target, and recording the receiver
    /// alongside it would be a second answer to a question already answered.
    pub fn to_props(&self) -> serde_json::Value {
        let Self::ReturnOf(fqn) = self;
        serde_json::json!({ Self::PROPS_KEY: fqn })
    }

    /// Decode from `edges.props`. `None` when the edge carries no hint, which
    /// is every call emitted before this existed and every receiver the
    /// producer could not name — both stay unresolved, as they are.
    pub fn from_props(props: &serde_json::Value) -> Option<Self> {
        props.get(Self::PROPS_KEY).and_then(|v| v.as_str()).map(|f| Self::ReturnOf(f.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #201: ONE fqn minter. `indexer::fqn` is "the single encoder"; the v1
    /// joiners here (`item`, `lib`) built a second, four-segment scheme, and a
    /// lookup keyed on it could never match a v2 node (imports placed 2.3%
    /// against 48.5% for references). A second encoder that still compiles is
    /// one an author reaches for, so neither may be defined or called.
    ///
    /// The predicates (`is_external`, `sql_is_external`) READ an fqn and stay.
    #[test]
    fn no_second_fqn_minter_is_defined_or_called() {
        fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    rust_files(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&src, &mut files);
        // Spelled at runtime so this test's own text never matches itself.
        let minters = ["item", "lib", "method"];
        let calls: Vec<String> = minters.iter().map(|m| format!("languages::fqn::{m}(")).collect();
        let mut found = Vec::new();
        for f in &files {
            let text = std::fs::read_to_string(f).unwrap();
            for (i, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                if calls.iter().any(|c| code.contains(c.as_str())) {
                    found.push(format!("{}:{}", f.display(), i + 1));
                }
            }
        }
        assert!(found.is_empty(), "v1 minter calls: {found:#?}");
        let own = std::fs::read_to_string(src.join("languages/fqn.rs")).unwrap();
        for m in minters {
            let def = format!("pub fn {m}(");
            assert!(!own.contains(&def), "languages::fqn still defines `{def}`");
        }
    }

    /// The prefix and the builder must not be able to disagree.
    ///
    /// `LIB_PREFIX` is a literal because SQL needs one, and a literal that
    /// restates `SEP` is exactly the kind of copy that drifts when the
    /// separator moves. This is what makes moving it a compile-and-test
    /// failure rather than a silent mismatch in which every external node
    /// stops being recognised as external.
    #[test]
    fn the_external_marker_agrees_with_the_builder() {
        assert_eq!(LIB_PREFIX, format!("lib{SEP}"), "the prefix must be `lib` + SEP");
        // The builder is the one encoder, `indexer::fqn` (#201).
        use crate::indexer::fqn::{Form, refer};
        let lib = |package, member| refer(&Form::Lib { package, member }).unwrap().to_string();
        assert!(is_external(&lib("serde", "Deserialize")));
        assert!(is_external(&lib("node:fs", "")));
        assert!(sql_is_external("n.fqn").contains(LIB_PREFIX), "the SQL test uses the same prefix");
        assert_eq!(sql_is_external("n.fqn"), format!("n.fqn LIKE '{LIB_PREFIX}%'"));
        // NULL-safe: `NOT LIKE` on a NULL is NULL, not true, so the guard has to
        // be part of the expression rather than remembered at each call site.
        assert!(sql_is_not_external("n.fqn").contains("IS NULL OR"));
    }

    /// A first-party FQN whose package merely BEGINS with `lib` is not external
    /// — `rust·libc·…` is a dependency named libc, matched on the separator.
    #[test]
    fn a_package_named_lib_something_is_not_external() {
        assert!(!is_external("rust·libc·ffi·open"));
        assert!(!is_external("rust·library·mod·f"));
        assert!(!is_external("libfoo·bar"));
    }

    #[test]
    fn sep_is_middot() {
        assert_eq!(SEP, '\u{00B7}');
        // No common identifier char equals it.
        assert_ne!(SEP, '.');
        assert_ne!(SEP, ':');
    }

    /// The hint is written by the emit path and read by the resolver, in
    /// different modules, days apart in the pipeline. What it carries is a
    /// METHOD fqn to read a return type from, and the key name is the contract
    /// that keeps the two ends agreeing across that gap.
    #[test]
    fn a_receiver_hint_round_trips_through_edge_props() {
        let hint = ReceiverHint::ReturnOf("rust·senseid·tasks::executor·TaskContext·pg".into());
        assert_eq!(
            ReceiverHint::from_props(&hint.to_props()),
            Some(hint.clone()),
            "{hint:?} must survive the round trip through edges.props"
        );
        assert_eq!(
            ReceiverHint::from_props(&serde_json::json!({})),
            None,
            "an edge with no hint yields no hint — never a defaulted one"
        );
        assert_eq!(
            ReceiverHint::from_props(&serde_json::json!({ "relation": "trait_impl" })),
            None,
            "another writer's props key is not a receiver hint"
        );
    }
}
