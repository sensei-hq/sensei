set search_path to sensei, extensions;

-- Discriminator for activity.session_facet_tags. Goal categories and friction
-- kinds are both {name → weight} multisets per session, so they share one table
-- rather than two near-identical ones that would drift apart.
create type facet_tag_kind
    as enum ('goal', 'friction');
