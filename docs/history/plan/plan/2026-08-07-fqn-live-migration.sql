-- FQN symbol-table rebuild — live-DB deploy gates (Phase 7.3).
-- Surgical, idempotent DDL that brings the live `sensei` DB's FQN-relevant schema
-- in line with database/ddl/table/sensei/nodes.ddl (+ node_kind enum). Applied with
-- the daemon STOPPED (no concurrent writes). Additive + behaviour-preserving:
-- every existing row has a non-null file_path, so the partial identity index covers
-- exactly the rows the old table constraint did.
--
-- Scope note: this is JUST the FQN gates — NOT a full `dbd reconcile` (which would
-- also apply ~60 commits of unrelated schema drift). The full schema sync rides the
-- develop→main deploy (docs/plan/2026-08-06-develop-to-main-merge.md).
--
-- The D1 edges unique indexes and the D4c inference.communities.props column are
-- already present on the live DB (verified) — not repeated here.

\set ON_ERROR_STOP on

-- node_kind += lib_symbol, lib_package (autocommit — ADD VALUE can't be used in the
-- same tx that adds it; we don't use them here, the daemon does later).
ALTER TYPE sensei.node_kind ADD VALUE IF NOT EXISTS 'lib_symbol';
ALTER TYPE sensei.node_kind ADD VALUE IF NOT EXISTS 'lib_package';

BEGIN;

-- nodes: fqn / resolved / language columns (additive).
ALTER TABLE sensei.nodes ADD COLUMN IF NOT EXISTS fqn        text;
ALTER TABLE sensei.nodes ADD COLUMN IF NOT EXISTS resolved   boolean NOT NULL DEFAULT false;
ALTER TABLE sensei.nodes ADD COLUMN IF NOT EXISTS language   text;

-- file_path nullable (reference stubs + lib_symbol nodes have no local file).
ALTER TABLE sensei.nodes ALTER COLUMN file_path DROP NOT NULL;

-- nodes_unique_identity: table CONSTRAINT → PARTIAL unique index (where file_path
-- is not null). Behaviour-preserving today; frees stub rows (file_path NULL) to be
-- governed solely by nodes_unique_fqn so two same-name different-fqn stubs stay distinct.
ALTER TABLE sensei.nodes DROP CONSTRAINT IF EXISTS nodes_unique_identity;
CREATE UNIQUE INDEX IF NOT EXISTS nodes_unique_identity
    ON sensei.nodes (folder_id, file_path, kind, name, parent_id, line_start)
    NULLS NOT DISTINCT
 WHERE file_path IS NOT NULL;

-- nodes_unique_fqn: partial unique index on the FQN moniker.
CREATE UNIQUE INDEX IF NOT EXISTS nodes_unique_fqn
    ON sensei.nodes (folder_id, fqn)
 WHERE fqn IS NOT NULL;

COMMIT;
