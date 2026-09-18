//! Re-indexing a file replaces exactly what that file claims
//! (spec R10, A8, D8, D9; plan step 7b).
//!
//! [`super::persist::write`] only ADDS. A call deleted from a source file keeps
//! its edge and reads back as a live fact, which is a wrong edge no query can
//! tell from a real one (R4). This module is the other half: the per-file set
//! diff that removes what the file stopped saying, and nothing else.
//!
//! Three rules shape everything here and they are worth having in front of you.
//!
//! 1. **Removal is DEMOTION, never deletion** (R10.2, D8). A node reconcile no
//!    longer has a claim for becomes a stub. It is not deleted, because a node
//!    deletion cascades through `source_id`, `target_id` and `parent_id` with no
//!    count, and the input that would trigger it cannot be trusted. The one row
//!    reconcile deletes is an edge whose occurrence object became empty.
//! 2. **The unit of attribution is not `file_path`** (R10.1). For a declaration
//!    it is the CLAIM SET; for an edge it is the occurrence key inside the row.
//!    `nodes.file_path` is neither: 356 of this repository's 372 files have
//!    their own module node declared by a different file, and 3,290 file-scope
//!    edges hang off such a node, so `DELETE … WHERE file_path = $F` takes away
//!    rows nobody re-indexed.
//! 3. **A damaged parse is undetectable** (R10.3). Truncating 294 corpus files
//!    to half their bytes produced zero read errors and kept 57% of symbols;
//!    `tree.has_error()` is true for 6 of 372 INTACT files and false for 53 of
//!    366 truncated ones. So reconcile does not try to judge the parse. Rule 1
//!    is what makes being wrong survivable, and [`Brake`] covers the one diff
//!    shape that is almost always damage.
//
// This module has no caller on purpose — see the note in `mod.rs`.
#![allow(dead_code)]

use std::collections::BTreeSet;

use super::facts::{FileFacts, Language};
use super::persist::{self, Written};
use crate::db::pg_store::{Dropped, PgStore};

/// Where a file is, in the terms its own identity is minted from.
///
/// A file cannot state its package or its module path; the processor supplies
/// both from the manifest and the path, and [`super::lang::rust::read`] carries
/// them through onto the facts. Reconcile needs them for a file that is GONE,
/// where there are no facts to carry them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub language: Language,
    pub package: String,
    pub module: String,
    pub path: String,
}

/// What a file states NOW — the only input reconcile takes.
///
/// This enum is the read boundary R10.3 requires, and its job is to make one
/// mistake unrepresentable: a file whose READ failed can never arrive here as
/// an empty fact set. `read` returns `Result<FileFacts, ReadError>`, `FileFacts`
/// has no production constructor outside a language module's `read` — guarded
/// by a test over the guarded sources — and there is no variant here that a caller
/// could put an `Err` into. On `Err` the caller does not call reconcile at all,
/// the file keeps the graph it had, and the failure is reported.
///
/// [`Self::Gone`] is not "a parse that found nothing". It is a file that is not
/// on disk, which is an OBSERVED fact the caller already holds. That
/// distinction is what R10.3's brake turns on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stated {
    /// A parse succeeded and these are its facts.
    Parsed(FileFacts),
    /// The file is not there. `reconcile(F, claims = ∅)`, down the same path as
    /// a file emptied to a comment, so deletion cannot acquire a second removal
    /// semantics that behaves differently (R10.5).
    Gone(Located),
}

impl Stated {
    /// Where the file is, whichever way it is stated.
    pub fn located(&self) -> Located {
        match self {
            Self::Parsed(facts) => Located {
                language: facts.language,
                package: facts.package.clone(),
                module: facts.module.clone(),
                path: facts.path.clone(),
            },
            Self::Gone(located) => located.clone(),
        }
    }

    /// The identities this file declares. Empty for a file that is gone, which
    /// is the whole of `reconcile(F, ∅)`.
    fn claims(&self) -> BTreeSet<String> {
        match self {
            Self::Parsed(facts) => {
                facts.symbols.iter().map(|s| s.fqn.as_str().to_string()).collect()
            }
            Self::Gone(_) => BTreeSet::new(),
        }
    }
}

/// Whether the one diff shape that is almost always damage was applied (R10.3).
///
/// A file that previously claimed at least one declaration and now claims none
/// is either a real emptying (`//! moved to bar.rs`) or a truncated read, and
/// nothing IN the read tells them apart: `""`, `"fn main() {"` and garbage all
/// return `Ok` with zero facts. So reconcile reads the file once more and
/// applies the diff only if the second read agrees.
///
/// A threshold on the retained fraction was rejected on measurement: truncating
/// the corpus gave a per-file retention of 0% to 100% with a median of 52%, so
/// any cut-off refuses real edits at about the rate it catches damage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Brake {
    /// The diff was not that shape.
    NotNeeded,
    /// It was, and the second read said the same thing. The diff was applied.
    Confirmed,
    /// It was, and the second read did not confirm it. NOTHING was applied.
    Held(Held),
}

/// Why the brake held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Held {
    /// The second read found declarations after all, so the first read was
    /// damaged. Carries how many, because a caller that only learns "held"
    /// cannot tell a transient from a bug in the walk.
    SecondReadClaimed(usize),
    /// The second read failed. An unreadable file is not an empty one (R4).
    SecondReadFailed(String),
}

/// What writing the new facts did — or that there were none to write.
///
/// An enum and not a [`Written`] full of zeroes: a file that is gone was not
/// written with zero symbols, it was not written at all, and a caller summing
/// counts across files must not be handed a fabricated write to add in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wrote {
    Facts(Written),
    Nothing,
}

/// An identity this file released that ANOTHER file still declares (R10.4).
///
/// Two files claiming one identity is an A7 violation, not a state to support
/// gracefully — so it is REPORTED, the same way [`persist::Written`] returns
/// its collisions, and the repair belongs to the identity rule rather than
/// here. Reconcile's own duty is only not to demote a node somebody else is
/// still defining.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contested {
    pub fqn: String,
    /// Who is left, this file excluded.
    pub still_claimed_by: Vec<String>,
}

/// Everything one reconcile did.
///
/// Destructured exhaustively by its tests (R9), so a counter added here has to
/// be checked somewhere before the build goes green again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconciled {
    pub wrote: Wrote,
    /// Identities this file claims now.
    pub claimed: usize,
    /// Claims it gave up.
    pub released: usize,
    /// Of those, the ones no file claims any more, so the node became a stub.
    pub demoted: usize,
    /// Of those, the ones another file still claims. See [`Contested`].
    pub contested: Vec<Contested>,
    /// Edge rows this file stopped contributing to.
    pub occurrences_dropped: usize,
    /// Of those, the rows that had nothing left on them. The only deletion
    /// reconcile performs.
    pub edges_deleted: usize,
    pub braked: Brake,
}

/// Re-index one file: write what it says now, remove what it stopped saying,
/// and touch nothing that belongs to another file (R10.6).
///
/// `again` is the second read the brake needs, and it is the ONE exception R1
/// records to "no stage may re-read source bytes". It is a parameter rather
/// than something this module does for itself because reading a file is the
/// caller's business — this module has no path resolution, no IO and no
/// grammar, and giving it any would make it a second walk.
pub async fn reconcile(
    store: &PgStore,
    folder_id: &uuid::Uuid,
    stated: &Stated,
    again: &dyn Fn() -> Result<Stated, String>,
) -> Result<Reconciled, String> {
    let here = stated.located();
    let now = stated.claims();
    let before = store.claims_of_file(folder_id, &here.path).await?;
    // The one identity this file claims by EXISTING rather than by declaring.
    // The brake subtracts it so that a file which declares nothing is still
    // recognised as declaring nothing once it emits a module for itself.
    let its_own_module =
        persist::file_identity_of(here.language, &here.package, &here.module, &here.path)?;

    // R10.3's brake. Only an INFERRED emptiness is braked: a file that is
    // `Gone` was observed to be absent, and re-reading it would only re-observe
    // that.
    let braked = match brake(stated, &its_own_module, &before, &now, again) {
        Ok(brake) => brake,
        Err(held) => {
            return Ok(Reconciled {
                wrote: Wrote::Nothing,
                claimed: 0,
                released: 0,
                demoted: 0,
                contested: Vec::new(),
                occurrences_dropped: 0,
                edges_deleted: 0,
                braked: Brake::Held(held),
            });
        }
    };

    // The new facts go in FIRST. An identity this file both dropped and
    // re-declared under a moved span is then already re-claimed, and the
    // release below sees it as a claim it still holds rather than one to give
    // up and re-take.
    let wrote = match stated {
        Stated::Parsed(facts) => Wrote::Facts(persist::write(store, folder_id, facts).await?),
        Stated::Gone(_) => Wrote::Nothing,
    };

    let mut released = 0usize;
    let mut demoted = 0usize;
    let mut contested: Vec<Contested> = Vec::new();
    for fqn in before.difference(&now) {
        let Some(release) = store.release_claim(folder_id, fqn, &here.path).await? else {
            // The claim was read out of this folder a moment ago. A node that is
            // not there now is an inconsistency, not something to count as a
            // release that happened.
            return Err(format!(
                "reconcile({}): {fqn} claimed the file but no node holds that identity",
                here.path
            ));
        };
        released += 1;
        if release.still_claimed_by.is_empty() {
            // Back to the shape a not-yet-indexed reference has — the kind the
            // IDENTITY states, never one guessed from what used to be there.
            let (kind, _) = persist::stub_kind_and_name(fqn)?;
            store.demote_symbol(&release.node_id, kind).await?;
            demoted += 1;
        } else {
            contested
                .push(Contested { fqn: fqn.clone(), still_claimed_by: release.still_claimed_by });
        }
    }

    let (occurrences_dropped, edges_deleted) =
        drop_stale_occurrences(store, folder_id, &here, &before, &now, &wrote).await?;

    Ok(Reconciled {
        wrote,
        claimed: now.len(),
        released,
        demoted,
        contested,
        occurrences_dropped,
        edges_deleted,
        braked,
    })
}

/// Take this file's occurrences off every edge it used to contribute to and no
/// longer does (R10.1, R10.4).
///
/// The keep-set is the row ids the write just landed on, and it comes from the
/// write because only the write can know them: which row an edge becomes is
/// decided by the database's conflict key and by whatever the source identity's
/// upsert did.
async fn drop_stale_occurrences(
    store: &PgStore,
    folder_id: &uuid::Uuid,
    here: &Located,
    before: &BTreeSet<String>,
    now: &BTreeSet<String>,
    wrote: &Wrote,
) -> Result<(usize, usize), String> {
    let keep = match wrote {
        Wrote::Facts(written) => &written.edge_rows,
        Wrote::Nothing => &BTreeSet::new(),
    };
    // Everything this file's edges can be sourced at: what it declares, what it
    // USED to declare (a renamed symbol's old identity still sources old rows),
    // and its own module identity — which is in neither claim set, because the
    // `mod x;` that mints it sits in the parent file.
    let mut sources: Vec<String> = before.union(now).cloned().collect();
    sources.push(persist::file_identity_of(
        here.language,
        &here.package,
        &here.module,
        &here.path,
    )?);

    let mut dropped = 0usize;
    let mut deleted = 0usize;
    for edge in store.edges_contributed_by(folder_id, &here.path, &sources).await? {
        if keep.contains(&edge) {
            continue;
        }
        match store.drop_edge_occurrences(&edge, &here.path).await? {
            Dropped::Kept => dropped += 1,
            Dropped::RowDeleted => {
                dropped += 1;
                deleted += 1;
            }
            // The row was selected out of this folder a moment ago, so a miss
            // here is an inconsistency rather than a drop that did nothing.
            Dropped::NoSuchEdge => {
                return Err(format!(
                    "reconcile({}): edge {edge} carried this file's occurrences and then vanished",
                    here.path
                ));
            }
        }
    }
    Ok((dropped, deleted))
}

/// R10.3's brake, as a decision separated from the work it gates.
///
/// `Err(Held)` means "apply nothing" — it is not a failure of reconcile, it is
/// the brake doing its job, which is why the caller turns it into a
/// [`Reconciled`] rather than propagating it.
fn brake(
    stated: &Stated,
    its_own_module: &str,
    before: &BTreeSet<String>,
    now: &BTreeSet<String>,
    again: &dyn Fn() -> Result<Stated, String>,
) -> Result<Brake, Held> {
    // A file that is GONE was observed to be absent. Nothing was inferred from a
    // parse, so there is nothing to confirm.
    if matches!(stated, Stated::Gone(_)) || declared(now, its_own_module) > 0 || before.is_empty() {
        return Ok(Brake::NotNeeded);
    }
    match again() {
        // The file really does declare nothing now.
        Ok(second) if declared(&second.claims(), its_own_module) == 0 => Ok(Brake::Confirmed),
        Ok(second) => Err(Held::SecondReadClaimed(declared(&second.claims(), its_own_module))),
        Err(why) => Err(Held::SecondReadFailed(why)),
    }
}

/// How many identities a read claims BEYOND the file's own existence.
///
/// The brake asks "did this read declare nothing", and a file's own module is
/// not a declaration the file made — it is the file being there at all. Once
/// every parsed file emits one [`SymbolKind::Module`] for itself, `claims()` is
/// never empty for a parsed file, so a trigger written as `now.is_empty()`
/// would never fire again and R10.3's damaged-parse defence would be gone for
/// every language at once — silently, because nothing fails when a brake stops
/// braking.
///
/// Subtracting exactly one known identity rather than filtering by kind: the
/// claim sets are strings, the file identity is the one string this file is
/// entitled to claim for free, and a kind test would need the symbols the brake
/// deliberately does not take.
fn declared(claims: &BTreeSet<String>, its_own_module: &str) -> usize {
    claims.iter().filter(|claim| claim.as_str() != its_own_module).count()
}

#[cfg(test)]
mod tests {
    use crate::db::pg_store::graph_seed::SeedGraph;
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::db::pg_store::PgStore;
    use crate::db::pg_store::tests::create_test_folder;
    use crate::indexer::facts::RelationKind;
    use crate::indexer::persist::TargetRow;

    use crate::indexer::walked_rust as walk_of;

    fn stated(module: &str, path: &str, text: &str) -> Stated {
        Stated::Parsed(walk_of(module, path, text))
    }

    async fn a_folder(store: &PgStore, test: &str) -> uuid::Uuid {
        create_test_folder(store, &format!("reconcile_{test}_{}", uuid::Uuid::new_v4())).await
    }

    /// Reconcile the way the pipeline does — barrier first.
    ///
    /// Stage 3 is a BARRIER (R14): every `files` row for a folder exists before
    /// any parse task for that folder runs, so the writers LOOK THE FILE UP AND
    /// FAIL CLOSED (R13) rather than minting one. A fixture calling `reconcile`
    /// directly skipped that step — not a different contract, just a
    /// precondition production establishes and the test did not.
    ///
    /// Seeded only for a PARSED file. `Stated::Gone` means the file was observed
    /// to be absent, and creating a row saying it exists would be the fixture
    /// asserting the opposite of what it is testing.
    async fn reconciled(
        store: &PgStore,
        folder: &uuid::Uuid,
        stated: &Stated,
        again: &dyn Fn() -> Result<Stated, String>,
    ) -> Result<Reconciled, String> {
        if let Stated::Parsed(facts) = stated {
            store.seed_only_file(folder, &facts.path).await?;
        }
        reconcile(store, folder, stated, again).await
    }

    /// A second read that must never be asked for.
    ///
    /// Passed wherever the diff is not the shape R10.3's brake guards, so a
    /// change that starts re-reading every file — one extra parse per file
    /// across a whole scan — fails here instead of quietly costing that.
    fn never_again() -> Result<Stated, String> {
        panic!("the brake fired on a diff that is not the shape it guards")
    }

    /// Every node row of the folder, or only those `claimed_by` a given file,
    /// as the columns hold them.
    ///
    /// Whole rows as jsonb rather than a chosen projection: this is what the
    /// "nothing else moved" clause compares, and a projection is a list of the
    /// columns whoever wrote it thought of. `id`, `folder_id` and `embedding`
    /// come off — the first two are the same in both halves of every comparison
    /// and the third is a vector nobody here writes.
    async fn node_rows(
        store: &PgStore,
        folder: &uuid::Uuid,
        claimed_by: Option<&str>,
    ) -> Vec<serde_json::Value> {
        // `file_id` OUT, `file_path` IN. The id is a per-row uuid, so two runs of
        // the same fixture over different folders disagree on it by
        // construction — an R6 order-independence comparison over whole rows
        // would fail on an identity that says nothing about the graph. The path
        // is the fact the assertion is about.
        //
        // `parent_id` is the same shape and gets the same treatment: OUT, with
        // the parent's FQN in. Dropping it outright would stop the comparison
        // saying anything about containment at all, which is the half of
        // `nodes.parent_id` an R6 test most needs to cover now that a module
        // fills it for every file-scope declaration.
        let rows: Vec<(serde_json::Value,)> = sqlx_core::query_as::query_as(
            "SELECT (to_jsonb(n) - 'id' - 'folder_id' - 'embedding' - 'file_id' - 'parent_id')
                    || jsonb_build_object('file_path', to_jsonb(np.file_path))
                    || jsonb_build_object('parent_fqn', to_jsonb(p.fqn))
               FROM sensei.nodes n
               LEFT JOIN sensei.node_paths np ON np.node_id = n.id
               LEFT JOIN sensei.nodes p ON p.id = n.parent_id
              WHERE n.folder_id = $1 AND ($2::text IS NULL OR n.props -> 'claims' ? $2)
              ORDER BY n.fqn, n.name",
        )
        .bind(folder)
        .bind(claimed_by)
        .fetch_all(store.pool())
        .await
        .expect("the node rows read");
        rows.into_iter().map(|(row,)| row).collect()
    }

    /// Every edge row of the folder, or only those a given file contributes to,
    /// with both endpoints as the identities they carry.
    async fn edge_rows(
        store: &PgStore,
        folder: &uuid::Uuid,
        contributed_by: Option<&str>,
    ) -> Vec<serde_json::Value> {
        let rows: Vec<(serde_json::Value,)> = sqlx_core::query_as::query_as(
            "SELECT jsonb_build_object(
                        'source', s.fqn, 'kind', e.kind::text, 'target', t.fqn,
                        'target_name', e.target_name, 'props', e.props,
                        'modified_at', e.modified_at)
                   FROM sensei.edges e
                   JOIN sensei.nodes s ON s.id = e.source_id
              LEFT JOIN sensei.nodes t ON t.id = e.target_id
                  WHERE e.folder_id = $1
                    AND ($2::text IS NULL OR e.props -> 'occurrences' ? $2)
               ORDER BY s.fqn, e.kind, t.fqn, e.target_name",
        )
        .bind(folder)
        .bind(contributed_by)
        .fetch_all(store.pool())
        .await
        .expect("the edge rows read");
        rows.into_iter().map(|(row,)| row).collect()
    }

    /// One node row by identity, or nothing.
    async fn node_row(
        store: &PgStore,
        folder: &uuid::Uuid,
        fqn: &str,
    ) -> Option<serde_json::Value> {
        // `file_path` is projected back onto the row from `sensei.node_paths`
        // (R13 moved it off `nodes` and onto `files`). LEFT JOINed, so a stub —
        // which HAS no file by construction — still reads back as a JSON null
        // rather than vanishing from the row.
        let row: Option<(serde_json::Value,)> = sqlx_core::query_as::query_as(
            "SELECT (to_jsonb(n) - 'id' - 'folder_id' - 'embedding' - 'file_id')
                    || jsonb_build_object('file_path', to_jsonb(np.file_path))
               FROM sensei.nodes n
               LEFT JOIN sensei.node_paths np ON np.node_id = n.id
              WHERE n.folder_id = $1 AND n.fqn = $2",
        )
        .bind(folder)
        .bind(fqn)
        .fetch_optional(store.pool())
        .await
        .expect("the node row reads");
        row.map(|(row,)| row)
    }

    // ── the diff ─────────────────────────────────────────────────────────────

    /// R10, in its smallest form. Today's write path only ADDS, so the call's
    /// edge survives the call's deletion and reads back as a live fact — a wrong
    /// edge that no query can tell from a real one (R4).
    #[tokio::test]
    async fn a_call_a_file_stopped_making_loses_its_edge() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "call_removed").await;
        let toll = "rust·senseid·bell·toll·item";

        reconciled(
            &store,
            &folder,
            &stated("bell", "src/bell.rs", "pub fn toll() {}\npub fn ring() { toll(); }"),
            &never_again,
        )
        .await
        .expect("the first index");

        let done = reconciled(
            &store,
            &folder,
            &stated("bell", "src/bell.rs", "pub fn toll() {}\npub fn ring() {}"),
            &never_again,
        )
        .await
        .expect("the re-index");

        let stored =
            crate::indexer::persist::read_back(&store, &folder).await.expect("the rows read back");
        let calls: Vec<&crate::indexer::persist::ReferenceRow> = stored
            .references
            .iter()
            .filter(|r| matches!(&r.target, TargetRow::Resolved { fqn, .. } if fqn == toll))
            .collect();
        assert!(
            calls.is_empty(),
            "the call was deleted from the source, so the graph must not still answer with it: \
             {calls:?}"
        );
        assert_eq!(
            (done.occurrences_dropped, done.edges_deleted),
            (1, 1),
            "one edge lost this file's occurrences and nothing else contributed to it, so the \
             row goes"
        );
        assert!(
            node_row(&store, &folder, toll).await.is_some(),
            "the CALLEE is still declared; only the call went"
        );
    }

    /// A declaration the file stopped making stops being a definition — and
    /// keeps its node, as a stub (R10.2, D8).
    ///
    /// Both halves are asserted. Deleting the row would also make the first
    /// half pass, and it is what destroys every inbound edge from every file
    /// that is not being re-indexed: measured, 1,485 of 12,519 declarations have
    /// one, and the worst has them from 67 files.
    #[tokio::test]
    async fn a_declaration_a_file_stopped_making_becomes_a_stub_and_not_a_hole() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "decl_removed").await;
        let gone = "rust·senseid·bell·gone·item";

        reconciled(
            &store,
            &folder,
            &stated("bell", "src/bell.rs", "pub fn kept() {}\npub fn gone() {}"),
            &never_again,
        )
        .await
        .expect("the first index");

        let done = reconciled(
            &store,
            &folder,
            &stated("bell", "src/bell.rs", "pub fn kept() {}"),
            &never_again,
        )
        .await
        .expect("the re-index");

        assert_eq!((done.released, done.demoted), (1, 1), "one claim released, one node demoted");
        assert_eq!(done.contested, vec![], "no other file declared it");

        let stored =
            crate::indexer::persist::read_back(&store, &folder).await.expect("the rows read back");
        assert_eq!(
            stored.symbols.iter().map(|s| s.fqn.as_str()).collect::<Vec<&str>>(),
            vec!["rust·senseid·bell·kept·item", "rust·senseid·bell·mod"],
            "only what the file still declares reads back as a definition — which now \
             includes the file's OWN module, because a file is a module and says so"
        );

        let row = node_row(&store, &folder, gone).await.expect("the node must SURVIVE as a stub");
        assert_eq!(row["resolved"], serde_json::json!(false), "a stub is not resolved");
        assert_eq!(row["file_path"], serde_json::Value::Null, "and names no file");
        for column in ["line_start", "line_end", "signature", "docstring"] {
            assert_eq!(
                row[column],
                serde_json::Value::Null,
                "a row that says `resolved = false` while still carrying {column} answers \
                 \"where is this defined\" with a place that no longer defines it (R4)"
            );
        }
        assert_eq!(row["is_exported"], serde_json::json!(false));
        for prop in ["symbol_kind", "visibility", "declared_type", "params", "span_columns"] {
            assert_eq!(
                row["props"].get(prop),
                None,
                "props.{prop} is a definition-only prop and must go with the definition"
            );
        }
        assert!(
            row["props"].get("claims").is_some(),
            "the claim set stays — now empty — because it is what the node is keyed by"
        );
    }

    /// Removing a declaration removes the edges it SOURCED.
    ///
    /// Not implied by the test above: the node survives demotion, and an edge
    /// hanging off it would survive with it. The occurrence key is what has to
    /// go, and it is keyed by file rather than by source.
    #[tokio::test]
    async fn removing_a_declaration_removes_the_edges_it_sourced() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "source_removed").await;

        reconciled(
            &store,
            &folder,
            &stated("bell", "src/bell.rs", "pub fn kept() {}\npub fn gone() { kept(); }"),
            &never_again,
        )
        .await
        .expect("the first index");
        assert_eq!(
            edge_rows(&store, &folder, Some("src/bell.rs")).await.len(),
            1,
            "the fixture must actually produce the edge, or the assertion below is vacuous"
        );

        reconciled(
            &store,
            &folder,
            &stated("bell", "src/bell.rs", "pub fn kept() {}"),
            &never_again,
        )
        .await
        .expect("the re-index");

        assert_eq!(
            edge_rows(&store, &folder, None).await,
            Vec::<serde_json::Value>::new(),
            "the calling declaration is gone, so the call it made is gone"
        );
    }

    /// R10.4. Two files produce ONE edge row; re-indexing one must take only
    /// that one's occurrences.
    ///
    /// A fixture and not the corpus, because the corpus cannot carry this test:
    /// measured, 4 of its 82,913 edges have more than one contributing file. The
    /// two files here share a module path, so they share one file identity, and
    /// the imports of both hang off it — the real shape, which `crates/mcp`'s
    /// `lib.rs` and `main.rs` produce.
    #[tokio::test]
    async fn a_shared_edge_loses_only_the_re_indexed_files_occurrences() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "shared_edge").await;
        let import = "use std::collections::BTreeMap;\n";

        for path in ["src/first.rs", "src/second.rs"] {
            reconciled(
                &store,
                &folder,
                &stated(
                    "shared",
                    path,
                    &format!("{import}pub fn a() -> BTreeMap<u32, u32> {{ BTreeMap::new() }}"),
                ),
                &never_again,
            )
            .await
            .unwrap_or_else(|e| panic!("{path} indexes: {e}"));
        }
        let both = edge_rows(&store, &folder, None).await;
        let shared: Vec<&serde_json::Value> = both
            .iter()
            .filter(|row| row["props"]["occurrences"].as_object().is_some_and(|o| o.len() == 2))
            .collect();
        assert!(
            !shared.is_empty(),
            "the fixture must actually put two files on one edge row, or this asserts nothing: \
             {both:#?}"
        );

        reconciled(
            &store,
            &folder,
            &stated("shared", "src/first.rs", "pub fn a() -> u32 { 1 }"),
            &never_again,
        )
        .await
        .expect("the re-index");

        for row in edge_rows(&store, &folder, None).await {
            let occurrences = row["props"]["occurrences"].as_object().expect("an object");
            assert!(
                !occurrences.contains_key("src/first.rs"),
                "the re-indexed file stopped making this, so its key must be gone: {row}"
            );
        }
        assert_eq!(
            edge_rows(&store, &folder, Some("src/second.rs")).await.len(),
            shared.len(),
            "every edge the OTHER file contributes to must still be there — a source-scoped \
             DELETE would have taken them with it"
        );
    }

    // ── the parse that cannot be judged (R10.3) ──────────────────────────────

    /// A read that produced no declarations where there were some prunes
    /// NOTHING until a second read agrees.
    ///
    /// This is the only defence there is. Measured: truncating a file to half
    /// its bytes produces zero read errors, and `""`, `"fn main() {"` and
    /// garbage all return `Ok` with zero facts — identical to a genuinely empty
    /// file. There is no in-band discriminator, so the discriminator is a second
    /// read.
    #[tokio::test]
    async fn a_read_that_suddenly_claims_nothing_prunes_nothing_until_a_second_read_agrees() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "truncated").await;
        let whole = "pub fn toll() {}\npub fn ring() { toll(); }";

        reconciled(&store, &folder, &stated("bell", "src/bell.rs", whole), &never_again)
            .await
            .expect("the first index");
        let nodes = node_rows(&store, &folder, None).await;
        let edges = edge_rows(&store, &folder, None).await;

        // What tree-sitter returns for a file cut in half mid-item: a tree, no
        // error, and no declarations.
        let truncated = stated("bell", "src/bell.rs", "pub fn toll(");
        let intact = || Ok(stated("bell", "src/bell.rs", whole));
        let done = reconciled(&store, &folder, &truncated, &intact)
            .await
            .expect("reconcile runs; it just must not apply anything");

        assert_eq!(
            done.braked,
            Brake::Held(Held::SecondReadClaimed(2)),
            "the second read found the declarations, so the first read was damaged"
        );
        assert_eq!(
            (done.released, done.demoted, done.occurrences_dropped, done.edges_deleted),
            (0, 0, 0, 0),
            "a file that failed to parse must prune NOTHING"
        );
        assert_eq!(done.wrote, Wrote::Nothing, "and must not write the half it did read either");
        assert_eq!(node_rows(&store, &folder, None).await, nodes, "no node moved");
        assert_eq!(edge_rows(&store, &folder, None).await, edges, "no edge moved");
    }

    /// R10.3's brake must survive a file declaring its own MODULE.
    ///
    /// The brake's trigger is "this read claims nothing". Once every parsed
    /// file emits one [`SymbolKind::Module`] for itself, no parsed file ever
    /// claims nothing again — `claims()` is exactly the symbol fqn set — so the
    /// trigger would never fire and the damaged-parse defence would be GONE,
    /// silently and for every language at once. A truncated read that today
    /// prunes nothing until a second read agrees would instead apply its empty
    /// diff and demote every real declaration in the file.
    ///
    /// So the trigger is re-anchored on what a file claims BEYOND its own
    /// existence. Landed BEFORE any file-module is emitted, so the property
    /// never has a window in which it is dead.
    ///
    /// Tested against `brake` directly rather than through `reconciled`,
    /// because the claim sets are the brake's actual inputs and a DB round trip
    /// would only re-derive them.
    #[test]
    fn a_file_claiming_nothing_but_its_own_module_still_trips_the_brake() {
        let its_own_module =
            persist::file_identity_of(Language::Rust, "senseid", "bell", "src/bell.rs")
                .expect("a rust file has a module identity");

        let before = BTreeSet::from(["rust·senseid·bell·toll·item".to_string()]);
        let now = BTreeSet::from([its_own_module.clone()]);
        let damaged = stated("bell", "src/bell.rs", "//! the read was truncated");
        let intact = || Ok(stated("bell", "src/bell.rs", "pub fn toll() {}"));

        assert_eq!(
            brake(&damaged, &its_own_module, &before, &now, &intact),
            Err(Held::SecondReadClaimed(1)),
            "a file whose ONLY claim is its own module has declared nothing, and the second \
             read found a declaration — so the first read was damaged"
        );
    }

    /// The other half: a file that really did lose its declarations still
    /// reconciles, and is not held forever by its own module identity.
    #[test]
    fn a_file_emptied_down_to_its_own_module_is_confirmed_not_held() {
        let its_own_module =
            persist::file_identity_of(Language::Rust, "senseid", "bell", "src/bell.rs")
                .expect("a rust file has a module identity");

        let before = BTreeSet::from(["rust·senseid·bell·toll·item".to_string()]);
        let now = BTreeSet::from([its_own_module.clone()]);
        let emptied = stated("bell", "src/bell.rs", "//! nothing");
        let agrees = || Ok(stated("bell", "src/bell.rs", "//! nothing"));

        assert_eq!(
            brake(&emptied, &its_own_module, &before, &now, &agrees),
            Ok(Brake::Confirmed),
            "both reads agree it declares nothing of its own, so the diff applies"
        );
    }

    /// A second read that FAILS is not a confirmation. An unreadable file is not
    /// an empty one (R4).
    #[tokio::test]
    async fn a_second_read_that_fails_holds_the_brake() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "unreadable").await;

        reconciled(
            &store,
            &folder,
            &stated("bell", "src/bell.rs", "pub fn toll() {}"),
            &never_again,
        )
        .await
        .expect("the first index");
        let nodes = node_rows(&store, &folder, None).await;

        let done =
            reconciled(&store, &folder, &stated("bell", "src/bell.rs", "//! nothing"), &|| {
                Err("permission denied".to_string())
            })
            .await
            .expect("reconcile runs");

        assert_eq!(
            done.braked,
            Brake::Held(Held::SecondReadFailed("permission denied".to_string()))
        );
        assert_eq!((done.released, done.demoted), (0, 0));
        assert_eq!(node_rows(&store, &folder, None).await, nodes, "nothing moved");
    }

    /// A file genuinely emptied reads zero twice and reconciles on the second.
    /// The brake is a confirmation, not a refusal.
    #[tokio::test]
    async fn a_file_genuinely_emptied_reconciles_once_the_second_read_agrees() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "emptied").await;

        reconciled(
            &store,
            &folder,
            &stated("bell", "src/bell.rs", "pub fn toll() {}"),
            &never_again,
        )
        .await
        .expect("the first index");

        let empty = "//! moved to bar.rs";
        let done = reconciled(&store, &folder, &stated("bell", "src/bell.rs", empty), &|| {
            Ok(stated("bell", "src/bell.rs", empty))
        })
        .await
        .expect("the re-index");

        assert_eq!(done.braked, Brake::Confirmed);
        assert_eq!((done.released, done.demoted), (1, 1));
        assert_eq!(
            crate::indexer::persist::read_back(&store, &folder)
                .await
                .expect("the rows read back")
                .symbols
                .iter()
                .map(|s| s.fqn.as_str())
                .collect::<Vec<&str>>(),
            vec!["rust·senseid·bell·mod"],
            "the file states nothing OF ITS OWN, so the graph records nothing but the file \
             itself — which is still there and is still a module"
        );
    }

    /// A DELETED file goes down the same code path as an emptied one, and skips
    /// the brake — absence on disk is OBSERVED, not concluded from a parse
    /// (R10.5).
    #[tokio::test]
    async fn a_deleted_file_takes_the_same_path_as_an_emptied_one() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "deleted").await;

        reconciled(
            &store,
            &folder,
            &stated("bell", "src/bell.rs", "pub fn toll() {}\npub fn ring() { toll(); }"),
            &never_again,
        )
        .await
        .expect("the first index");

        let done = reconciled(
            &store,
            &folder,
            &Stated::Gone(Located {
                language: Language::Rust,
                package: "senseid".to_string(),
                module: "bell".to_string(),
                path: "src/bell.rs".to_string(),
            }),
            &never_again,
        )
        .await
        .expect("the deletion reconciles");

        assert_eq!(done.braked, Brake::NotNeeded, "an observed absence needs no confirming read");
        assert_eq!(done.wrote, Wrote::Nothing);
        assert_eq!(
            (done.released, done.demoted),
            (3, 3),
            "the two declarations AND the file's own module, which a deleted file also \
             stops claiming"
        );
        assert_eq!(
            edge_rows(&store, &folder, None).await,
            Vec::<serde_json::Value>::new(),
            "every edge the file contributed to had only its occurrences on it"
        );
    }

    // ── other files (R10.6 clause b) ─────────────────────────────────────────

    /// Two files declare ONE identity; releasing one claim must not demote it
    /// (R10.4, D9).
    ///
    /// Without a claim set this is unanswerable: reconcile cannot tell "F was
    /// the only declarer" from "F was one of two", so it demotes a node another
    /// file still defines. Measured, 2 identities in this repository are claimed
    /// by two files today.
    #[tokio::test]
    async fn releasing_one_of_two_claims_does_not_demote_the_node() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "two_claims").await;
        let shared = "rust·senseid·shared·twice·item";

        for path in ["src/first.rs", "src/second.rs"] {
            reconciled(&store, &folder, &stated("shared", path, "pub fn twice() {}"), &never_again)
                .await
                .unwrap_or_else(|e| panic!("{path} indexes: {e}"));
        }

        let done = reconciled(
            &store,
            &folder,
            &stated("shared", "src/first.rs", "pub fn other() {}"),
            &never_again,
        )
        .await
        .expect("the re-index");

        assert_eq!(done.released, 1, "the first file gave up its claim");
        assert_eq!(done.demoted, 0, "but the second file still declares it");
        assert_eq!(
            done.contested,
            vec![Contested {
                fqn: shared.to_string(),
                still_claimed_by: vec!["src/second.rs".to_string()],
            }],
            "two files claiming one identity is an A7 violation and is REPORTED, not swallowed"
        );
        let row = node_row(&store, &folder, shared).await.expect("the node is there");
        assert_eq!(row["resolved"], serde_json::json!(true), "a node someone still declares");
    }

    /// The case `file_path` attribution destroys: a child module's node is
    /// declared by its PARENT file, and the child's own file-scope edges hang
    /// off it.
    ///
    /// Measured over this repository: 356 of 372 files have their own module
    /// node declared by another file, and 3,290 file-scope edges are sourced at
    /// such a node. `DELETE FROM sensei.nodes WHERE file_path = $F` — what
    /// `delete_nodes_by_file` does — therefore cascades away every child's edges
    /// when a `lib.rs` is re-indexed, in silence, from files nobody touched.
    #[tokio::test]
    async fn re_indexing_a_parent_module_leaves_its_childrens_edges_alone() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "parent_module").await;

        reconciled(
            &store,
            &folder,
            &stated("", "src/lib.rs", "pub mod child;\npub fn top() {}"),
            &never_again,
        )
        .await
        .expect("the parent indexes");
        reconciled(
            &store,
            &folder,
            &stated(
                "child",
                "src/child.rs",
                "use std::collections::BTreeMap;\npub fn deep() -> u32 { 1 }",
            ),
            &never_again,
        )
        .await
        .expect("the child indexes");

        let child_module = "rust·senseid·child·mod";
        let declared_by = node_row(&store, &folder, child_module).await.expect("the module node");
        assert_eq!(
            declared_by["file_path"],
            serde_json::json!("src/child.rs"),
            "the child declares its OWN module now — `pub mod child;` in the parent names a \
             module whose body is elsewhere, and a name is not a declaration"
        );
        let child_edges = edge_rows(&store, &folder, Some("src/child.rs")).await;
        assert!(
            child_edges.iter().any(|row| row["source"] == serde_json::json!(child_module)),
            "the fixture must reproduce the shape: a file-scope edge of the child sourced at a \
             node the parent declared: {child_edges:#?}"
        );

        reconciled(&store, &folder, &stated("", "src/lib.rs", "pub fn top() {}"), &never_again)
            .await
            .expect("the parent is re-indexed without the `mod` declaration");

        assert_eq!(
            edge_rows(&store, &folder, Some("src/child.rs")).await,
            child_edges,
            "the child was not re-indexed, so not one of its rows may have moved"
        );
    }

    /// R10.6 clause (b), the load-bearing one, over two files that share
    /// nothing: reconciling one leaves the other's contribution byte-identical.
    #[tokio::test]
    async fn reconciling_one_file_leaves_another_files_contribution_byte_identical() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "clause_b").await;

        reconciled(
            &store,
            &folder,
            &stated("one", "src/one.rs", "pub struct A { pub v: u32 }\npub fn a() -> u32 { 1 }"),
            &never_again,
        )
        .await
        .expect("the first indexes");
        reconciled(
            &store,
            &folder,
            &stated("two", "src/two.rs", "pub struct B { pub w: u32 }\npub fn b() -> u32 { 2 }"),
            &never_again,
        )
        .await
        .expect("the second indexes");

        let other_nodes = node_rows(&store, &folder, Some("src/two.rs")).await;
        let other_edges = edge_rows(&store, &folder, Some("src/two.rs")).await;
        assert!(
            !other_nodes.is_empty() && !other_edges.is_empty(),
            "the other file must have some"
        );

        reconciled(
            &store,
            &folder,
            &stated("one", "src/one.rs", "pub fn a() -> u32 { 1 }"),
            &never_again,
        )
        .await
        .expect("the first is re-indexed with its struct removed");

        assert_eq!(node_rows(&store, &folder, Some("src/two.rs")).await, other_nodes);
        assert_eq!(edge_rows(&store, &folder, Some("src/two.rs")).await, other_edges);
    }

    // ── idempotence ──────────────────────────────────────────────────────────

    /// Re-indexing an unchanged file changes nothing (R10.6).
    ///
    /// Idempotence is what lets the invariant be checked by RUNNING reconcile
    /// rather than by reading it, and it is the property a partial
    /// implementation loses first.
    ///
    /// Node rows are compared WHOLE, `modified_at` included. Edge rows are
    /// compared without it: `insert_edge_with_props` stamps `modified_at =
    /// now()` on every conflicting insert, it is a pre-existing `pg_store`
    /// function the legacy indexer calls, and this one may not change its behaviour.
    /// The row's CONTENT — endpoints, kind, props, occurrences — is compared in
    /// full.
    #[tokio::test]
    async fn re_indexing_an_unchanged_file_changes_nothing() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "idempotent").await;
        let text = "use std::collections::BTreeMap;\n\
                    pub struct Gadget { pub width: u32 }\n\
                    impl Gadget { pub fn new() -> Gadget { Gadget { width: 1 } } }\n\
                    pub fn widest(a: u32) -> u32 { a }";

        reconciled(&store, &folder, &stated("gadget", "src/gadget.rs", text), &never_again)
            .await
            .expect("the first index");
        let nodes = node_rows(&store, &folder, None).await;
        let edges: Vec<serde_json::Value> = edge_rows(&store, &folder, None)
            .await
            .into_iter()
            .map(|mut row| {
                row.as_object_mut().expect("an object").remove("modified_at");
                row
            })
            .collect();

        let done =
            reconciled(&store, &folder, &stated("gadget", "src/gadget.rs", text), &never_again)
                .await
                .expect("the re-index");

        // Destructured EXHAUSTIVELY (R9). A counter added to `Reconciled` stops
        // this compiling until someone says what an unchanged re-index does to
        // it — which is the question a new counter is easiest to get wrong.
        let Reconciled {
            wrote,
            claimed,
            released,
            demoted,
            contested,
            occurrences_dropped,
            edges_deleted,
            braked,
        } = done;
        assert_eq!(
            (released, demoted, occurrences_dropped, edges_deleted),
            (0, 0, 0, 0),
            "nothing changed in the file, so nothing may be released, demoted or dropped"
        );
        assert_eq!(
            claimed, 5,
            "the file still declares its struct, its field, its method, its function — \
             and itself, which is a module"
        );
        assert_eq!(contested, vec![]);
        assert_eq!(braked, Brake::NotNeeded);
        match wrote {
            Wrote::Facts(written) => {
                assert_eq!(written.collisions, vec![], "no identity was minted twice");
                assert_eq!(written.edge_collisions, vec![], "no two groups landed on one row");
            }
            Wrote::Nothing => panic!("the file states facts, so they were written"),
        }
        assert_eq!(
            node_rows(&store, &folder, None).await,
            nodes,
            "not one node row moved — modified_at included, so a re-scan of an untouched file \
             does not churn the graph"
        );
        assert_eq!(
            edge_rows(&store, &folder, None)
                .await
                .into_iter()
                .map(|mut row| {
                    row.as_object_mut().expect("an object").remove("modified_at");
                    row
                })
                .collect::<Vec<serde_json::Value>>(),
            edges,
            "and no edge row's content moved"
        );
    }

    /// R10.6 clause (a): what the graph records this file as stating is exactly
    /// what the facts state.
    #[tokio::test]
    async fn the_claims_of_a_file_are_exactly_the_identities_its_facts_declare() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "clause_a").await;
        let text = "pub struct Gadget { pub width: u32 }\n\
                    impl Gadget { pub fn new() -> Gadget { Gadget { width: 1 } } }";

        let facts = walk_of("gadget", "src/gadget.rs", text);
        reconciled(&store, &folder, &Stated::Parsed(facts.clone()), &never_again)
            .await
            .expect("the index");

        assert_eq!(
            store.claims_of_file(&folder, "src/gadget.rs").await.expect("the claims read"),
            facts.symbols.iter().map(|s| s.fqn.as_str().to_string()).collect::<BTreeSet<String>>(),
            "the set of nodes claiming this file must equal the set of symbols it declares"
        );

        // And an ownership relation is still a parent_id after reconcile, so the
        // diff has not undone what the write set up.
        let containment: BTreeMap<String, Option<String>> =
            store.containment(&folder).await.expect("the containment reads").into_iter().collect();
        for relation in facts.relations.iter().filter(|r| r.kind == RelationKind::Owns) {
            if let crate::indexer::facts::Resolution::Resolved { fqn: parent, .. } =
                &relation.parent
            {
                assert_eq!(
                    containment.get(relation.child.as_str()).map(Option::as_deref),
                    Some(Some(parent.as_str())),
                    "{} is a member of {parent}",
                    relation.child.as_str()
                );
            }
        }
    }

    /// A move between two files sharing a module path is order-independent
    /// (R10.5, A6): the fqn does not change, only which file claims it.
    #[tokio::test]
    async fn a_symbol_moving_between_two_files_of_one_module_lands_the_same_either_way() {
        async fn run(order: [&str; 2]) -> Vec<serde_json::Value> {
            let store = PgStore::connect_test().await.expect("the test database must be reachable");
            let folder =
                create_test_folder(&store, &format!("move_{}", uuid::Uuid::new_v4())).await;
            // `moved` starts in `first.rs`.
            reconciled(
                &store,
                &folder,
                &stated("shared", "src/first.rs", "pub fn moved() {}"),
                &never_again,
            )
            .await
            .expect("the first index");

            for file in order {
                let text = if file == "src/first.rs" { "//! it left" } else { "pub fn moved() {}" };
                reconciled(&store, &folder, &stated("shared", file, text), &|| {
                    Ok(stated("shared", "src/first.rs", "//! it left"))
                })
                .await
                .unwrap_or_else(|e| panic!("{file}: {e}"));
            }
            node_rows(&store, &folder, None)
                .await
                .into_iter()
                .map(|mut row| {
                    let row = row.as_object_mut().expect("an object");
                    row.remove("modified_at");
                    row.remove("created_at");
                    serde_json::Value::Object(row.clone())
                })
                .collect()
        }

        let old_first = run(["src/first.rs", "src/second.rs"]).await;
        let new_first = run(["src/second.rs", "src/first.rs"]).await;

        // THE MOVED SYMBOL is the subject, and it is order-independent whole.
        let moved = |rows: &[serde_json::Value]| -> Vec<serde_json::Value> {
            rows.iter().filter(|r| r["fqn"] != "rust·senseid·shared·mod").cloned().collect()
        };
        assert_eq!(
            moved(&old_first),
            moved(&new_first),
            "a claim is a SET member, not a winner, so which file the scan reaches first cannot \
             change the graph (R6)"
        );

        // THE MODULE NODE is claimed by BOTH files here, and its scalar columns
        // (file_path, span) are therefore last-writer-wins. That is not a defect
        // this test can report, because the input cannot occur: this fixture
        // hands two files one module path by hand, and neither language's rule
        // produces that. Rust maps 382 files to 382 identities; the web rule
        // maps 925 to 925 since it stopped dropping non-omittable extensions.
        //
        // What R6 DOES guarantee about it is asserted instead — the claim SET,
        // which is a set and merges either way round.
        let claims = |rows: &[serde_json::Value]| -> serde_json::Value {
            rows.iter()
                .find(|r| r["fqn"] == "rust·senseid·shared·mod")
                .expect("the module node")["props"]["claims"]
                .clone()
        };
        assert_eq!(
            claims(&old_first),
            claims(&new_first),
            "both files claim the module, and a claim set does not depend on arrival order"
        );
    }

    // ── the lookup that finds a file's edges (R10.1) ─────────────────────────

    /// The affordable predicate must be a SUPERSET of the occurrence key, never
    /// a convenient narrowing of it.
    ///
    /// Over real files, because the shape it can miss — an edge sourced at a
    /// node no file declares — is the split-`impl` anchor, and a fixture has to
    /// be built knowing about it while the corpus simply contains 26 of them.
    #[tokio::test]
    async fn the_edge_lookup_finds_every_edge_the_occurrence_key_names() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "superset").await;

        // The files that produce the split-`impl` anchoring, plus the module
        // that declares the type they anchor on.
        let sources: Vec<(String, String)> = crate::indexer::corpus_rust_sources()
            .into_iter()
            .filter(|(path, _)| path.contains("db/pg_store/"))
            .take(6)
            .collect();
        assert!(sources.len() >= 4, "the corpus slice must be real: {}", sources.len());

        let mut indexed: Vec<(String, String, String)> = Vec::new();
        for (path, text) in &sources {
            // The package is found on DISK, from the absolute path; everything
            // else uses the folder-relative one, which is the grain a real walk
            // produces and `sensei.files` is keyed by (R13).
            let package = crate::indexer::package_of(path);
            let path = &crate::indexer::workspace_relative(path);
            let module = crate::indexer::module_of(path);
            let facts = walk_of(&module, path, text);
            reconciled(&store, &folder, &Stated::Parsed(facts), &never_again)
                .await
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            indexed.push((package, module, path.clone()));
        }

        for (package, module, path) in &indexed {
            let claims: Vec<String> = store
                .claims_of_file(&folder, path)
                .await
                .expect("the claims read")
                .into_iter()
                .collect();
            let mut sources = claims.clone();
            sources.push(
                crate::indexer::persist::file_identity_of(Language::Rust, package, module, path)
                    .expect("the file identity"),
            );
            let found: BTreeSet<uuid::Uuid> = store
                .edges_contributed_by(&folder, path, &sources)
                .await
                .expect("the lookup runs")
                .into_iter()
                .collect();
            let truth: BTreeSet<uuid::Uuid> = store
                .occurrence_edges_naming_file(&folder, path)
                .await
                .expect("the unnarrowed read runs")
                .into_iter()
                .collect();
            let missed: Vec<&uuid::Uuid> = truth.difference(&found).collect();
            assert!(
                missed.is_empty(),
                "{path}: {} of {} edges naming this file are not reachable through R10.1's \
                 predicate, so reconcile would leave their occurrences behind",
                missed.len(),
                truth.len()
            );
            assert!(!truth.is_empty(), "{path} must contribute to some edge at all");
        }
    }
    // ── the read boundary (R10.3) ────────────────────────────────────────────

    /// A failed READ cannot reach reconcile as an empty fact set, and this is a
    /// property of the CODE rather than of any value it computes.
    ///
    /// The type argument is only as good as the absence of a back door.
    /// `Stated` has no variant an `Err` fits into, so the way to get one in
    /// would be to BUILD a `FileFacts` with empty vectors — which reads exactly
    /// like a file that genuinely declares nothing, and would prune a whole
    /// file's graph on an IO error. So nothing outside a language module's
    /// `read` may construct one, and no reader may swallow its `Result`.
    #[test]
    fn nothing_but_a_language_read_can_produce_file_facts() {
        let mut constructors = 0usize;
        let mut readers = 0usize;
        for (path, body) in crate::indexer::guard_sources() {
            let production = crate::indexer::outside_tests(&body).to_string();
            for line in production.lines() {
                let trimmed = line.trim();
                // A signature is not a construction, and neither is the type's
                // own declaration. A FUNCTIONAL UPDATE (`..facts`) is one, but a
                // harmless one: it needs an existing value to update, so it
                // cannot conjure a fact set where a read failed. That is the
                // shape the ladder uses to put placed references back on the
                // facts the walk produced.
                if trimmed.contains("FileFacts {")
                    && !trimmed.contains("-> FileFacts {")
                    && !trimmed.starts_with("pub struct FileFacts")
                    && !trimmed.contains("..")
                {
                    assert!(
                        path.starts_with("lang/"),
                        "{path}: `{trimmed}` builds a FileFacts from nothing, outside a language \
                         module's read. An empty one is byte-identical to a file that declares \
                         nothing, so reconcile would prune the file's whole graph on an IO \
                         error (R4)."
                    );
                    constructors += 1;
                }
                for forbidden in ["FileFacts::default", "Default for FileFacts"] {
                    assert!(
                        !trimmed.contains(forbidden),
                        "{path}: `{trimmed}` — a defaulted FileFacts is the empty fact set with \
                         nothing said about where it came from"
                    );
                }
                if !trimmed.contains("read(") {
                    continue;
                }
                readers += 1;
                for swallowed in ["unwrap_or_default", "unwrap_or(", ".ok()", "unwrap_or_else"] {
                    assert!(
                        !trimmed.contains(swallowed),
                        "{path}: `{trimmed}` turns a failed read into a value. An `Err` from \
                         `read` means the file keeps the graph it had; a fact set does not (R4)."
                    );
                }
            }
        }
        assert!(
            constructors > 0,
            "the guard found no FileFacts constructor at all, so it is \
             not guarding what it claims to"
        );
        assert!(readers > 0, "the guard found no call to a read, so it is not guarding one");
    }
}
