set search_path to sensei, extensions;

-- THE module identity, in one place (#222).
--
-- Both the Structure diagram's `module` rollup (PgStore::structure_group_sql)
-- and `sensei.module_edges` need to name the same unit, and the Layers and
-- Cycles screens read the two side by side. Two spellings of this expression is
-- how a node ends up on one screen and its edges on another.
drop function if exists module_of(text, text) cascade;

create or replace function module_of(pkg text, mod text)
returns text
language sql
immutable
parallel safe
as $$
  select case
           -- NO FQN AT ALL. `split_part` on a null yields null, so this is the
           -- node that carries no identity, and it has no module rather than one
           -- rooted at the empty string.
           when mod is null then null
           -- A PACKAGE-ROOT SYMBOL IS IN ITS PACKAGE. The module segment is
           -- empty and present, which is a fact and not an absence (#231): since
           -- `indexer::fqn` stopped dropping it, `split_part(fqn, '·', 3)` yields
           -- `''` for a declaration written at the root rather than sliding the
           -- symbol's own NAME into the module's place.
           --
           -- The package's own name and not `pkg || '/'`, which reads as a
           -- truncation and sorts oddly beside `senseid/tasks`; and not null,
           -- which would drop a C header's whole contents off every structure
           -- diagram rather than showing them at the root.
           when mod = '' then pkg
           else pkg || '/' || split_part(split_part(mod, '/', 1), '::', 1)
         end
$$;

comment on function module_of(text, text) is
'The module a symbol belongs to, as one string — `<package>/<top module segment>`.

TWO ARGUMENTS, NOT ONE, AND THAT IS THE POINT. A bare module name is not unique
within a project: measured 2026-10-05 on project `sensei`, 922 (package, module)
pairs collapse to 914 distinct module names, so 6 names are reused across
packages (`lib` appears under more than one web package, `core::config` under
more than one crate). Keying on the name alone merges unrelated modules into one
node, which fabricates edges between them and can fabricate a CYCLE that exists
in no source file.

THE FIRST SEGMENT, NOT THE WHOLE MODULE PATH, and that is a measured choice
rather than a stylistic one. On sensei''s own corpus the fqn''s module segment is
per-FILE across most of the tree: grouping on it whole gives 1,446 groups over
1,751 files (1.21x) with 1,390 of them holding a single file, which is not a
level. Its first segment gives 150 groups over the same files (11.7x), and they
are the units a reader names — `senseid/tasks`, `senseid/api`.

Rust separates with `::` and the JS/TS trees with `/`; splitting on both leaves a
segment that already has no separator unchanged, so one expression covers every
language in the corpus.

NULL IN, NULL OUT. `||` propagates, so a symbol with no package has no module
identity rather than a plausible-looking one rooted at the empty string. Callers
filter it out; none may coalesce it to ''''.

IMMUTABLE so PostgreSQL INLINES it. The body is a single SELECT with no table
access, which makes it eligible for inlining into the calling query — verified by
EXPLAIN on `module_edges`, where the expression appears in the GROUP BY with no
Function Scan node. Marking it STABLE or VOLATILE would turn every row into a
function call and cost the plan its grouping.

Common queries:
  SELECT module_of(''senseid'', ''tasks::handlers::scan'')   -- senseid/tasks
  SELECT module_of(''senseid'', ''api/routes'')              -- senseid/api
  SELECT module_of(NULL, ''anything'')                       -- NULL';
