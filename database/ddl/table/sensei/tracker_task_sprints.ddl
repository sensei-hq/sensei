set search_path to sensei, extensions;

-- Which sprints list a ticket. A ticket carried over sits in every sprint that
-- listed it, as the provider models it. Spec: tracker-jira-linkage.md R1, R5.
create table if not exists tracker_task_sprints (
  task_id    uuid  not null references sensei.tracker_tasks(id) on delete cascade
, sprint_id  uuid  not null references sensei.sprints(id)       on delete cascade
, primary key (task_id, sprint_id)
);

create index if not exists tracker_task_sprints_sprint_id_idx
    on tracker_task_sprints(sprint_id);

comment on table tracker_task_sprints is
'Sprint membership of tickets (many-to-many). Replaced per fetched sprint on each sync; a removed sprint''s rows are deleted.';

comment on column tracker_task_sprints.task_id
     is 'The ticket.';
comment on column tracker_task_sprints.sprint_id
     is 'A sprint that lists it.';
