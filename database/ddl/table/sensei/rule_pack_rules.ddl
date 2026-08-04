set search_path to sensei, extensions;

-- The rules a pack contains — the pack OWNS these (they carry their full text and
-- structure), because a pack is namespace-agnostic and its rules don't belong to
-- any namespace until the pack is adopted. This is the authored SOURCE of a rule;
-- on adoption, resolution surfaces these for the adopting namespace (see
-- sensei.rule_pack_adoptions). Modelled from purpose, not the mockup's one-liner:
-- a rule must be understood (statement + rationale), applied (body), scoped
-- (applies_to), ranked (enforcement), verified (verification + checker_ref),
-- satisfied (remediation + skill_ref), and trusted (evidence). Shared plane
-- (D-LOCAL-PACKS) — see sensei.rule_packs.
create table if not exists rule_pack_rules (
  id            uuid        primary key default gen_random_uuid()
, pack_id       uuid        not null references rule_packs(id) on delete cascade
, ordinal       integer     not null default 0          -- display order within the pack

  -- ── the rule ────────────────────────────────────────────────────────────────
, statement     text        not null                    -- the headline imperative ("Never log secrets")
, body          text        not null default ''         -- the full rule: scope, examples, exceptions (markdown)
, rationale     text                                    -- why it exists (the "impact"/reason)
, enforcement   enforcement not null default 'recommended'   -- this rule's authority (may differ from the pack default)
, verification  rule_check  not null default 'manual'   -- how conformance is decided
, checker_ref   text                                    -- concrete verifier: a lint id / regex / command / skill name
, remediation   text                                    -- the positive pattern — how to satisfy it
, skill_ref     text                                    -- a marketplace skill that does/teaches it
, applies_to    jsonb       not null default '{}'::jsonb -- applicability predicate: { path_globs[], languages[], when }
, evidence      jsonb       not null default '{}'::jsonb -- learned-rule grounding: { corrections, incidents[], cite }

, created_at    timestamptz not null default now()
, updated_at    timestamptz not null default now()
);

-- Unique so the staging seed (staging.import_rule_pack_rules) can guarded-upsert on
-- (pack_id, ordinal); ordinal is the pack-local display order, one row per position.
-- NB the name is …_uq, NOT the former non-unique …_idx: `create unique index if not
-- exists` is a silent no-op when an index of the SAME name already exists, so on a DB
-- that already had the non-unique rule_pack_rules_pack_idx the uniqueness would never
-- take and the import's ON CONFLICT (pack_id, ordinal) fails with "no unique or
-- exclusion constraint matching". A fresh name is always created; any leftover old
-- index is a harmless duplicate until reconcile prunes it.
create unique index if not exists rule_pack_rules_pack_uq on rule_pack_rules(pack_id, ordinal);

comment on table rule_pack_rules is
'A pack''s owned rules, with their full text + enforcement structure. The authored
SOURCE of a rule (namespace-agnostic); adoption of the parent pack surfaces these
for a namespace. Prose (examples/exceptions) lives in `body`; the behaviour-driving
parts (verification/checker_ref/skill_ref/applies_to) are structured so the daemon
can scope injection to relevant files and point at (or run) the checker.';
comment on column rule_pack_rules.verification
     is 'How conformance is decided (sensei.rule_check): manual | checker | test | review. `checker`/`test` name the verifier in checker_ref.';
comment on column rule_pack_rules.applies_to
     is 'Applicability predicate finer than scope — e.g. {"path_globs":["**/auth/**"],"languages":["ts"]}. Empty = applies wherever the adopting namespace applies.';
comment on column rule_pack_rules.evidence
     is 'Grounding for a learned rule — e.g. {"corrections":4,"incidents":["INC-12"],"cite":"OWASP A02"}. Ties governance to the correction/incident data sensei already captures.';
