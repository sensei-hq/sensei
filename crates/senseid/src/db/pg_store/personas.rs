//! Persona resolution — turning a raw git author email into a working identity.
//!
//! `repository_metrics.identity` is whatever a commit's author trailer said. One
//! human routinely configures several addresses across machines and repos, and
//! deliberately keeps some apart (business vs personal vs employer). Reading
//! metrics per email therefore fragments one identity across many rows while also
//! risking merging identities the user wants separate.
//!
//! Resolution is a DERIVATION, never a rewrite: `identity` keeps the raw email and
//! `persona_id` is filled alongside it. That is what makes re-attribution
//! re-runnable — reassign an email to a different persona and re-resolve, with the
//! original assertion still there to resolve from.

use super::PgStore;

impl PgStore {
    /// Create a persona, or return the existing one with that label.
    ///
    /// Idempotent on `lower(label)` so a re-run of a seed/backfill converges
    /// instead of erroring or duplicating.
    pub async fn upsert_persona(
        &self,
        label: &str,
        is_self: bool,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.personas(label, is_self) VALUES($1, $2) \
             ON CONFLICT (lower(label)) DO UPDATE SET is_self = EXCLUDED.is_self, \
                                                      modified_at = now() \
             RETURNING id",
        )
        .bind(label)
        .bind(is_self)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("upsert_persona: {e}"))?;
        Ok(row.0)
    }

    /// Attach a git author email to a persona.
    ///
    /// Moving an email between personas is a legitimate correction (the user
    /// realises an address belongs to the other identity), so a conflict on the
    /// live-email index REASSIGNS rather than failing. Re-running
    /// [`Self::resolve_persona_ids`] afterwards moves the metric rows with it.
    pub async fn link_persona_email(
        &self,
        persona_id: &uuid::Uuid,
        email: &str,
        source: &str,
    ) -> Result<(), String> {
        // Clear any live claim by another persona first: the partial unique index
        // is on lower(email) across ALL personas, so a plain upsert on the
        // (persona_id, email) PK would hit the index instead of reassigning.
        sqlx_core::query::query(
            "UPDATE sensei.persona_emails SET removed_at = now() \
              WHERE lower(email) = lower($1) AND removed_at IS NULL AND persona_id <> $2",
        )
        .bind(email)
        .bind(persona_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("link_persona_email (clear): {e}"))?;

        sqlx_core::query::query(
            "INSERT INTO sensei.persona_emails(persona_id, email, source) VALUES($1, $2, $3) \
             ON CONFLICT (persona_id, email) DO UPDATE SET removed_at = NULL, \
                                                           source = EXCLUDED.source",
        )
        .bind(persona_id)
        .bind(email)
        .bind(source)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("link_persona_email: {e}"))?;
        Ok(())
    }

    /// Fill `repository_metrics.persona_id` from `identity` via the email map.
    ///
    /// Idempotent and re-runnable — that is the point. It rewrites the resolved
    /// column only, leaving `identity` untouched, so a persona reassignment is
    /// corrected by simply running this again.
    ///
    /// Rows whose email maps to no persona are set to NULL rather than guessed:
    /// an unrecognised author is genuinely unknown, and inventing an attribution
    /// would put someone else's commits in the user's own numbers.
    ///
    /// Returns rows updated.
    pub async fn resolve_persona_ids(&self) -> Result<u64, String> {
        let n = sqlx_core::query::query(
            "UPDATE sensei.repository_metrics rm \
                SET persona_id = pe.persona_id \
               FROM sensei.persona_emails pe \
              WHERE pe.removed_at IS NULL \
                AND lower(pe.email) = lower(rm.identity) \
                AND rm.identity IS NOT NULL \
                AND rm.persona_id IS DISTINCT FROM pe.persona_id",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("resolve_persona_ids: {e}"))?
        .rows_affected();
        Ok(n)
    }

    /// Every distinct `identity` in the metric store that no live persona email
    /// claims — the addresses awaiting assignment.
    ///
    /// This is what makes an unrecognised author VISIBLE instead of silently
    /// unattributed. Without it a new machine's gitconfig quietly produces
    /// personaless rows that no per-identity read ever surfaces.
    pub async fn unassigned_identities(&self) -> Result<Vec<(String, i64)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT rm.identity, count(*)::int8 \
               FROM sensei.repository_metrics rm \
              WHERE rm.identity IS NOT NULL \
                AND NOT EXISTS ( \
                      SELECT 1 FROM sensei.persona_emails pe \
                       WHERE pe.removed_at IS NULL \
                         AND lower(pe.email) = lower(rm.identity)) \
              GROUP BY rm.identity ORDER BY count(*) DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("unassigned_identities: {e}"))
    }

    /// Record a persona's VERIFIED GitHub identity, and adopt the account's
    /// emails as claimed aliases.
    ///
    /// Matches on `github_user_id` first, because a login can be renamed and the
    /// numeric id cannot — matching on the login would create a second persona
    /// for the same human the day they rename themselves.
    ///
    /// The label is REPLACED by the verified login only while the persona is
    /// still unverified. Once a user has chosen a display name, an OAuth refresh
    /// must not silently overwrite it — `sensei-hq` reading better than
    /// `sensei-hq-org` is a legitimate preference, not drift to correct.
    ///
    /// Emails arrive with `source='claimed'`: GitHub verified mailbox control,
    /// which is a materially stronger assertion than a git commit trailer that
    /// anyone can set to any address.
    pub async fn link_persona_identity(
        &self,
        persona_hint: &str,
        github_login: &str,
        github_user_id: i64,
        verified_emails: &[String],
    ) -> Result<uuid::Uuid, String> {
        // Existing persona for this GitHub account, or the hinted one, or new.
        let existing: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.personas WHERE github_user_id = $1",
        )
        .bind(github_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("link_persona_identity (by id): {e}"))?;

        let id = match existing {
            Some((id,)) => id,
            None => {
                let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
                    "INSERT INTO sensei.personas(label, is_self) VALUES($1, true) \
                     ON CONFLICT (lower(label)) DO UPDATE SET modified_at = now() \
                     RETURNING id",
                )
                .bind(persona_hint)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| format!("link_persona_identity (upsert): {e}"))?;
                row.0
            }
        };

        sqlx_core::query::query(
            "UPDATE sensei.personas \
                SET github_login = $2, github_user_id = $3, verified_at = now(), \
                    label = CASE WHEN verified_at IS NULL THEN $2 ELSE label END, \
                    modified_at = now() \
              WHERE id = $1",
        )
        .bind(id)
        .bind(github_login)
        .bind(github_user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("link_persona_identity (verify): {e}"))?;

        for email in verified_emails {
            self.link_persona_email(&id, email, "claimed").await?;
        }
        Ok(id)
    }
}

