set search_path to sensei, extensions;

-- The domain of `activity.task_executions.task_kind` — one value per
-- `TaskKind::to_string()` in `crates/senseid/src/tasks/mod.rs`.
--
-- WHY AN ENUM. The column was `text`, so a renamed or retired task kind
-- silently orphaned its history: rows kept the old string, nothing referenced
-- it, and no query or test noticed. Four such orphans had accumulated before
-- this type existed (see RETIRED below). As an enum, adding or renaming a kind
-- is a migration event — the insert fails loudly instead of writing a value
-- nothing will ever read again.
--
-- Named `task_execution_kind`, not `task_kind`, to stay clearly distinct from
-- `sensei.task_type_kind` (commit types: feat/fix/refactor/…), which is an
-- unrelated concept that would otherwise be one character away.
--
-- RETIRED VALUES are kept deliberately. They are the real domain of the column
-- for historical rows, and the alternative — remapping them to a current kind —
-- would fabricate history that never happened:
--   * build_connections  retired outright: its `covers` set is now the
--                        sensei.doc_coverage view, so the pass has no successor.
--   * resolve_edges      retired in 243e4fc5; FQN edges now resolve at emit, so
--                        the pass has no successor at all.
--   * plan_metric_days   retired in 8ec9384a with the day-planner.
--   * compute_metrics    SPLIT in 8ec9384a into compute_project_metrics +
--                        compute_group_metrics — a 1:many change, so no single
--                        successor exists to map it to.
--   * reconcile_identity renamed to reconcile_repo_metadata (it reads repo
--                        README frontmatter and never touched user identity;
--                        the name would have collided with the real identity
--                        work). Retained here because 1,208 rows carry it.
--   * backfill_transcripts / backfill_transcript_file renamed to
--                        ingest_captures / ingest_capture. The names became
--                        WRONG when the coordinator gained a `from` bound: it
--                        does ordinary parameterised ingestion, and calling that
--                        "backfill" implies a separate mode that no longer
--                        exists.
-- Retention eventually prunes these rows; the values stay so the type remains a
-- truthful description of what the column has held.
create type task_execution_kind as enum (
  -- ── index pipeline ──
  'scan_root'
, 'process_git_folder'
, 'process_folder'
, 'process_file'
  -- One per package manifest in a repo. Resolves the PLACEMENT (package +
  -- module) that `index_file` needs before it can mint an fqn. Distinct from
  -- `extract_deps`, which is repo-wide and feeds the library index.
, 'process_manifest'
  -- The manifest→files gate. Blocked on every `process_manifest` of its repo
  -- via the queue's ordinary dependency barrier; when they drain it fans out
  -- one `process_file` per supported file.
, 'process_repo_files'
, 'delete_file'
, 'delete_folder'
, 'branch_switch'
, 'extract_deps'
, 'embed_nodes'
, 'detect_communities'
  -- ── library pipeline ──
, 'resolve_libs'
, 'import_lib'
, 'index_library'
, 'index_library_page'
  -- ── activity pipeline ──
, 'ingest_captures'
, 'ingest_capture'
, 'analyze_project'
, 'analyze_session_process'
, 'reconcile_repo_metadata'
  -- ── metrics pipeline ──
, 'compute_project_metrics'
, 'compute_group_metrics'
, 'compute_health'
, 'backfill_coverage'
  -- ── inference pipeline ──
, 'measure_verdicts'
, 'classify_pending_verdicts'
, 'aggregate_corrections'
, 'aggregate_tool_insights'
, 'consolidate_governance'
, 'warm_narration_cache'
, 'learn_playbooks'
, 'scan_doc_drift'
, 'publish_relay_segments'
, 'advance_run'
, 'publish_run'
  -- ── RETIRED (see header) ──
, 'build_connections'
, 'resolve_edges'
, 'plan_metric_days'
, 'compute_metrics'
, 'reconcile_identity'
, 'backfill_transcripts'
, 'backfill_transcript_file'
);
