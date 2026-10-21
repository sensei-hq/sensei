set search_path to sensei, extensions;

-- Which tracker tickets an AI session worked on, and how we know.
-- Spec: docs/spec/daemon/tracker-jira-linkage.md R2, R6.
create table if not exists session_task_links (
  session_id   uuid         not null references activity.sessions(id) on delete cascade
, task_id      uuid         not null references sensei.tracker_tasks(id) on delete cascade
, source       link_source  not null
, matched_key  text         not null
, mentions     integer      not null check (mentions > 0)
, linked_at    timestamptz  not null default now()
, primary key (session_id, task_id, source)
);

create index if not exists session_task_links_task_id_idx
    on session_task_links(task_id);

comment on table session_task_links is
'A session linked to a tracker ticket because its branch or prompts carry one of the ticket''s keys (current or former). Recomputed on every sync of the ticket''s tracker (a full relink in the sync''s apply transaction). Only keys the board actually has can link.';

comment on column session_task_links.session_id
     is 'The AI session.';
comment on column session_task_links.task_id
     is 'The ticket.';
comment on column session_task_links.source
     is 'Where the key was found: branch or prompt (commit is reserved).';
comment on column session_task_links.matched_key
     is 'The key as matched, upper-cased with its digits as written (an old key of a renamed issue stays as it was typed).';
comment on column session_task_links.mentions
     is 'branch: distinct branches carrying the key; prompt: occurrences across the session''s prompts.';
comment on column session_task_links.linked_at
     is 'When the link was (re)computed.';
