set search_path to sensei, extensions;

-- What a supported tool is (#455): an `assistant` does the work (a harness that
-- runs a model); an `editor` hosts one. A word the catalogue does not list is
-- neither, and readers call it unknown — never one of these by default.
create type tool_kind as enum ('assistant', 'editor');
