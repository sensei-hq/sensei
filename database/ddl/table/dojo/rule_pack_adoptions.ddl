set search_path to dojo, sensei, extensions;

-- A namespace adopting a rule pack — the bind that makes a library pack actually
-- govern. `namespace_id` is WHOEVER adopts: the org namespace at org-level, a
-- project namespace at project-level, a stack namespace at stack-level (it is a
-- concrete sensei.namespaces instance — polymorphic over the scope ladder, not a
-- project). Once adopted, resolution (get_rules) unions the pack's rules for any
-- repo whose namespaces include this one, so they flow through the SessionStart /
-- PreCompact push. Adoption is atomic (the whole pack) and pins a version.
create table if not exists dojo.rule_pack_adoptions (
  pack_id        uuid        not null references dojo.rule_packs(id)   on delete cascade
, namespace_id   uuid        not null references sensei.namespaces(id) on delete cascade
, pinned_version integer     not null      -- the rule_packs.version adopted; re-adopt to take an update
, enforcement    enforcement               -- optional per-adoption tier override (e.g. adopt an advisory
                                            --   pack as `required` for this org); NULL = each rule's own tier
, adopted_by     text        not null
, adopted_at     timestamptz not null default now()
, updated_at     timestamptz not null default now()
, primary key (pack_id, namespace_id)
);

create index if not exists rule_pack_adoptions_ns_idx on dojo.rule_pack_adoptions(namespace_id);

comment on table dojo.rule_pack_adoptions is
'A namespace''s adoption of a rule pack — the bind that makes a library pack govern.
namespace_id is the adopting scope instance (org id at org-level, project at
project-level, stack at stack-level). Resolution unions an adopted pack''s rules for
every repo whose namespace set includes namespace_id. Pins a pack version; a per-
adoption enforcement override can raise (or lower) the whole pack''s tier locally.';
comment on column dojo.rule_pack_adoptions.namespace_id
     is 'The adopting namespace — a concrete sensei.namespaces instance, polymorphic over the scope ladder (organization / project / stack / …). Org-level adoption uses the org''s namespace id.';
