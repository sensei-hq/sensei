set search_path to dojo, sensei, extensions;

-- Seeds the default governance "constitution" — the curated baseline of
-- principles (mandatory), guardrails (required/recommended), guidelines
-- (advisory), and stack templates (recommended/advisory) that every sensei
-- ships knowing. Delivered as DATA in the global-dōjō (rows in
-- dojo.shared_rules), distributed down over the built Dōjō inbox, and served to
-- the assistant via the get_rules MCP tool. See
-- docs/spec/governance/default-constitution.md.
--
-- Namespace resolution: dojo.shared_rules.namespace_id FKs sensei.namespaces,
-- NOT dojo.tenants — there is no tenant→namespace FK. The global-dojo tenant
-- (dojo.tenants key='org/global-dojo', scope='global') is the *federation*
-- boundary; the *governance* namespace these rules carry is sensei.namespaces
-- (scope_key='general', slug='global-dojo'). scope_key='general' is deliberate:
-- resolve_global_rules() (senseid/src/db/pg_store.rs) only surfaces rules whose
-- scope_key IN ('general','user'), so 'general' is what makes the constitution
-- resolve *everywhere* through get_rules once it federates down. The namespace
-- row is created here (idempotent get-or-create), mirroring how the Dōjō store
-- upserts a namespace on publish (dojo-mind/src/store.rs::publish).
--
-- content_hash: sha256 hex of the trim+lowercased content — NOT md5. The spec
-- suggested md5, but the whole federation contract (dojo_protocol::content_hash,
-- governance.rs dedup, store.rs pull cursor) keys on sha256(trim+lower). Using
-- md5 here would make the seeded rows dedup inconsistently with every other rule
-- in the system. The spec's intent ("a reworded rule is a new hash → new row")
-- is preserved: a reworded content string yields a new sha256 → new row.
--
-- Idempotent: ON CONFLICT (namespace_id, content_hash) DO NOTHING. Safe to run
-- on every deploy/boot — re-running never duplicates and never overwrites an
-- edited row. published_by='sensei'; published_at=now() (ON CONFLICT dedups on
-- content_hash, not time, so a fresh now() on re-run is harmless — the conflict
-- keeps the original row untouched).
create or replace procedure dojo.seed_default_governance()
language plpgsql
as $$
declare
  ns_id uuid;
begin
  -- Resolve (or create, idempotently) the global-dōjō governance namespace at
  -- the always-on 'general' scope. Guard: the 'general' scope must be seeded
  -- (dojo-mind seed_scopes runs before this on boot); if it is missing, raise a
  -- clear notice and bail rather than FK-erroring cryptically.
  if not exists (select 1 from sensei.scopes where key = 'general') then
    raise notice 'seed_default_governance: sensei.scopes has no ''general'' row — run seed_scopes first; skipping.';
    return;
  end if;

  insert into sensei.namespaces (scope_key, slug, name)
  values ('general', 'global-dojo', 'Global Dōjō')
  on conflict (scope_key, slug) do update set name = excluded.name
  returning id into ns_id;

  -- Bulk insert every seed rule. content_hash = sha256(trim+lower(content)).
  -- (title, rule_type, enforcement, impact, content) — one row per rule.
  insert into dojo.shared_rules (
      namespace_id, content_hash, rule_type, title, content, impact,
      enforcement, status, version, origin_repo, published_by, published_at, updated_at
  )
  select
      ns_id,
      encode(sha256(lower(trim(v.content))::bytea), 'hex'),
      v.rule_type,
      v.title,
      v.content,
      v.impact,
      v.enforcement::sensei.enforcement,
      'active',
      1,
      null,
      'sensei',
      now(),
      now()
  from (
      values
      -- ── Constitution — principles (mandatory) ─────────────────────────────
        ('Constitution', 'Measure, then keep what helps',
         'Try a practice, measure its effect, keep it if it moves the number, drop it if it does not. No practice is sacred — the data decides.',
         'The core loop: practices earn their place by measured impact, not by tradition.',
         'mandatory')
      , ('Constitution', 'The right thing beats more things',
         'Better velocity is about direction, not raw speed. Ask of any change: is this the code that does what the user needs?',
         'Guards against shipping fast in the wrong direction.',
         'mandatory')
      , ('Constitution', 'Strong fundamentals first — AI amplifies whatever you already are',
         'Tests, small changes, and clear direction make AI a multiplier instead of a chaos engine.',
         'Strong fundamentals + AI = extraordinary; weak fundamentals + AI = more chaos.',
         'mandatory')
      , ('Constitution', 'Make it safe to question the AI',
         'A generative culture surfaces the assistant''s mistakes early; nobody rubber-stamps a model''s output.',
         'The 5th DORA key (generative culture) applied to human+AI pairing.',
         'mandatory')

      -- ── Guardrails — Quality (required) ───────────────────────────────────
      , ('Quality', 'Every change ships with a test',
         'Every change ships with a test.',
         'Tests catch AI hallucinations before production.',
         'required')
      , ('Quality', 'Never merge on red',
         'Never merge on red.',
         null,
         'required')
      , ('Quality', 'A human reviews AI-written code before it lands',
         'A human reviews AI-written code before it lands.',
         null,
         'required')

      -- ── Guardrails — Architecture (required) ──────────────────────────────
      , ('Architecture', 'Prefer the simplest design that passes the tests',
         'Prefer the simplest design that passes the tests.',
         null,
         'required')
      , ('Architecture', 'Refactor continuously, not in a separate phase',
         'Refactor continuously, not in a separate phase.',
         null,
         'required')
      , ('Architecture', 'Keep changes small and single-purpose',
         'Keep changes small and single-purpose.',
         null,
         'required')

      -- ── Guardrails — Process (required, one recommended) ──────────────────
      , ('Process', 'Integrate to trunk continuously',
         'Integrate to trunk continuously.',
         null,
         'required')
      , ('Process', 'Keep change lead time short',
         'Keep change lead time short (commit to production in hours).',
         null,
         'required')
      , ('Process', 'Daily plan and weekly review/retro in plain language',
         'Daily plan and weekly review/retro in plain language.',
         null,
         'required')
      , ('Process', 'Review with the Perfection Game',
         'Review with the Perfection Game ("what would make this a 10?").',
         null,
         'recommended')

      -- ── Guardrails — Tools (required) ─────────────────────────────────────
      , ('Tools', 'Keep the pipeline green and fast',
         'Keep the pipeline green and fast (a broken pipeline stops the line).',
         null,
         'required')
      , ('Tools', 'Automate the deploy so shipping is boring and on-demand',
         'Automate the deploy so shipping is boring and on-demand.',
         null,
         'required')

      -- ── Guardrails — Patterns (required, one recommended) ─────────────────
      , ('Patterns', 'Match the house style over a new idiom',
         'Match the house style over a new idiom.',
         null,
         'required')
      , ('Patterns', 'Reuse before a 4th near-duplicate',
         'Reuse before a 4th near-duplicate.',
         null,
         'recommended')

      -- ── Guidelines — practices (advisory) ─────────────────────────────────
      , ('Guidelines', 'Big-picture backlog',
         'Goals across time (last quarter, this month, next half), each with a why-it-matters, not a pile of tickets.',
         null,
         'advisory')
      , ('Guidelines', 'Plain English over jargon',
         'Say what you mean (sensei''s own insight copy follows this).',
         null,
         'advisory')
      , ('Guidelines', 'Sustainable pace',
         'The loop only compounds if it is still turning next month.',
         null,
         'advisory')

      -- ── Stack templates — Rust (adopt-per-dōjō) ───────────────────────────
      -- Net-new: high-value, terse, testable technology templates a dōjō
      -- adopts. Marked "[stack: <lang>] (template)" in the title so they read
      -- as opt-in defaults, not house law. rule_type='Stack'.
      , ('Stack', '[stack: rust] (template) clippy-clean',
         'Rust code is clippy-clean (cargo clippy passes with no warnings).',
         'Template — a dōjō adopts this for Rust projects.',
         'recommended')
      , ('Stack', '[stack: rust] (template) prefer Result over unwrap/panic',
         'Prefer Result over unwrap/panic/expect in non-test code.',
         'Template — a dōjō adopts this for Rust projects.',
         'recommended')
      , ('Stack', '[stack: rust] (template) no blocking in async',
         'No blocking calls in async contexts (use spawn_blocking or an async API).',
         'Template — a dōjō adopts this for Rust projects.',
         'advisory')

      -- ── Stack templates — TypeScript/Svelte (adopt-per-dōjō) ──────────────
      , ('Stack', '[stack: typescript] (template) strict types, no any',
         'Strict TypeScript; no implicit or explicit any.',
         'Template — a dōjō adopts this for TypeScript/Svelte projects.',
         'recommended')
      , ('Stack', '[stack: svelte] (template) Svelte 5 runes over legacy stores',
         'Use Svelte 5 runes ($state/$derived/$effect) over legacy stores.',
         'Template — a dōjō adopts this for Svelte projects.',
         'recommended')
      , ('Stack', '[stack: svelte] (template) tokens and scale, not literals',
         'Use design tokens and the type/spacing scale, not literal px/hex values (per the frontend guidelines).',
         'Template — a dōjō adopts this for Svelte/UI projects.',
         'advisory')

      -- ── Stack templates — Python (adopt-per-dōjō) ─────────────────────────
      , ('Stack', '[stack: python] (template) type hints',
         'Public functions carry type hints.',
         'Template — a dōjō adopts this for Python projects.',
         'recommended')
      , ('Stack', '[stack: python] (template) ruff clean',
         'Code is ruff-lint and ruff-format clean.',
         'Template — a dōjō adopts this for Python projects.',
         'recommended')
      , ('Stack', '[stack: python] (template) no bare except',
         'No bare except: catch specific exception types.',
         'Template — a dōjō adopts this for Python projects.',
         'advisory')
  ) as v(rule_type, title, content, impact, enforcement)
  on conflict (namespace_id, content_hash) do nothing;
end;
$$;
