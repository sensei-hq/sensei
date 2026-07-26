set search_path to sensei, extensions;

-- The subject area a rule pack curates — the seven governance domains a pack
-- groups its rules under, independent of enforcement (authority) and scope
-- (where it applies). Presentation filters/groups packs by area; resolution
-- ignores it. `tech_stack` = language/framework rules; `design` = UX/design-system
-- rules (e.g. Rokkit's Zen-Sumi); `process` = how-we-work (reviews, commits,
-- releases). Declared in reading order (broad principles → specific process).
--
-- Lives in the `sensei` schema (D-LOCAL-PACKS): rule packs are a SHARED-plane
-- artifact — the tables deploy to the local daemon DB (`default` scope) AND to the
-- Dōjō Supabase (`dojo` scope). A shared schema is the idiom for "one table, two
-- scopes"; `sensei` already carries the shared governance closure (namespaces/scopes).
create type rule_pack_area
    as enum ('principles', 'architecture', 'security', 'compliance', 'tech_stack', 'design', 'process');
