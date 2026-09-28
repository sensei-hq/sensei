set search_path to sensei, extensions;

-- Who can act on a reason. An enum rather than text+CHECK, per the house rule.
create type reason_actor as enum ('user', 'organization');

comment on type reason_actor is
'Who the remedy is addressed to. NULL on the column when nobody can act — a
`normal` code resolves itself and names no actor.';
