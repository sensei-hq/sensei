set search_path to sensei, extensions;

-- What a folder IS, structurally. Two axes are mixed here and that is
-- deliberate — the value answers "why does this folder have a row?":
--
--   git               a repository root
--   workspace_member  a package DECLARED by an ancestor workspace root
--                     (package.json `workspaces`, Cargo.toml `[workspace]
--                     members`)
--   package           holds a manifest, but NO workspace declares it.
--                     A build unit in its own right — `marketplace/` here —
--                     that is not a member of anything. Distinct from
--                     workspace_member because membership is DECLARED, and
--                     calling an undeclared package a member would assert a
--                     relationship no manifest states (R4).
--   subtree           a nested git repo / git subtree
--   sibling           non-git, sits beside a repo
--   standalone        non-git, with no git siblings
--   folder            an ordinary directory inside a repo, holding no manifest.
--                     Written by heal_nested_standalone_roots when a directory
--                     wrongly registered as its own root is demoted back.
create type folder_kind
    as enum ('git', 'workspace_member', 'package', 'subtree', 'sibling', 'standalone', 'folder');
