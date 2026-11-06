set search_path to sensei, extensions;

-- A board this install must withdraw from a dōjō tenant (spec
-- docs/spec/daemon/tracker-delivery-metrics.md D3 "Withdrawal", B13; #355):
-- queued for every push mark of a tracker as it is deleted, by whatever path
-- (sensei.queue_delivery_withdrawals, a BEFORE DELETE trigger: a workspace pruned
-- or deleted takes its trackers too), and when a pushed board stops being
-- eligible; sent on the next dojo_sync tick and deleted when it lands, or when
-- the same board is pushed and acknowledged again.
create table if not exists delivery_withdrawals (
  board_key     text          not null check (board_key ~ '^[0-9a-f]{64}$')
, tenant_id     uuid          not null
, persona_id    uuid          not null references sensei.personas(id) on delete cascade
, queued_at     timestamptz   not null default now()
, attempted_at  timestamptz
, refused_at    timestamptz
, error         text
, primary key (board_key, tenant_id, persona_id)
);

comment on table delivery_withdrawals is
'Boards this install reported to a dōjō and must withdraw (B13): the tracker was deleted, or its workspace lost consent. Outlives the tracker by design — the board key is all the DELETE needs. Signing out queues nothing; a persona removed drops its queue (it can no longer sign the request).';

comment on column delivery_withdrawals.board_key
     is 'hex(sha256(kind + LF + base_url + LF + board_ref)), as pushed (crate::dojo::delivery::board_key).';
comment on column delivery_withdrawals.tenant_id
     is 'The dōjō tenant the board was pushed to (its push mark''s tenant).';
comment on column delivery_withdrawals.persona_id
     is 'The install-persona that pushed: the DELETE is signed by its session and names it as the source of the rows to remove.';
comment on column delivery_withdrawals.attempted_at
     is 'The last time the DELETE was tried; NULL until then.';
comment on column delivery_withdrawals.refused_at
     is 'When the dōjō last refused it (4xx other than its own 404): not tried again for an hour, so a refusal never fails every tick.';
comment on column delivery_withdrawals.error
     is 'Why the last try did not land; NULL when it has not been tried or landed.';
