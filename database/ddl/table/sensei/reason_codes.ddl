set search_path to sensei, extensions;

-- "Why didn't this happen?" — asked of repository sharing, scheduled workers,
-- metric pushes, governance pulls and pack adoptions, answered from ONE place.
--
-- Each of those used to answer differently: a boolean, a free-text last_error, a
-- log line, or nothing. Five shapes for one question, and every UI that wanted to
-- explain any of them re-implemented the vocabulary.
--
-- REPORTING DATA ONLY. This table holds no predicate, no condition, no
-- threshold, no branch. The DOMAIN decides which code applies — the view or the
-- enforcing procedure — and this only knows what the code means to a reader and
-- who can act on it. The test: delete every row and the system must behave
-- IDENTICALLY, losing only the prose. If deleting it would change behaviour,
-- logic has leaked in. Columns that must never exist here: condition, predicate,
-- when, sql, expression, threshold, applies_if.
--
-- See docs/architecture/reason-codes.md.
create table if not exists reason_codes (
  -- Which subsystem's vocabulary. Scoping by domain is what keeps `precedence`
  -- meaningful: "fix the subscription before the election" is a real ordering,
  -- "fix the subscription before the schedule window" is not.
  domain      text                not null
, code        text                not null
, kind        sensei.reason_kind  not null
  -- Lower = fix this FIRST, ordered within a domain. A thing can fail several
  -- ways at once; without an explicit order the answer depends on which SQL
  -- branch happened to run first, which is the accidental behaviour this registry
  -- exists to remove. Unique per domain so an accidental overlap fails loudly
  -- rather than silently picking one.
, precedence  smallint            not null
, summary     text                not null
, detail      text                not null
  -- What the READER can do. NULL when nothing can be done.
, remedy      text
, actor       sensei.reason_actor
, created_at  timestamptz         not null default now()
, modified_at timestamptz         not null default now()
, primary key (domain, code)
, unique (domain, precedence)
  -- `normal` means it clears itself, so it addresses nobody. A remedy on a
  -- `normal` row is the contradiction that made `forge_visibility_unknown` look
  -- benign while it was permanently stuck.
, constraint reason_codes_normal_is_silent
      check (kind <> 'normal' or (remedy is null and actor is null))
);

comment on table reason_codes is
'Human-readable reasons, keyed (domain, code). REPORTING ONLY — the domain
decides which code applies; this says what it means and who can act.

Delete every row and behaviour must be identical, losing only the prose. That is
the test for whether logic has leaked in.

Seeded from database/import/staging/reason_codes.jsonl.';

comment on column reason_codes.domain
     is 'The subsystem whose vocabulary this belongs to (repository_sharing, schedule, metric_push, …). Scopes `precedence`, which is meaningless across domains.';
comment on column reason_codes.kind
     is 'normal | refusal | fault. The axis is "does this clear itself without a human?", not "did anyone decide?".';
comment on column reason_codes.precedence
     is 'Lower = fix first, WITHIN a domain. A thing failing several ways reports the lowest; without this the answer depends on SQL branch order.';
comment on column reason_codes.remedy
     is 'What the reader can do about it. NULL when nobody can — and always NULL for a `normal` code, which the CHECK enforces.';
comment on column reason_codes.actor
     is 'Who the remedy addresses. NULL when nobody can act.';

-- Read cross-plane through dojo.reason_codes. The BASE table grant is required,
-- not just the view's: `security_invoker = on` runs as the caller, the Worker
-- holds a service_role key, and dojo.metric_catalogue answered "permission
-- denied" until both were granted. Inert on the daemon, whose owner connection
-- never queries as either role.
grant select on reason_codes to authenticated, service_role;
