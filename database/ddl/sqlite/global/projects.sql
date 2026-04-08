-- projects
-- Unified registry of repos and ideas. kind discriminates between them.
-- 'repo'  — backed by a git repository, has local_path and/or remote_url
-- 'idea'  — pre-code concept, no repo yet; may graduate to a repo later
--
-- maturity: 0=seed 1=sprout 2=growing 3=active 4=stable 5=mature
-- Computed by the daemon from card count, code presence, and last activity.
-- Developer can override by setting maturity_override.

create table if not exists projects (
  id                text    not null primary key
, kind              text    not null default 'repo'
                            check (kind in ('repo', 'idea'))
, name              text    not null
, description       text
, local_path        text                          -- repos only; null for ideas
, remote_url        text                          -- repos only; null for ideas
, default_branch    text                          -- repos only
, stack             text    not null default '[]' -- JSON: ["typescript","react"]
, entry_points      text    not null default '[]' -- JSON: [{path, role}]
, maturity          integer not null default 0
                            check (maturity between 0 and 5)
, maturity_override integer                       -- developer-set; overrides computed
                            check (maturity_override between 0 and 5)
, is_archived       integer not null default 0    -- 0=active 1=archived
, last_indexed_at   text                          -- ISO 8601
, last_indexed_commit text                        -- git SHA
, is_public         integer not null default 0    -- 0=private 1=public (telemetry stub)
, graduated_from    text    references projects(id) on delete set null
                                                  -- idea→repo graduation link
, created_at        text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at       text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by       text    not null default 'system'
);

create unique index if not exists projects_local_path_ukey
  on projects(local_path) where local_path is not null;

create unique index if not exists projects_remote_url_ukey
  on projects(remote_url) where remote_url is not null;

create index if not exists projects_kind_archived_idx
  on projects(kind, is_archived, modified_at desc);
