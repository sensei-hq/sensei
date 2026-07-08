set search_path to dojo, extensions;

-- Lifecycle of a confidentiality incident (a near-leak on client work).
-- `open` = raised, unowned or unstarted; `investigating` = being contained;
-- `resolved` = closed (resolved_at set). Open count = rows where resolved_at
-- is null.
create type dojo.incident_status
    as enum ('open', 'investigating', 'resolved');
