set search_path to sensei, extensions;

-- Where a playbook or playbook rule came from.
--
-- Shared by sensei.playbooks.source and sensei.playbook_rules.source, which
-- previously carried the same three-value CHECK constraint each. Two copies of
-- one value set drift; an enum is one definition and gives cleaner
-- introspection.
--
-- NAMED FOR THE CONCEPT, not for a member table. `source` alone would collide in
-- this schema — extension_source, metric_source and library_source_type already
-- exist — and `playbook_rules_source` would be arbitrary, since playbooks uses it
-- just as much.
--
--   builtin — ships with sensei
--   org     — authored by the organisation
--   learned — inferred from this install's own history
create type source_kind as enum ('builtin', 'org', 'learned');
