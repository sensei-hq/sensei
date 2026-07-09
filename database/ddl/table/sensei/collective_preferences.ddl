set search_path to sensei, extensions;

-- The local user's collective-intelligence SHARING preferences — what leaves
-- this machine, toward whom, and how often. Backs the Preferences → Sharing
-- screen (docs/llm-spec/screen/observatory-collective.md, kanji 群) and gates
-- the Dōjō contribute loop (C6): destination + cadence + per-category filters +
-- the default attribution mode applied when an insight leaves the machine.
--
-- SINGLE-ROW pattern: this is one logical setting for the one local user, so the
-- table holds exactly one row. `singleton` is a boolean PK fixed to true by a
-- CHECK — the PK's uniqueness plus the CHECK make a second row impossible, and
-- the upsert keys on `ON CONFLICT (singleton)`. (Chosen over a sentinel uuid: no
-- magic constant to remember, and the constraint is self-documenting.)
create table if not exists collective_preferences (
  singleton            boolean     primary key default true  check (singleton)
, destination          text        not null default 'none'         -- none | global | dojo | both
, cadence              text        not null default 'manual'       -- manual | daily | weekly
, categories           jsonb       not null default '{}'::jsonb     -- per-category share toggles, e.g. {"memory":true,"pattern":true,...}
, attribution_default  text        not null default 'dereferenced'  -- named | anonymous | dereferenced (dojo.attribution_mode)
, updated_at           timestamptz not null default now()
);

comment on table collective_preferences is
'Single-row (singleton boolean PK) collective-intelligence sharing preferences for
the local user. Backs GET/PUT /api/preferences/collective and gates the Dōjō
contribute loop: destination (none|global|dojo|both), cadence (manual|daily|weekly),
per-category share toggles (jsonb), and the default attribution mode. When no row
exists the API returns conservative defaults (destination none, manual, attribution
dereferenced).';

comment on column collective_preferences.singleton
     is 'Fixed true (CHECK) — enforces exactly one row; the upsert keys ON CONFLICT (singleton).';
comment on column collective_preferences.destination
     is 'Where insights go: none | global (public commons) | dojo (company) | both. Default none — nothing leaves the machine.';
comment on column collective_preferences.cadence
     is 'How often batches are prepared: manual | daily | weekly. manual pauses batching until the user triggers next-batch.';
comment on column collective_preferences.categories
     is 'Per-category share toggles as jsonb {category: bool}. Categories: memory, pattern, rule, prompt, guard, skill, agent (spec Purpose list). A false value drops that kind from the next batch.';
comment on column collective_preferences.attribution_default
     is 'Default attribution applied when an insight leaves the machine: named | anonymous | dereferenced (mirrors dojo.attribution_mode / dojo_protocol::AttributionMode). Default dereferenced — the safest (source stripped).';
