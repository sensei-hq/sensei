set search_path to sensei, extensions;

-- Seeds the default governance "constitution" as BUNDLED RULE PACKS (D-SEED),
-- so a fresh install inherits a curated starter constitution offline instead of
-- an empty rules file. Replaces the retired dojo.seed_default_governance (which
-- wrote dojo.shared_rules and had zero callers — it "reached nobody"): the pack
-- is now the single source, and it deploys to BOTH planes (D-LOCAL-PACKS) — the
-- daemon-local `sensei` DB via the default scope AND the Dōjō Supabase via the
-- dojo scope — from this one definition. See docs/spec/governance/
-- default-constitution.md and docs/decisions.md D-SEED / D-LOCAL-PACKS.
--
-- Shape: the constitution splits into packs by rule_pack_area (the enum exists
-- to carry area, so use it): `default-principles` (mandatory), plus the
-- guardrails as `default-architecture` and `default-process`. These three are
-- AUTO-ADOPTED at the always-on general namespace (scope_key='general',
-- slug='global-dojo') so they resolve for every repo through get_rules with zero
-- dōjō. The `stack-templates` pack is SEEDED but NOT adopted — it is opt-in per
-- stack (a project adopts it when the stack matches), mirroring the spec's
-- "adopt-per-dōjō" intent.
--
-- Idempotent (safe on every deploy/boot): packs upsert on their global slug;
-- each pack's rules are cleared + re-inserted so this procedure is the source of
-- truth; adoptions reference packs by id and ON CONFLICT DO NOTHING. All refs
-- are schema-qualified because a procedure body runs under the CALLER's
-- search_path, not this file's (the same lesson as seed_ponytail_pack).
create or replace procedure sensei.seed_default_constitution()
language plpgsql
set search_path = sensei, extensions
as $$
begin
  -- The general/user always-on scopes are seeded by import_scopes (staging).
  -- Guard rather than FK-error cryptically if a caller runs this too early.
  if not exists (select 1 from sensei.scopes where key = 'general') then
    raise notice 'seed_default_constitution: sensei.scopes has no ''general'' row — run import_scopes first; skipping.';
    return;
  end if;

  -- ── Packs (global library: owner_namespace_id = null) ────────────────────
  insert into sensei.rule_packs
    (slug, name, kanji, area, attribution, summary, enforcement, owner_namespace_id, status, published_by)
  select v.slug, v.name, v.kanji, v.area::sensei.rule_pack_area, v.attribution, v.summary,
         v.enforcement::sensei.enforcement, null, 'active', 'sensei'
  from (values
    ('default-principles', 'Default constitution — principles', '憲', 'principles',
     'sensei default constitution (DORA · XP/CD · Core Protocols)',
     'The mandatory principles every sensei ships knowing: measure and keep what helps, direction over raw speed, strong fundamentals first, and make it safe to question the AI.',
     'mandatory'),
    ('default-architecture', 'Default constitution — architecture guardrails', '構', 'architecture',
     'sensei default constitution (DORA · XP/CD · Core Protocols)',
     'Design + pattern guardrails: the simplest design that passes the tests, continuous refactor, small single-purpose changes, match the house style, and reuse before a 4th near-duplicate.',
     'required'),
    ('default-process', 'Default constitution — process guardrails', '流', 'process',
     'sensei default constitution (DORA · XP/CD · Core Protocols)',
     'Quality, flow, tooling and practice guardrails: a test with every change, never merge on red, human review of AI code, trunk-based flow, short lead time, a green pipeline, and a sustainable pace.',
     'required'),
    ('stack-templates', 'Stack templates — opt-in per stack', '型', 'tech_stack',
     'sensei default constitution (stack templates)',
     'Opt-in, per-stack coding templates (Rust · TypeScript/Svelte · Python) a project adopts when the stack matches — seeded but not auto-adopted.',
     'recommended')
  ) as v(slug, name, kanji, area, attribution, summary, enforcement)
  on conflict (slug) where owner_namespace_id is null
    do update set name        = excluded.name,
                  kanji       = excluded.kanji,
                  area        = excluded.area,
                  attribution = excluded.attribution,
                  summary     = excluded.summary,
                  enforcement = excluded.enforcement,
                  status      = excluded.status,
                  version     = sensei.rule_packs.version + 1,
                  updated_at  = now();

  -- Re-sync every rule for these four packs to this procedure's definition.
  delete from sensei.rule_pack_rules r
    using sensei.rule_packs p
   where r.pack_id = p.id
     and p.owner_namespace_id is null
     and p.slug in ('default-principles', 'default-architecture', 'default-process', 'stack-templates');

  insert into sensei.rule_pack_rules
    (pack_id, ordinal, statement, body, rationale, enforcement, verification)
  select p.id, v.ordinal, v.statement, v.body, v.rationale,
         v.enforcement::sensei.enforcement, 'review'::sensei.rule_check
  from (values
    -- ── principles (mandatory) ───────────────────────────────────────────
    ('default-principles', 1, 'Measure, then keep what helps',
     'Try a practice, measure its effect, keep it if it moves the number, drop it if it does not. No practice is sacred — the data decides.',
     'The core loop: practices earn their place by measured impact, not by tradition.', 'mandatory'),
    ('default-principles', 2, 'The right thing beats more things',
     'Better velocity is about direction, not raw speed. Ask of any change: is this the code that does what the user needs?',
     'Guards against shipping fast in the wrong direction.', 'mandatory'),
    ('default-principles', 3, 'Strong fundamentals first — AI amplifies whatever you already are',
     'Tests, small changes, and clear direction make AI a multiplier instead of a chaos engine.',
     'Strong fundamentals + AI = extraordinary; weak fundamentals + AI = more chaos.', 'mandatory'),
    ('default-principles', 4, 'Make it safe to question the AI',
     'A generative culture surfaces the assistant''s mistakes early; nobody rubber-stamps a model''s output.',
     'The 5th DORA key (generative culture) applied to human+AI pairing.', 'mandatory'),

    -- ── architecture guardrails (required, one recommended) ───────────────
    ('default-architecture', 1, 'Prefer the simplest design that passes the tests',
     'Prefer the simplest design that passes the tests.', null, 'required'),
    ('default-architecture', 2, 'Refactor continuously, not in a separate phase',
     'Refactor continuously, not in a separate phase.', null, 'required'),
    ('default-architecture', 3, 'Keep changes small and single-purpose',
     'Keep changes small and single-purpose.', null, 'required'),
    ('default-architecture', 4, 'Match the house style over a new idiom',
     'Match the house style over a new idiom.', null, 'required'),
    ('default-architecture', 5, 'Reuse before a 4th near-duplicate',
     'Reuse before a 4th near-duplicate.', null, 'recommended'),

    -- ── process guardrails (required, some recommended/advisory) ──────────
    ('default-process', 1, 'Every change ships with a test',
     'Every change ships with a test.', 'Tests catch AI hallucinations before production.', 'required'),
    ('default-process', 2, 'Never merge on red',
     'Never merge on red.', null, 'required'),
    ('default-process', 3, 'A human reviews AI-written code before it lands',
     'A human reviews AI-written code before it lands.', null, 'required'),
    ('default-process', 4, 'Integrate to trunk continuously',
     'Integrate to trunk continuously.', null, 'required'),
    ('default-process', 5, 'Keep change lead time short',
     'Keep change lead time short (commit to production in hours).', null, 'required'),
    ('default-process', 6, 'Daily plan and weekly review/retro in plain language',
     'Daily plan and weekly review/retro in plain language.', null, 'required'),
    ('default-process', 7, 'Review with the Perfection Game',
     'Review with the Perfection Game ("what would make this a 10?").', null, 'recommended'),
    ('default-process', 8, 'Keep the pipeline green and fast',
     'Keep the pipeline green and fast (a broken pipeline stops the line).', null, 'required'),
    ('default-process', 9, 'Automate the deploy so shipping is boring and on-demand',
     'Automate the deploy so shipping is boring and on-demand.', null, 'required'),
    ('default-process', 10, 'Big-picture backlog',
     'Goals across time (last quarter, this month, next half), each with a why-it-matters, not a pile of tickets.', null, 'advisory'),
    ('default-process', 11, 'Plain English over jargon',
     'Say what you mean (sensei''s own insight copy follows this).', null, 'advisory'),
    ('default-process', 12, 'Sustainable pace',
     'The loop only compounds if it is still turning next month.', null, 'advisory'),

    -- ── stack templates (opt-in; seeded, not auto-adopted) ────────────────
    ('stack-templates', 1, '[stack: rust] (template) clippy-clean',
     'Rust code is clippy-clean (cargo clippy passes with no warnings).',
     'Template — a dōjō adopts this for Rust projects.', 'recommended'),
    ('stack-templates', 2, '[stack: rust] (template) prefer Result over unwrap/panic',
     'Prefer Result over unwrap/panic/expect in non-test code.',
     'Template — a dōjō adopts this for Rust projects.', 'recommended'),
    ('stack-templates', 3, '[stack: rust] (template) no blocking in async',
     'No blocking calls in async contexts (use spawn_blocking or an async API).',
     'Template — a dōjō adopts this for Rust projects.', 'advisory'),
    ('stack-templates', 4, '[stack: typescript] (template) strict types, no any',
     'Strict TypeScript; no implicit or explicit any.',
     'Template — a dōjō adopts this for TypeScript/Svelte projects.', 'recommended'),
    ('stack-templates', 5, '[stack: svelte] (template) Svelte 5 runes over legacy stores',
     'Use Svelte 5 runes ($state/$derived/$effect) over legacy stores.',
     'Template — a dōjō adopts this for Svelte projects.', 'recommended'),
    ('stack-templates', 6, '[stack: svelte] (template) tokens and scale, not literals',
     'Use design tokens and the type/spacing scale, not literal px/hex values (per the frontend guidelines).',
     'Template — a dōjō adopts this for Svelte/UI projects.', 'advisory'),
    ('stack-templates', 7, '[stack: python] (template) type hints',
     'Public functions carry type hints.',
     'Template — a dōjō adopts this for Python projects.', 'recommended'),
    ('stack-templates', 8, '[stack: python] (template) ruff clean',
     'Code is ruff-lint and ruff-format clean.',
     'Template — a dōjō adopts this for Python projects.', 'recommended'),
    ('stack-templates', 9, '[stack: python] (template) no bare except',
     'No bare except: catch specific exception types.',
     'Template — a dōjō adopts this for Python projects.', 'advisory')
  ) as v(pack_slug, ordinal, statement, body, rationale, enforcement)
  join sensei.rule_packs p on p.slug = v.pack_slug and p.owner_namespace_id is null;

  -- Make the lint-checkable stack templates enforceable (D-CHECKER): a
  -- `verification = 'checker'` rule whose `checker_ref` names a canonical command
  -- verb runs that verb's discovered command and yields a pass/fail verdict. Only
  -- the "clean/strict-lint" templates map cleanly to the repo's `lint` command;
  -- the rest stay `review` (manual). checker_ref = the canonical verb, resolved
  -- per-repo against sensei.project_commands.
  update sensei.rule_pack_rules r
     set verification = 'checker'::sensei.rule_check, checker_ref = 'lint'
    from sensei.rule_packs p
   where r.pack_id = p.id
     and p.owner_namespace_id is null
     and p.slug = 'stack-templates'
     and r.statement in (
       '[stack: rust] (template) clippy-clean',
       '[stack: typescript] (template) strict types, no any',
       '[stack: python] (template) ruff clean');

  -- ── Auto-adopt the constitution at the always-on general namespace ────────
  -- The stack-templates pack is deliberately excluded — it is opt-in per stack.
  insert into sensei.namespaces (scope_key, slug, name)
  values ('general', 'global-dojo', 'Global Dōjō')
  on conflict (scope_key, slug) do update set name = excluded.name;

  insert into sensei.rule_pack_adoptions (pack_id, namespace_id, pinned_version, adopted_by)
  select p.id, n.id, p.version, 'sensei'
  from sensei.rule_packs p
  join sensei.namespaces n on n.scope_key = 'general' and n.slug = 'global-dojo'
  where p.owner_namespace_id is null
    and p.slug in ('default-principles', 'default-architecture', 'default-process')
  on conflict (pack_id, namespace_id) do nothing;
end;
$$;
