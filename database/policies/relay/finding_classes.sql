-- RLS for relay.finding_classes: one shared vocabulary, readable by all. Only
-- the daemon owner (seed import, learning loop) writes it.
alter table relay.finding_classes enable row level security;

drop policy if exists finding_classes_open_read on relay.finding_classes;
create policy finding_classes_open_read on relay.finding_classes
    for select to authenticated
    using (true);

grant select on relay.finding_classes to authenticated;
