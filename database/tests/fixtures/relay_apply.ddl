-- Applies the real relay DDL inside the caller's transaction (tests only —
-- deploys go through dbd). Skips when the schema already exists (converged):
-- the same files pass pre- and post-merge. Rolls back with the caller.
-- Paths are the real DDL: nothing here is a copy.
select count(*) > 0 as have_relay from pg_namespace where nspname = 'relay';
\gset
\if :have_relay
select 'relay present; self-apply skipped' as relay_apply;
\else
create schema if not exists relay;
create schema if not exists staging;

\ir ../../ddl/enum/relay/lane_role.ddl
\ir ../../ddl/enum/relay/lease_kind.ddl
\ir ../../ddl/enum/relay/lease_state.ddl
\ir ../../ddl/enum/relay/item_kind.ddl
\ir ../../ddl/enum/relay/item_state.ddl
\ir ../../ddl/enum/relay/task_phase.ddl
\ir ../../ddl/enum/relay/task_state.ddl
\ir ../../ddl/enum/relay/action_kind.ddl
\ir ../../ddl/enum/relay/limit_scope.ddl
\ir ../../ddl/enum/relay/run_state.ddl
\ir ../../ddl/enum/relay/task_executor.ddl
\ir ../../ddl/table/relay/crews.ddl
\ir ../../ddl/table/relay/runs.ddl
\ir ../../ddl/table/relay/lanes.ddl
\ir ../../ddl/table/relay/lane_waiters.ddl
\ir ../../ddl/table/relay/leases.ddl
\ir ../../ddl/table/relay/items.ddl
\ir ../../ddl/table/relay/run_items.ddl
\ir ../../ddl/table/relay/tasks.ddl
\ir ../../ddl/table/relay/actions.ddl
\ir ../../ddl/table/relay/finding_classes.ddl
\ir ../../ddl/table/relay/daemon_user.ddl
\ir ../../ddl/table/staging/finding_classes.ddl
\ir ../../ddl/procedure/staging/import_finding_classes.ddl
\ir ../../ddl/view/relay/now.ddl
\ir ../../ddl/view/relay/task_queue.ddl
\ir ../../ddl/view/relay/task_history.ddl
\ir ../../policies/relay/crews.sql
\ir ../../policies/relay/lanes.sql
\ir ../../policies/relay/leases.sql
\ir ../../policies/relay/items.sql
\ir ../../policies/relay/tasks.sql
\ir ../../policies/relay/runs.sql
\ir ../../policies/relay/run_items.sql
\ir ../../policies/relay/lane_waiters.sql
\ir ../../policies/relay/actions.sql
\ir ../../policies/relay/finding_classes.sql
\ir ../../policies/relay/daemon_user.sql
\ir ../../policies/relay/now.sql
\ir ../../policies/relay/task_history.sql
\ir ../../policies/relay/task_queue.sql
\ir ../../apply/after/relay_api_grants.sql
\endif
