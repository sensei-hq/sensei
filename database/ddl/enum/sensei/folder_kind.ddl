set search_path to sensei, extensions;

-- What a folder IS. This answers "why does this folder have a row?" and
-- NOTHING ELSE — in particular it does not answer "who claims it", which is a
-- relationship and lives in `folders.workspace_root_id`.
--
--   git         a repository root
--   module      a manifest-bearing build unit inside a repo — a crate, an npm
--               package, a Go module. Whether a workspace DECLARES it is a
--               separate fact; see workspace_root_id.
--   subtree     a nested git repo / git subtree
--   standalone  non-git, with no git siblings. v1 scan paths still write it;
--               v2 does not, and it goes when those paths do (stages 4-7).
--   folder      an ordinary directory inside a repo, holding no manifest.
--               Written by heal_nested_standalone_roots when a directory
--               wrongly registered as its own root is demoted back.
--
-- `sibling` WAS here, meaning "a non-git directory sitting beside git
-- folders". Removed: v2's `folders` holds repo roots and manifest-bearing
-- modules, so a non-git sibling never gets a row. It had NO WRITER anywhere in
-- the tree — only a wire-enum variant and the parser arm that would have
-- deserialised it — so it named a value the system could not produce.
--
-- `workspace_member` and `package` WERE two values here, splitting modules by
-- whether an ancestor manifest declared them. Merged into `module`: that split
-- is a RELATIONSHIP, not a kind. It flips for several folders the moment
-- someone adds a `workspaces` array — nothing about the directory changes —
-- and a `kind` that moves under an unrelated edit is describing the wrong
-- thing. It also read as "Rust vs Node" in this repo purely because the root
-- Cargo.toml declares members and there is no root package.json, which is an
-- accident of layout and not a rule.
create type folder_kind
    as enum ('git', 'module', 'subtree', 'standalone', 'folder');
