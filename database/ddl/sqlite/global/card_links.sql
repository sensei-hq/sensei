-- card_links
-- Typed, bidirectional edges in the project knowledge graph.
-- Links connect cards to other cards, code symbols, files, or sessions.
-- All edges are stored once; the application queries both directions.
--
-- relation types:
--   drives        — card A drives card B (requirement → design)
--   implements    — card A implements card B (task → requirement)
--   references    — card A references a symbol or file (informational)
--   informs       — card A (decision/ADR) informs card B
--   supersedes    — card A supersedes card B (use cards.superseded_by for canonical link)
--   blocks        — card A blocks card B
--   addresses     — session addressed this card (session → card)
--
-- target_type:
--   card          — target_ref is a cards.id
--   symbol        — target_ref is "<file_path>#<symbol_name>"
--   file          — target_ref is a repo-relative file path
--   session       — target_ref is a sessions.id

create table if not exists card_links (
  id          text not null primary key
, source_id   text not null references cards(id) on delete cascade
, relation    text not null
                   check (relation in ('drives','implements','references',
                                       'informs','supersedes','blocks','addresses'))
, target_type text not null
                   check (target_type in ('card','symbol','file','session'))
, target_ref  text not null   -- card id, "file#symbol", file path, or session id
, note        text            -- optional annotation on why this link exists
, created_at  text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by text not null default 'system'
, unique(source_id, relation, target_type, target_ref)
);

create index if not exists card_links_source_idx  on card_links(source_id, relation);
create index if not exists card_links_target_idx  on card_links(target_type, target_ref);
