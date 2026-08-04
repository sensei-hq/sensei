set search_path to staging, sensei, extensions;

-- Seed the library packs' RULES from staging.rule_pack_rules (dbd auto-runs on import).
-- Resolves each row's pack_slug → the GLOBAL sensei.rule_packs.id (owner_namespace_id
-- NULL), so it only ever touches library packs, never an org's private pack that
-- happens to share a slug. A row whose pack_slug is unknown is silently skipped (the
-- join drops it) — the pack import must land first (it does: import_rule_packs sorts
-- before import_rule_pack_rules alphabetically, and both run in one import pass).
--
-- Same strict guard as import_rule_packs: upsert on (pack_id, ordinal), update only
-- when the datafile's modified_at is STRICTLY newer than the live row — incremental,
-- never a full reload, never clobbers a prod edit.
create or replace procedure import_rule_pack_rules()
language plpgsql as $$
begin
  insert into sensei.rule_pack_rules (
      pack_id, ordinal, statement, body, rationale,
      enforcement, verification, checker_ref, remediation, skill_ref,
      applies_to, evidence, updated_at
  )
  select
      p.id
    , coalesce(stg.ordinal, 0)
    , stg.statement
    , coalesce(stg.body, '')
    , nullif(stg.rationale, '')
    , coalesce(nullif(stg.enforcement, ''), 'recommended')::sensei.enforcement
    , coalesce(nullif(stg.verification, ''), 'manual')::sensei.rule_check
    , nullif(stg.checker_ref, '')
    , nullif(stg.remediation, '')
    , nullif(stg.skill_ref, '')
    , coalesce(stg.applies_to, '{}'::jsonb)
    , coalesce(stg.evidence, '{}'::jsonb)
    , coalesce(stg.modified_at, now())
  from staging.rule_pack_rules stg
  join sensei.rule_packs p
    on p.slug = stg.pack_slug and p.owner_namespace_id is null
  where stg.statement is not null
  on conflict (pack_id, ordinal)
  do update set
      statement    = excluded.statement
    , body         = excluded.body
    , rationale    = excluded.rationale
    , enforcement  = excluded.enforcement
    , verification = excluded.verification
    , checker_ref  = excluded.checker_ref
    , remediation  = excluded.remediation
    , skill_ref    = excluded.skill_ref
    , applies_to   = excluded.applies_to
    , evidence     = excluded.evidence
    , updated_at   = excluded.updated_at
  where excluded.updated_at > sensei.rule_pack_rules.updated_at;
end;
$$;
