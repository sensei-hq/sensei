set search_path to dojo, extensions;

-- How a contribution is credited when it leaves the machine. `named` = public
-- or org-internal credit to the author; `anonymous` = a stable rotated
-- anonymous id (collective default); `dereferenced` = source reference stripped
-- (mandatory for client work — the lesson travels, the source never does).
create type dojo.attribution_mode
    as enum ('named', 'anonymous', 'dereferenced');
