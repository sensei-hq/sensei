# Inference Tasks Design

> 6 new TaskKinds for intelligence, memory lifecycle, and collective insights.

## Task Kinds

### AnalyzePatterns
- **Trigger:** After BuildConnections completes
- **Input:** folder_id (from task.repo_id lookup)
- **Reads:** sensei.nodes, sensei.edges (call patterns, structural patterns)
- **Writes:** inference.detected_patterns (upsert with instance_count, confidence)
- **Logic:**
  1. Query nodes grouped by kind — identify repeated structures (adapter pattern, factory, etc.)
  2. Query edges — find god-nodes (high in-degree), orphan nodes (zero edges), cycles
  3. For each pattern found, upsert into detected_patterns with lifecycle=suggested
  4. Anti-patterns: flag duplicated code (nodes with identical signatures across files), god-nodes (degree > 20)

### DetectCommunities
- **Trigger:** After BuildConnections completes (parallel with AnalyzePatterns)
- **Input:** folder_id
- **Reads:** sensei.edges (calls, imports)
- **Writes:** inference.communities, sensei.nodes.community_id
- **Logic:**
  1. Build adjacency list from edges (calls=weight 3, imports=weight 2)
  2. Run Leiden-inspired clustering (iterative local moves maximizing modularity)
  3. Upsert community records with node_count, label (derived from dominant file path)
  4. Update nodes with community_id assignment

### EvaluateSession
- **Trigger:** On session complete (checkpoint event with outcome)
- **Input:** session_id, folder_id
- **Reads:** activity.sessions, activity.events (corrections, tool_calls), sensei.memories
- **Writes:** sensei.memories, sensei.memory_evidence, inference.recommendations
- **Logic:**
  1. Load session events — count corrections, identify patterns
  2. For each correction event:
     a. Check if existing memory matches (fuzzy match on content)
     b. If match: reinforce_memory(+1.0), increment reinforced_count, status→reinforced
     c. If no match + 2nd occurrence of same correction: create new memory
  3. Check for violated memories — if assistant acted contrary to a known memory:
     a. Increment violated_count, set status→challenged, update last_relevant_at
  4. If session.ftr == false and corrections >= 2: create recommendation (urgency based on pattern frequency)
  5. Strength lifecycle:
     - reinforced_count >= 3 && violated_count == 0 → status=battle_tested
     - strength < 1.0 → status=archived
     - After 30 days without last_relevant_at update → decay strength by 0.5

### RunMOE
- **Trigger:** FTR drop >10% over 5 sessions, recurring correction (3+ same issue), pattern emerging (5+ instances)
- **Input:** trigger_event, trigger_detail (jsonb with context)
- **Reads:** gateway.routers, gateway.models (available models), context from memories + sessions
- **Writes:** inference.reasoning_traces, inference.recommendations
- **Logic:**
  1. Check gateway for available models (routers with is_active=true, at least 2 models)
  2. If no models available: skip, log warning
  3. Construct debate prompt:
     - System context: relevant memories, recent session corrections, detected patterns
     - Question: "Given {trigger_event}, what should change?"
  4. Round 1 — Proposer (model A): generate proposed change
  5. Round 2 — Challenger (model B): critique the proposal
  6. Round 3 — Synthesizer (model A or C): reconcile into recommendation
  7. Store full exchange in reasoning_traces (models_used, exchanges jsonb, consensus jsonb)
  8. If consensus.confidence >= 0.7: create recommendation linked to trace
  9. Recommendation action_types: promote_pattern, add_skill, modify_rule, create_memory

### SyncInsights
- **Trigger:** Periodic (daily) or when pending_insights > threshold (e.g., 10)
- **Input:** none (processes all unsent insights)
- **Reads:** inference.recommendations (verdict=positive), inference.detected_patterns (lifecycle=rule), sensei.memories (strength >= 3)
- **Writes:** inference.insight_batches, inference.insights
- **Logic:**
  1. Query battle-tested data: recommendations with positive verdicts, promoted patterns, strong memories
  2. Anonymize: strip project names, file paths, personal identifiers
  3. Keep: pattern names, confidence scores, FTR impact deltas, action types
  4. Create insight_batch with count + target (e.g., "collective-api")
  5. Insert individual insights with source_table, source_id, category, anonymized payload
  6. POST batch to collective API endpoint (configurable, can be disabled)
  7. Mark batch as sent_at

### PullInsights
- **Trigger:** Periodic (daily) or on user request
- **Input:** none (fetches from collective API)
- **Reads:** remote collective API
- **Writes:** sensei.memories (scope=global, type=convention), inference.detected_patterns
- **Logic:**
  1. GET from collective API: insights since last pull timestamp
  2. For each insight:
     a. Check category relevance (match against local project stacks)
     b. If relevant: create memory with scope=global, strength=0.5, type=convention
     c. For pattern insights: upsert into detected_patterns with lifecycle=suggested
  3. Store pull timestamp in config ("last_collective_pull")

## Pipeline Integration

```
[existing]  scan → process → resolve → build_connections
                                              ↓
              ┌─────────────────────────────────┐
              │  AnalyzePatterns (post-index)    │  parallel
              │  DetectCommunities (post-index)  │
              └─────────────────────────────────┘
                                              ↓
            EvaluateSession (after each session ends)
                ↓ (if trigger conditions met)
            RunMOE (multi-model debate → recommendation)
                ↓ (periodically)
            SyncInsights → collective API → PullInsights
```

## PgStore Methods Needed

Already exist: create_memory, reinforce_memory, archive_memory, list_active_memories, add_memory_evidence, create_recommendation, insert_reasoning_trace, upsert_pattern, upsert_community, list_patterns_by_folder.

New methods needed:
- `update_memory_status(id, status, reinforced_count, violated_count)` — for lifecycle transitions
- `find_similar_memory(project_id, content_fragment)` — fuzzy search for dedup
- `list_battle_tested_insights(min_strength)` — for SyncInsights
- `create_insight(batch_id, source_table, source_id, category, payload)` — for SyncInsights
- `create_insight_batch(count, target)` — for SyncInsights
- `get_config("last_collective_pull")` — already exists

## Task Handler Files

```
crates/senseid/src/tasks/handlers/
├── scan.rs           (existing)
├── process.rs        (existing)
├── resolve.rs        (existing)
├── libraries.rs      (existing)
├── inference.rs      (NEW — AnalyzePatterns, DetectCommunities)
├── memory.rs         (NEW — EvaluateSession)
├── moe.rs            (NEW — RunMOE)
├── collective.rs     (NEW — SyncInsights, PullInsights)
```

## Executor Wiring

Add to TaskKind enum and execute_task match:
```rust
TaskKind::AnalyzePatterns => handlers::analyze_patterns(ctx, task).await,
TaskKind::DetectCommunities => handlers::detect_communities(ctx, task).await,
TaskKind::EvaluateSession => handlers::evaluate_session(ctx, task).await,
TaskKind::RunMOE => handlers::run_moe(ctx, task).await,
TaskKind::SyncInsights => handlers::sync_insights(ctx, task).await,
TaskKind::PullInsights => handlers::pull_insights(ctx, task).await,
```

## Memory Status Lifecycle

```
active (new, strength=1.0)
  → reinforced (reinforced_count >= 1, strength >= 2.0)
    → battle_tested (reinforced_count >= 3, violated_count == 0, strength >= 3.0)
  → challenged (violated_count >= 1)
    → active (if violation resolved)
  → archived (strength < 1.0 after decay)
```
