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

/// The `personas.session_slot` value for a persona name.
///
/// The persona reaches the daemon from a QUERY STRING, so its case is the
/// caller's choice, not a fact. The Keychain side settled this long ago —
/// `dojo_client::session::account_for` lowercases, with a test spelling out that
/// "Sensei-HQ" and "sensei-hq" must not become two half-signed-in states — and
/// the registry has to agree or the two halves of a sign-out disagree about
/// which row they are talking about.
///
/// One function for the write and the clear so they cannot drift: they did, and
/// `?persona=Sensei-HQ` deleted the credentials while matching no row, leaving
/// `session_slot` set for [`PgStore::signed_in_personas`] to keep enumerating.
fn slot_of(persona: &str) -> String {
    persona.to_lowercase()
}

impl PgStore {
    /// The KEYCHAIN SLOTS of personas that have completed a dōjō sign-in.
    ///
    /// The persona registry. `docs/spec/dojo/daemon-sync.md` §3 originally proposed
    /// a `sensei.dojo_personas` table for it; the registry already existed in
    /// `sensei.personas`, so the table was never created.
    ///
    /// **Returns `session_slot`, NOT `label`, and that is the whole subtlety.**
    /// The spec's first version claimed they were the same string. They are not:
    /// `link_persona_identity` REWRITES `label` to the verified GitHub login, so a
    /// user who signs in as `default` ends up with a row labelled `sensei-hq-org`
    /// whose session is still at `refresh_token.default`. Returning the label sent
    /// `live_access_token` looking for a slot that does not exist, which reported
    /// `SignedOut`, skipped the persona, and left the cycle claiming success while
    /// pushing nothing. Observed live before this column existed, not theorised.
    ///
    /// Both conditions are needed. `session_slot IS NOT NULL` is the one that
    /// matters — it is written at sign-in, cleared by
    /// [`Self::clear_persona_session`] at sign-out, and is the only field that
    /// names a real Keychain entry. `verified_at IS NOT NULL` is kept beside it
    /// so a row that was given a slot but never completed the OAuth callback is
    /// not enumerated.
    ///
    /// This comment used to claim `verified_at` was what kept a signed-out row
    /// from being re-enumerated. It was false in both halves: sign-out cleared
    /// neither column, and `verified_at` is never cleared at all, so the cycle
    /// went on listing signed-out personas every cadence forever.
    ///
    /// A row proves a sign-in HAPPENED, not that its token is still valid — the
    /// caller resolves a live access token per slot and skips the expired ones.
    pub async fn signed_in_personas(&self) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT session_slot FROM sensei.personas \
              WHERE session_slot IS NOT NULL AND verified_at IS NOT NULL \
              ORDER BY session_slot",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("signed_in_personas: {e}"))?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// Release a persona's Keychain slot — the DATABASE half of signing out.
    ///
    /// Returns whether a row actually held the slot, so the caller can tell "I
    /// forgot your session" from "there was nothing to forget". Reporting success
    /// either way would hide a persona/slot mismatch, which is the class of bug
    /// `session_slot` exists to make visible.
    ///
    /// Clears ONLY the slot. `verified_at`, `github_login` and `github_user_id`
    /// stay: which GitHub account this persona is remains true after a sign-out,
    /// and discarding it would force a second OAuth round trip to re-learn
    /// something already proved. [`Self::signed_in_personas`] requires the slot,
    /// so nulling it is sufficient to stop the sync cycle picking the row up.
    ///
    /// Takes the PERSONA as the caller has it — a query-string parameter whose
    /// case is not a fact — and normalises through [`slot_of`], the same function
    /// the sign-in writes with. Comparing the raw parameter meant
    /// `?persona=Sensei-HQ` cleared the Keychain (which lowercases) and matched
    /// no row, which is the original defect restored by a capital letter.
    pub async fn clear_persona_session(&self, persona: &str) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.personas SET session_slot = NULL, modified_at = now() \
              WHERE session_slot = $1",
        )
        .bind(slot_of(persona))
        .execute(&self.pool)
        .await
        .map_err(|e| format!("clear_persona_session: {e}"))?;
        Ok(res.rows_affected() > 0)
    }

    /// Create a persona, or return the existing one with that label.
    ///
    /// Idempotent on `lower(label)` so a re-run of a seed/backfill converges
    /// instead of erroring or duplicating.
    pub async fn upsert_persona(&self, label: &str, is_self: bool) -> Result<uuid::Uuid, String> {
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
    /// `persona_hint` is only a FALLBACK. The hint is the Keychain slot the
    /// session was loaded from, which is not evidence of who signed in — a slot
    /// named `sensei-hq` can perfectly well hold the jerrythomas account, as it
    /// does on this machine after an early double sign-in. Trusting it would
    /// stamp one human's verified login onto another's persona.
    ///
    /// Resolution runs strongest-evidence-first:
    ///
    /// 1. `github_user_id` — the same account, definitively. Not the login: a
    ///    login can be RENAMED and the numeric id cannot, so matching on the
    ///    login forks one human into two personas the day they rename.
    /// 2. `primary_email` against a live persona email — the same human under a
    ///    name we inferred earlier. Passed explicitly rather than taken from
    ///    `verified_emails`, which is unordered: the account's identifying
    ///    address is a specific one, not whichever happens to sort first.
    /// 3. The hint.
    /// 4. A new persona labelled with the verified login.
    ///
    /// Steps 2 and 3 refuse to land on a persona ALREADY verified as a different
    /// GitHub account. Two accounts sharing an address (a work address on both a
    /// personal and an org account) would otherwise let the second sign-in
    /// silently take over the first's persona and re-attribute its history.
    pub async fn link_persona_identity(
        &self,
        persona_hint: &str,
        github_login: &str,
        github_user_id: i64,
        primary_email: Option<&str>,
        verified_emails: &[String],
    ) -> Result<uuid::Uuid, String> {
        let id = match self
            .resolve_persona_for_identity(persona_hint, github_user_id, primary_email)
            .await?
        {
            Some(id) => id,
            None => self.upsert_persona(github_login, true).await?,
        };

        // The label is REPLACED by the verified login only while the persona is
        // still unverified, and only when no other persona already holds that
        // label. Once a user has chosen a display name, re-verification must not
        // silently overwrite it — preferring `sensei-hq` to `sensei-hq-org` is a
        // legitimate choice, not drift to correct.
        sqlx_core::query::query(
            "UPDATE sensei.personas p \
                SET github_login = $2, github_user_id = $3, verified_at = now(), \
                    label = CASE WHEN p.verified_at IS NULL AND NOT EXISTS ( \
                                      SELECT 1 FROM sensei.personas o \
                                       WHERE lower(o.label) = lower($2) AND o.id <> p.id) \
                                 THEN $2 ELSE p.label END, \
                    session_slot = $4, \
                    modified_at = now() \
              WHERE p.id = $1",
        )
        .bind(id)
        .bind(github_login)
        .bind(github_user_id)
        // The slot the session was actually stored under — the hint string, NOT
        // the label the line above may have just rewritten. Recording it here is
        // what keeps the registry's lookup key and the Keychain key the same
        // string; deriving one from the other is what silently skipped the
        // persona. Through [`slot_of`], so the sign-out that has to find this row
        // again normalises with the same function rather than a second copy of
        // the rule.
        .bind(slot_of(persona_hint))
        .execute(&self.pool)
        .await
        .map_err(|e| format!("link_persona_identity (verify): {e}"))?;

        for email in verified_emails {
            self.link_persona_email(&id, email, "claimed").await?;
        }
        Ok(id)
    }

    /// Which existing persona this GitHub account belongs to, if any.
    ///
    /// See [`Self::link_persona_identity`] for why the order is what it is.
    async fn resolve_persona_for_identity(
        &self,
        persona_hint: &str,
        github_user_id: i64,
        primary_email: Option<&str>,
    ) -> Result<Option<uuid::Uuid>, String> {
        let found: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.personas WHERE github_user_id = $1",
        )
        .bind(github_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("resolve_persona_for_identity (by id): {e}"))?;
        if let Some((id,)) = found {
            return Ok(Some(id));
        }

        // `github_user_id IS NULL OR = $2` is the claim guard: an unverified
        // persona is free to claim, a persona verified as THIS account is
        // already ours, and one verified as another account is off limits.
        if let Some(email) = primary_email {
            let by_email: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
                "SELECT p.id FROM sensei.personas p \
                   JOIN sensei.persona_emails pe ON pe.persona_id = p.id \
                  WHERE pe.removed_at IS NULL AND lower(pe.email) = lower($1) \
                    AND (p.github_user_id IS NULL OR p.github_user_id = $2) \
                  LIMIT 1",
            )
            .bind(email)
            .bind(github_user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("resolve_persona_for_identity (by email): {e}"))?;
            if let Some((id,)) = by_email {
                return Ok(Some(id));
            }
        }

        let by_hint: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.personas \
              WHERE lower(label) = lower($1) \
                AND (github_user_id IS NULL OR github_user_id = $2)",
        )
        .bind(persona_hint)
        .bind(github_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("resolve_persona_for_identity (by hint): {e}"))?;
        Ok(by_hint.map(|(id,)| id))
    }
}
