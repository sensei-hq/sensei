set search_path to dojo, extensions;

-- How a contribution is credited when it leaves the machine. `named` = public
-- or org-internal credit to the author; `anonymous` = a stable rotated
-- anonymous id (the collective default; client work is anonymous with no id).
-- Source-dereference is NOT a mode — it is an always-on transform on the
-- publish path (every artifact is stripped regardless of credit).
create type dojo.attribution_mode
    as enum ('named', 'anonymous');
