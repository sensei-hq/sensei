set search_path to sensei, extensions;

-- A rule pack — the unit of CURATION · DISTRIBUTION · ADOPTION in the governance
-- plane. A namespace-agnostic LIBRARY artifact ("OWASP Top 10", "Clean Code",
-- "Rokkit Zen-Sumi", "Ponytail", or an org's private "Auth boundary guards") that
-- bundles a coherent set of rules (sensei.rule_pack_rules, which own their text)
-- under one area + source + default enforcement, versioned as a whole. A pack
-- governs nothing until a namespace ADOPTS it (sensei.rule_pack_adoptions);
-- resolution then unions the adopted packs' rules for a repo's namespaces.
--
-- SHARED plane (D-LOCAL-PACKS): lives in the `sensei` schema so it deploys to BOTH
-- the local daemon DB (`default` scope — curated packs resolve OFFLINE alongside
-- memories, without polluting the learned-memory store) AND the Dōjō Supabase
-- (`dojo` scope). When a user runs a dōjō the two planes sync; with no dōjō it is
-- just the local sharing mechanism. Distinct from dojo.shared_rules (the Dōjō-plane
-- per-namespace registry of individually promoted/learned rules) — a pack is the
-- reusable, cross-namespace source.
create table if not exists rule_packs (
  id                 uuid           primary key default gen_random_uuid()
, slug               text           not null      -- stable id, unique per owner (or globally when owner is NULL)
, name               text           not null      -- "Auth boundary guards"
, kanji              text                          -- 守 — optional display token
, area               rule_pack_area not null       -- the 7-set domain this pack curates
, attribution             text           not null      -- authority/provenance: "Robert C. Martin", "OWASP",
                                                   --   "PCI SSC", "Rokkit", "Ponytail · DietrichGebert"
, summary            text                          -- one-liner: why this bundle exists
, enforcement        enforcement    not null default 'recommended'  -- pack DEFAULT tier; a rule keeps its own
, owner_namespace_id uuid           references sensei.namespaces(id) on delete cascade
                                                   -- the ORG that authored a PRIVATE pack; NULL = global
                                                   -- curated library. Never a project — packs are cross-repo.
, version            integer        not null default 1   -- bumps on any membership/rule change; adoptions pin it
, status             text           not null default 'active'   -- active | draft | archived
, published_by       text           not null
, published_at       timestamptz    not null default now()
, updated_at         timestamptz    not null default now()
, constraint rule_packs_owner_slug unique (owner_namespace_id, slug)
  -- Shared vocabulary (see sensei.entity_scope / sensei.entity_origin).
  -- scope answers WHO MAY SEE IT and therefore whether it syncs; origin answers
  -- WHERE IT CAME FROM and therefore what a re-import may safely replace.
, scope         entity_scope  not null default 'local'
, origin        entity_origin not null default 'builtin'
);

-- Global packs (owner_namespace_id NULL) still need a unique slug — a NULL owner
-- makes the composite constraint above non-restrictive (NULLs are distinct), so a
-- partial index pins global-library slugs.
create unique index if not exists rule_packs_global_slug
    on rule_packs(slug) where owner_namespace_id is null;
create index if not exists rule_packs_area_idx on rule_packs(area);

comment on table rule_packs is
'A rule pack: a namespace-agnostic library bundle of governance rules (curation +
distribution + adoption). Rules live in sensei.rule_pack_rules (they own their text);
a namespace adopts the pack via sensei.rule_pack_adoptions. Shared plane
(D-LOCAL-PACKS) — deploys to both the local daemon DB and the Dōjō Supabase; curated
packs resolve offline, never poured into sensei.memories. Distinct from
dojo.shared_rules (the Dōjō-plane per-namespace resolved registry).';
comment on column rule_packs.owner_namespace_id
     is 'The org namespace that authored a private pack; NULL = a global curated library pack. Never a project — a pack is reusable across repos and scopes.';
comment on column rule_packs.enforcement
     is 'The pack''s DEFAULT enforcement tier. Each rule (rule_pack_rules.enforcement) may set its own; an adoption may override per-namespace (rule_pack_adoptions.enforcement).';
comment on column rule_packs.version
     is 'Pack version, bumped on any change to its rules/membership. An adoption pins the version it adopted (rule_pack_adoptions.pinned_version); re-adopt to take an update.';

-- ── Dōjō (Supabase) plane: security_invoker read path ───────────────────────────────────
-- dojo.rule_pack_library runs with security_invoker, so the CALLING role reads this table
-- directly. Grant + RLS limit `authenticated` to the GLOBAL curated library
-- (owner_namespace_id NULL, active) — private/org packs stay invisible even on a direct
-- PostgREST query. Inert on the local daemon (pure Postgres): it connects as the object
-- owner, which bypasses RLS, and never queries as `authenticated`. Idempotent for reconcile.
grant select on rule_packs to authenticated, service_role;
alter table rule_packs enable row level security;
drop policy if exists rule_packs_global_read on rule_packs;
create policy rule_packs_global_read on rule_packs
    for select to authenticated
    using (owner_namespace_id is null and status = 'active');

comment on column rule_packs.attribution is
'Human-readable credit for where this pack''s rules come from — "OWASP · sensei",
"Kent Beck · XP · sensei". Free text, deliberately.

Named `source` until 2026-08-24, which put a citation in the same column name
four other tables used for provenance. It is neither a scope nor an origin: those
are now sensei.entity_scope and sensei.entity_origin.';
