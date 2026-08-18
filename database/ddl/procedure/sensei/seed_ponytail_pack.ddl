set search_path to sensei, extensions;

-- Seeds the "Ponytail" global-library rule pack — the minimal-solution coding
-- discipline (YAGNI · reuse-first · stdlib/native over dependencies · minimal
-- diff), adapted from DietrichGebert/ponytail (MIT) and expressed as sensei
-- RULES so it resolves on the scope ladder + pushes via D-INJECT. See
-- docs/spec/governance/ponytail-pack.md and docs/decisions.md D-PACK-KIND.
--
-- Shared plane (D-LOCAL-PACKS): the pack tables live in the `sensei` schema, so
-- this procedure seeds BOTH the local daemon DB (default scope) and the Dōjō
-- Supabase (dojo scope) from one definition. A global-library pack:
-- owner_namespace_id = NULL, area = principles, default enforcement = recommended.
-- It governs nothing until a namespace ADOPTS it (sensei.rule_pack_adoptions).
-- Idempotent: the pack is upserted on its global slug, and its rule set is cleared
-- + re-inserted so edits to THIS procedure are the source of truth on every
-- re-run. Adoptions reference the pack by id (never rule ids) and union its
-- CURRENT rules, so re-seeding never breaks an adoption.
--
-- Note the plpgsql variable is `v_pack_id`, NOT `pack_id`: a variable named
-- `pack_id` would shadow the `rule_pack_rules.pack_id` column in the DELETE's
-- WHERE (pack_id = pack_id → always true → wipes every pack's rules).
create or replace procedure sensei.seed_ponytail_pack()
language plpgsql
set search_path = sensei, extensions
as $$
declare
  v_pack_id uuid;
begin
  insert into sensei.rule_packs
    (slug, name, kanji, area, source, summary, enforcement, owner_namespace_id, status, published_by)
  values
    ('ponytail', 'Ponytail — minimal-solution discipline', '省', 'principles',
     'Ponytail · DietrichGebert (MIT)',
     'The laziest solution that actually works: don''t write what you don''t need, reuse before you build, prefer the platform over a dependency, keep the diff minimal.',
     'recommended', null, 'active', 'sensei')
  on conflict (slug) where owner_namespace_id is null
    do update set name        = excluded.name,
                  kanji       = excluded.kanji,
                  area        = excluded.area,
                  source      = excluded.source,
                  summary     = excluded.summary,
                  enforcement = excluded.enforcement,
                  status      = excluded.status,
                  version     = sensei.rule_packs.version + 1,
                  updated_at  = now()
  returning id into v_pack_id;

  -- Re-sync the pack's rules to this procedure's definition.
  delete from sensei.rule_pack_rules where pack_id = v_pack_id;

  insert into sensei.rule_pack_rules
    (pack_id, ordinal, statement, body, rationale, enforcement, verification, remediation)
  values
    (v_pack_id, 1,
     'Question whether the code needs to exist at all.',
     'Before writing, ask if the requirement is real and needed now. Prefer deleting or not writing over adding. Do not build for imagined future needs — add it when a real second caller appears.',
     'The cheapest, most correct, most secure code is the code you never write.',
     'recommended', 'review',
     'State the concrete requirement the code serves; if you cannot, cut it.'),

    (v_pack_id, 2,
     'Reuse what already exists before writing something new.',
     'Search the codebase and its libraries for an existing function, type, or pattern that already does this. Three near-identical lines are a sign to refactor, not a reason to add a fourth.',
     'Duplication is debt; reuse compounds quality and shrinks the surface to maintain.',
     'recommended', 'review',
     'Run sensei `search` / `get_duplicates` before adding; if a shared implementation exists, call it.'),

    (v_pack_id, 3,
     'Prefer the standard library over a new dependency.',
     'Reach for the language/runtime standard library first. Add a third-party dependency only when it clearly earns its weight over what the stdlib already provides.',
     'Every dependency is a maintenance, supply-chain, and security surface.',
     'advisory', 'review',
     'Justify a new dependency against the stdlib equivalent; prefer the stdlib when parity is close.'),

    (v_pack_id, 4,
     'Prefer native platform features over a library.',
     'Use built-in language, runtime, and framework capabilities before pulling a package (e.g. native fetch over an HTTP client, platform CSS over a styling framework when it suffices).',
     'Fewer moving parts; less to break, patch, and ship.',
     'advisory', 'review',
     'Check for a native/built-in way first; adopt a library only where the platform genuinely falls short.'),

    (v_pack_id, 5,
     'Choose the minimal solution — one line over fifty; no unrequested abstraction.',
     'Solve the actual problem in front of you. Do not add layers, configuration, or generality nobody asked for. Add an abstraction when there is a real second caller, not in anticipation of one.',
     'Abstraction has a carrying cost; premature generality is harder to read, change, and verify.',
     'recommended', 'review',
     'Prefer the smallest change that meets the acceptance criteria; introduce structure only when duplication or a concrete second use demands it.'),

    (v_pack_id, 6,
     'Keep the diff as small as the change requires.',
     'Touch only what the task needs. Avoid drive-by rewrites, reformatting, and unrelated refactors in the same change.',
     'Small diffs review cleanly, revert cleanly, and localize blame.',
     'advisory', 'review',
     'Split unrelated improvements into their own change; keep this one focused.');
end;
$$;
