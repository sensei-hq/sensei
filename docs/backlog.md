# Backlog

> Granular, TDD-ready issues. Each can be worked unattended with clear inputs/outputs.
>
> **Every entity implementation must follow TDD (red/green/refactor) and include full coverage tests.** Write tests first, verify they fail, then implement. No entity is complete without tests covering: happy path, edge cases, error handling, and cascade behavior (deletes, FKs).

## Design gaps

| # | Title | Description | Priority |
|---|-------|-------------|----------|
| D1 | Session continuity journey | Idea 11 has no journey. Crash recovery, interrupted work resume, snapshot→continuity memory flow need an end-to-end user story. | HIGH |
| D2 | Collective intelligence journey | Idea 29 has no journey. How insights are batched, anonymized, shared, and what the user sees. Needs system pipeline doc too. | MEDIUM |
| D3 | Drift detection design | When files change, compare with associated documents and traceability matrix. Detect gaps between code and docs. Use inference gateway (idea 28) to analyze via multiple LLM providers/models, consolidate findings in structured format for visualization. Needs: trigger mechanism (on file change via watcher), scan strategy (which docs relate to which code), analysis pipeline (gateway → multi-model → structured output), gap visualization (observatory). Related: idea 13 (doc traceability), idea 28 (inference gateway). | HIGH |

## Database migration — SQLite+Kuzu → PostgreSQL

Replace `rusqlite` with `sqlx::PgPool` in `crates/senseid/src/db/`. 97 rusqlite references across 12 entity files.

### Phase 1: Connection pool + existing entities

| # | Title | Description | Test approach |
|---|-------|-------------|---------------|
| 1 | Migrate connection pool to PostgreSQL | Replace `Store { conn: rusqlite::Connection }` with `Store { pool: sqlx::PgPool }`. Update `open_memory()` test helper to use PG test DB. | Integration: connect, health check, verify schema exists |
| 2 | Migrate config CRUD | `config.rs` — get_config, set_config, delete_config | Test: set key, get key, delete key, get missing returns None |
| 3 | Migrate tags CRUD | `tags.rs` — add_tag, remove_tag, get_tags | Test: add tags, query by entity, remove, verify isolation by entity_type |
| 4 | Migrate repos CRUD | `repos.rs` — upsert_repo, upsert_repo_basic, get_repo, list_repos, mark_indexed, is_excluded, set_repo_project. Largest entity (15 rusqlite refs). | Test: create repo, update, list with tags batch-loaded, verify project assignment |
| 5 | Migrate projects CRUD | `projects.rs` — create_project, list_projects, get_project_repos | Test: create project, assign repos, list with repo counts |
| 6 | Migrate sessions CRUD | `sessions.rs` — create_session, complete_session, get_session, list_by_project | Test: create session, complete with ftr, list by project |
| 7 | Migrate events CRUD | `events.rs` — insert_event, get_by_session, get_by_type | Test: insert correction + tool_call events, query by type |
| 8 | Migrate excluded_paths CRUD | `excluded_paths.rs` — add, remove, is_excluded, list | Test: add exclusion, verify is_excluded, remove, verify gone |
| 9 | Migrate workflow_state CRUD | `workflow_state.rs` — get, set, delete | Test: set state, get state, verify jsonb round-trip |
| 10 | Migrate index_errors CRUD | `index_errors.rs` — insert, list_by_folder, clear_by_folder | Test: insert errors, list, clear, verify empty |
| 11 | Migrate detected_patterns CRUD | `detected_patterns.rs` — upsert, promote, list_by_folder, get_by_lifecycle. Largest entity (29 rusqlite refs). | Test: create suggested pattern, promote to rule, verify lifecycle transition |
| 12 | Migrate lib_docs CRUD | `lib_docs.rs` — upsert_library, add_section, search_sections | Test: create library, add sections, search by query |

### Phase 2: New entities (PG tables with no Rust entity yet)

| # | Title | Description | Test approach |
|---|-------|-------------|---------------|
| 13 | Add folders_to_watch CRUD | create, list, update_status, update_excluded | Test: create root, verify scanning→watching lifecycle |
| 14 | Add folders CRUD | upsert, list_by_root, list_by_project, delete_tree | Test: create parent→git hierarchy, verify cascade delete |
| 15 | Add nodes CRUD | upsert, get_by_folder, get_by_file, get_children, delete_by_folder | Test: create file→class→method tree, query by file_path |
| 16 | Add edges CRUD | insert, get_callers, get_callees, get_by_kind | Test: create nodes + call edges, verify bidirectional lookup |
| 17 | Add extensions CRUD + verify historize | create, update, get, list_by_kind, get_history (from past_extensions) | Test: create skill, update, verify past_extensions snapshot via trigger |
| 18 | Add libraries CRUD | upsert_library, add_page, link_to_folder, query libraries_in_project view | Test: create library, add pages, link to folder, verify view |
| 19 | Add scan_state CRUD | upsert, get_stale_files, delete_by_folder | Test: insert scan state, check mtime comparison, verify stale detection |
| 20 | Add memories CRUD | create, update, archive, list_active, get_by_scope, search | Test: create memory, reinforce (strength +1), archive (strength < 1), query by scope+filter |
| 21 | Add memory_examples CRUD | add_example, remove, list_by_memory | Test: add good+bad examples, list, remove, verify cascade on memory delete |
| 22 | Add memory_evidence CRUD | add_evidence, list_by_memory | Test: add session evidence, list, verify cascade |
| 23 | Add memory_links CRUD | link_parent_child, get_children, get_parent | Test: create parent+child link, query both directions, verify cascade |
| 24 | Add snapshots CRUD (activity) | create, get_latest, list_by_session | Test: create snapshot with progress_summary, get latest, verify fields |
| 25 | Add recommendations CRUD (inference) | create, accept, measure, list_by_project | Test: create recommendation, accept (baseline_ftr set), measure (verdict), sort by urgency |
| 26 | Add communities CRUD (inference) | upsert, list_by_folder | Test: create with node_ids, verify node_count |
| 27 | Add reasoning_traces CRUD (inference) | insert, get_by_recommendation | Test: insert trace with model exchanges, query by recommendation |
| 28 | Add services CRUD | upsert, list, get_by_kind | Test: create MCP service, list by kind, verify config jsonb |
| 29 | Add benchmark_reports + benchmark_runs CRUD | create_report, add_run, list_by_project | Test: create report, add runs, verify aggregation |

### Phase 2b: Read-only view entities

| # | Title | Description | Test approach |
|---|-------|-------------|---------------|
| V1 | Expose repositories view | `sensei.repositories` — git+subtree folders with parent info | Test: create folders, query view, verify git repos returned |
| V2 | Expose parent_folders view | `sensei.parent_folders` — folder hierarchy with parent path | Test: create nested folders, query, verify parent chain |
| V3 | Expose libraries_in_project view | `sensei.libraries_in_project` — libraries joined with folder/project | Test: create library + link to folder, query view, verify join |

### Phase 3: PG functions + search

| # | Title | Description | Test approach |
|---|-------|-------------|---------------|
| 30 | Wrap match_embeddings function | Rust caller for `sensei.match_embeddings(query_vector, project, limit)` | Test: insert nodes with vectors, call, verify cosine ranking |
| 31 | Wrap rank_bm25 function | Rust caller for `sensei.rank_bm25(query, project, limit)` | Test: insert nodes with text, search, verify ranked results |
| 32 | Wrap rank_bfs function | Rust caller for `sensei.rank_bfs(changed_files, project, depth)` | Test: insert import edges, query changed files, verify BFS traversal |

### Phase 4: Task queue + processors

| # | Title | Description | Test approach |
|---|-------|-------------|---------------|
| 33 | Update task queue to folder_id | Task struct uses folder_id, reads from folders table | Test: enqueue for folder, dequeue, verify folder_id |
| 34 | Update code processor to write nodes | Write file + symbol nodes via sqlx, with parent_id | Test: process Rust file, verify node hierarchy in PG |
| 35 | Update code processor to write edges | Write call + import edges via sqlx, with confidence | Test: process file with calls, verify edges in PG |
| 36 | Add community detection post-processor | Leiden on graph from PG, write community_id back | Test: small graph, run detection, verify assignments |

### Phase 5: Watcher + API + MCP

| # | Title | Description | Test approach |
|---|-------|-------------|---------------|
| 37 | Update watcher to read folders_to_watch | Use excluded jsonb from PG for filtering | Test: create root + exclusions, verify skip |
| 38 | Update API list endpoint to use folders/repositories view | Return git+subtree folders from PG view | Test: create folders, call endpoint, verify shape |
| 39 | Update session API endpoints for activity schema | Read activity.sessions + events from PG | Test: create session + events, verify timeline |
| 40 | Add recommendations API endpoint | GET /api/recommendations?project_id | Test: create recommendations, verify urgency sort |
| 41 | Add memories API endpoints | GET /api/memories?scope&project_id, POST /api/memories, PUT strength/status | Test: CRUD lifecycle, scope filtering |
| 42 | Update MCP search() to query nodes | FTS or name/signature match on PG nodes | Test: insert nodes, search, verify results |
| 43 | Update MCP get_callers/get_callees to query edges | edges where kind=calls from PG | Test: insert call edges, query, verify |
| 44 | Update MCP get_patterns() for inference schema | Query inference.detected_patterns from PG | Test: insert patterns, verify lifecycle filter |
| 45 | Add MCP report_learning() tool | New MCP tool → creates memory via PG | Test: call with structured input, verify stored |
| 46 | Update MCP get_session_context() for memory assembly | Multi-source context from PG: memories by scope, recent sessions, patterns | Test: create memories at different scopes, verify assembled markdown |
