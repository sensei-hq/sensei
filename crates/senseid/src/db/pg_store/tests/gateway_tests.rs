use super::*;

// ── Gateway chains / role assignments tests ─────────────────────

/// list_chains_with_models returns every active chain, correctly
/// workspaces the `role` column, and JSON-aggregates member models in
/// sequence order. Seeded prod rows drive the shape assertions —
/// only a chain the test itself creates is deleted at teardown.
#[tokio::test]
async fn list_chains_with_models_returns_role_and_ordered_members() {
    let s = pg_store().await;
    let chains = s.list_chains_with_models().await.unwrap();
    assert!(!chains.is_empty(), "seed data should include at least one chain");

    // `reasoning` seeds to role=inference; `embed` to role=embedding.
    let reasoning = chains
        .iter()
        .find(|c| c["name"] == "reasoning")
        .expect("seed data should include reasoning chain");
    assert_eq!(reasoning["role"], "inference");

    let embed =
        chains.iter().find(|c| c["name"] == "embed").expect("seed data should include embed chain");
    assert_eq!(embed["role"], "embedding");

    // Utility chain — consensus-proposer — must NOT carry a role.
    let proposer = chains
        .iter()
        .find(|c| c["name"] == "consensus-proposer")
        .expect("seed data should include consensus-proposer");
    assert!(proposer["role"].is_null(), "utility chains stay unassigned");

    // Members are JSON-aggregated in `sequence_order`.
    let members = reasoning["models"].as_array().expect("models is an array");
    if members.len() >= 2 {
        let first = members[0]["sequenceOrder"].as_i64().unwrap();
        let second = members[1]["sequenceOrder"].as_i64().unwrap();
        assert!(first < second, "members are ordered by sequence_order asc");
    }
}

/// set_chain_role writes the role, clears it on None, and rejects a
/// role that another chain already owns (the unique-when-set index).
/// Runs against a scratch chain so seed rows stay intact.
#[tokio::test]
async fn set_chain_role_writes_clears_and_rejects_duplicate() {
    let s = pg_store().await;

    // Create a scratch chain with capability=reasoning so it can carry
    // a role at all.
    let scratch_name = format!("_test:chain_{}", uuid::Uuid::new_v4());
    let (scratch_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO gateway.fallback_chains (name, capability, description, is_active)
             VALUES ($1, 'reasoning'::sensei.model_capability, 'scratch', true)
             RETURNING id",
    )
    .bind(&scratch_name)
    .fetch_one(s.pool())
    .await
    .unwrap();

    // Write voice (unassigned by seed) → row now carries the role.
    s.set_chain_role(&scratch_id, Some("voice")).await.unwrap();
    let row: (Option<String>,) = sqlx_core::query_as::query_as(
        "SELECT role::text FROM gateway.fallback_chains WHERE id = $1",
    )
    .bind(scratch_id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(row.0.as_deref(), Some("voice"));

    // Clear → back to null.
    s.set_chain_role(&scratch_id, None).await.unwrap();
    let row: (Option<String>,) = sqlx_core::query_as::query_as(
        "SELECT role::text FROM gateway.fallback_chains WHERE id = $1",
    )
    .bind(scratch_id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(row.0.is_none());

    // Taking a role already owned by another chain (seed: reasoning ↔
    // inference) must fail — surfaces as a duplicate-key DB error the
    // caller can map to a 409 CONFLICT.
    let err = s
        .set_chain_role(&scratch_id, Some("inference"))
        .await
        .expect_err("unique index rejects a second inference chain");
    assert!(
        err.contains("duplicate") || err.contains("unique"),
        "expected uniqueness violation, got: {err}"
    );

    // Unknown chain id → not-found error, not a silent no-op.
    let ghost = uuid::Uuid::new_v4();
    let err = s.set_chain_role(&ghost, Some("voice")).await.expect_err("missing row must error");
    assert!(err.contains("not found"), "expected not-found error, got: {err}");

    // Teardown — remove the scratch chain.
    sqlx_core::query::query("DELETE FROM gateway.fallback_chains WHERE id = $1")
        .bind(scratch_id)
        .execute(s.pool())
        .await
        .unwrap();
}

/// End-to-end chain-model editing: add → move → remove → compact.
/// Runs against a scratch chain so seed rows stay intact.
#[tokio::test]
async fn chain_model_editing_add_move_remove_compacts_sequence() {
    let s = pg_store().await;

    // Scratch chain, capability=chat so any chat model matches.
    let scratch_name = format!("_test:mchain_{}", uuid::Uuid::new_v4());
    let (chain_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO gateway.fallback_chains (name, capability, description, is_active)
             VALUES ($1, 'chat'::sensei.model_capability, 'scratch', true)
             RETURNING id",
    )
    .bind(&scratch_name)
    .fetch_one(s.pool())
    .await
    .unwrap();

    // Pick three (model, router) pairs with capability=chat.
    let pairs: Vec<(uuid::Uuid, uuid::Uuid)> = sqlx_core::query_as::query_as(
        "SELECT m.id, mir.router_id
               FROM gateway.models m
               JOIN gateway.models_in_router mir ON mir.model_id = m.id
              WHERE m.capabilities @> ARRAY['chat'::sensei.model_capability]
              LIMIT 3",
    )
    .fetch_all(s.pool())
    .await
    .unwrap();
    assert!(
        pairs.len() >= 2,
        "test needs at least 2 chat-capable (model, router) pairs; got {}",
        pairs.len()
    );

    // Add — sequence_order starts at 1 and advances.
    let (row_a, seq_a) = s.add_chain_model(&chain_id, &pairs[0].0, &pairs[0].1).await.unwrap();
    let (row_b, seq_b) = s.add_chain_model(&chain_id, &pairs[1].0, &pairs[1].1).await.unwrap();
    assert_eq!(seq_a, 1);
    assert_eq!(seq_b, 2);

    // Move A down (swap with B).
    let moved = s.move_chain_model(&chain_id, &row_a, 1).await.unwrap();
    assert!(moved, "A should swap with B");
    let (seq_a_now,): (i32,) = sqlx_core::query_as::query_as(
        "SELECT sequence_order FROM gateway.fallback_chain_models WHERE id = $1",
    )
    .bind(row_a)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(seq_a_now, 2, "A now sits at position 2");
    let (seq_b_now,): (i32,) = sqlx_core::query_as::query_as(
        "SELECT sequence_order FROM gateway.fallback_chain_models WHERE id = $1",
    )
    .bind(row_b)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(seq_b_now, 1, "B moved into A's old slot");

    // Move B up past the top — boundary, no-op.
    let moved = s.move_chain_model(&chain_id, &row_b, -1).await.unwrap();
    assert!(!moved, "top boundary should return false");

    // Remove B — A should compact to sequence_order 1.
    s.remove_chain_model(&chain_id, &row_b).await.unwrap();
    let (seq_a_final,): (i32,) = sqlx_core::query_as::query_as(
        "SELECT sequence_order FROM gateway.fallback_chain_models WHERE id = $1",
    )
    .bind(row_a)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(seq_a_final, 1, "A compacts after B removal");

    // Not-found errors surface, not silent no-ops.
    let ghost = uuid::Uuid::new_v4();
    let err = s.remove_chain_model(&chain_id, &ghost).await.expect_err("remove missing must error");
    assert!(err.contains("not found"), "expected not-found, got: {err}");
    let err = s.move_chain_model(&chain_id, &ghost, 1).await.expect_err("move missing must error");
    assert!(err.contains("not found"), "expected not-found, got: {err}");

    // Available list: chain has 1 model, so all others with matching
    // capability are available (excludes the row we still have).
    let available = s.list_available_models_for_chain(&chain_id).await.unwrap();
    assert!(!available.is_empty(), "at least one chat model should be available after removing B");

    // Bad direction rejected with a clear message.
    let err = s.move_chain_model(&chain_id, &row_a, 2).await.expect_err("direction 2 must reject");
    assert!(err.contains("-1") || err.contains("+1"), "expected direction hint, got: {err}");

    // Teardown.
    sqlx_core::query::query("DELETE FROM gateway.fallback_chains WHERE id = $1")
        .bind(chain_id)
        .execute(s.pool())
        .await
        .unwrap();
}
