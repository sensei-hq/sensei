set search_path to sensei, extensions;

-- Who may see a shareable entity — and therefore whether it syncs.
--
-- Every shareable entity had invented its own answer to this, and none of them
-- agreed. Measured before this type existed:
--
--   memories.scope         global | project     ← a visibility, but its own vocabulary
--   consolidated_rulesets.scope  global         ← same word, unrelated text column
--   rule_packs.source      'OWASP · sensei'     ← a CITATION wearing the name
--   playbooks.source       builtin              ← provenance
--   library_skills.source  manifest             ← provenance, different vocabulary
--   library_agents.source  manifest
--   intake_guide.source    builtin
--   playbook_rules.source  builtin
--   federated_memories     (nothing at all)
--
-- So `source` carried TWO distinct ideas — a citation and a provenance — and
-- `scope` carried a visibility in two incompatible spellings. Nothing anywhere
-- expressed the axis that actually matters for sharing: local vs remote.
--
-- This type is that axis, and only that axis. Provenance lives in
-- `entity_origin`; the human-readable credit lives in a plain `attribution`
-- column that no longer pretends to be an enum.
create type entity_scope as enum (
  -- This install only. Never leaves the machine.
    'local'
  -- Scoped to one project; shared with whoever shares that project.
  , 'project'
  -- Shared across a tenant (organisation).
  , 'tenant'
  -- Published to the marketplace. `marketplace/catalog.json` is already a real
  -- distribution channel; this is the first time the database can say so.
  , 'public'
);
