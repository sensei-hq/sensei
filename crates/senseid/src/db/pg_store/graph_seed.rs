//! Test-only graph seeding: the fixture's stand-in for the walk.
//!
//! # Why this exists
//!
//! R13 made `nodes.file_id` a foreign key into `sensei.files`, and the writers
//! LOOK THE ROW UP AND FAIL CLOSED — a node naming a file the walk never
//! recorded is refused, never get-or-created. That is the whole point: an
//! untracked file is a bug in the walk, and minting a `files` row on the write
//! path would hide it behind a plausible id.
//!
//! Production satisfies that by construction. Stage 3 is a BARRIER (R14): every
//! `files` row for a folder exists before any parse task for that folder runs,
//! so by the time a node is written its file is already there.
//!
//! A fixture that calls `upsert_node` directly skips the barrier. It is not
//! exercising a different contract — it just never performed the step that
//! production performs first. These helpers ARE that step: seed the file, then
//! write the node.
//!
//! # Why a trait rather than free functions
//!
//! Same shape, same arguments, same return type as the production method, so a
//! fixture reads identically and the only edit is the method name. A free
//! function would have forced every call site to be re-shaped, and re-shaping a
//! hundred fixtures is how an assertion quietly changes meaning.
//!
//! These are `#[cfg(test)]`: there is no path from a shipped binary to a
//! `files` row minted on demand.

use super::PgStore;

pub(crate) use super::folders::BARRIER_MTIME;

/// Seed the `files` row a node needs, the way stage 3's barrier would have.
///
/// Idempotent — fixtures name the same path from several nodes, and the second
/// one must not fail. `upsert_file_row` handles the conflict.
///
/// `content_hash` is the path itself: a fixture has no bytes to hash, and a
/// value that is stable per path is what the incremental gate wants (it compares
/// hashes to decide "unchanged", so a random value would make every fixture look
/// dirty on a re-seed).
async fn seed_file(pg: &PgStore, folder_id: &uuid::Uuid, file_path: &str) -> Result<(), String> {
    pg.upsert_file_row(folder_id, file_path, BARRIER_MTIME, file_path, None).await?;
    Ok(())
}

/// The fixture-side mirror of the graph writers, barrier included.
///
/// Each method seeds the file then delegates to its production counterpart, so
/// what is under test is the REAL writer — only the precondition is supplied.
#[allow(clippy::too_many_arguments)]
pub(crate) trait SeedGraph {
    async fn seed_node(
        &self,
        folder_id: &uuid::Uuid,
        kind: &str,
        name: &str,
        file_path: &str,
        parent_id: Option<&uuid::Uuid>,
        signature: Option<&str>,
        line_start: Option<i32>,
        line_end: Option<i32>,
    ) -> Result<uuid::Uuid, String>;

    /// Seeds the file named by `def`, if any. A reference stub carries no file
    /// by construction, and seeding nothing is the correct barrier for it.
    async fn seed_node_by_fqn(
        &self,
        folder_id: &uuid::Uuid,
        fqn: &str,
        kind: &str,
        name: &str,
        language: Option<&str>,
        def: Option<super::FqnDef<'_>>,
    ) -> Result<uuid::Uuid, String>;

    /// Seed a `files` row on its own, for a fixture that writes nodes through a
    /// path these helpers do not cover (a handler, a task, raw SQL).
    async fn seed_only_file(&self, folder_id: &uuid::Uuid, file_path: &str) -> Result<(), String>;
}

impl SeedGraph for PgStore {
    async fn seed_node(
        &self,
        folder_id: &uuid::Uuid,
        kind: &str,
        name: &str,
        file_path: &str,
        parent_id: Option<&uuid::Uuid>,
        signature: Option<&str>,
        line_start: Option<i32>,
        line_end: Option<i32>,
    ) -> Result<uuid::Uuid, String> {
        seed_file(self, folder_id, file_path).await?;
        self.upsert_node(
            folder_id, kind, name, file_path, parent_id, signature, line_start, line_end,
        )
        .await
    }

    async fn seed_node_by_fqn(
        &self,
        folder_id: &uuid::Uuid,
        fqn: &str,
        kind: &str,
        name: &str,
        language: Option<&str>,
        def: Option<super::FqnDef<'_>>,
    ) -> Result<uuid::Uuid, String> {
        if let Some(d) = def.as_ref() {
            seed_file(self, folder_id, d.file_path).await?;
        }
        self.upsert_node_by_fqn(folder_id, fqn, kind, name, language, def).await
    }

    async fn seed_only_file(&self, folder_id: &uuid::Uuid, file_path: &str) -> Result<(), String> {
        seed_file(self, folder_id, file_path).await
    }
}
