set search_path to sensei, extensions;

-- What a `library_content` row IS. R12: `library_skills`, `library_agents` and
-- `library_pages` were three near-identical tables, so a fifth kind of content
-- cost a fourth table and a fourth UNION arm in every "everything about this
-- library" query. As a discriminator it costs a row.
create type library_content_kind
    as enum ('skill', 'agent', 'page');
