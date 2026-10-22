-- Enum types the design no longer has. dbd's reconcile WARNS that an orphaned
-- enum is "dropped" but leaves it in place, and `dbd diff --exit-code` then
-- reports it — the release's verify step fails on a type nothing uses. Each is
-- dropped here, idempotently; a no-op once gone.
--
-- After, not before: dbd plans its reconcile before the before-hooks run, so a
-- type it ever does plan to drop is still there for it; this only sweeps what
-- it left.
--
-- 2026-10-22 (#307 R7, D34): a dōjō has no Clients section. Their tables went in
-- apply/before/dojo_clients_retired.sql, so nothing uses these.
drop type if exists dojo.engagement_status;
drop type if exists dojo.incident_severity;
drop type if exists dojo.incident_status;
