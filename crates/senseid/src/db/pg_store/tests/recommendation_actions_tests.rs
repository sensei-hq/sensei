//! L3 · ActionItems act (plan 2026-11-03 §L3.1): what the metric detail's action
//! items need from the store — a snooze that hides a pending recommendation until
//! it wakes, and the file a rework hotspot is about, so its action can open it.
use super::*;

async fn rec(
    s: &PgStore,
    ws: &uuid::Uuid,
    title: &str,
    action_type: &str,
    based_on: serde_json::Value,
) -> uuid::Uuid {
    s.create_recommendation_full(&crate::db::pg_store::RecommendationRow {
        workspace_id: ws,
        title,
        why: "why",
        impact: None,
        action_type,
        urgency: "medium",
        based_on: &based_on,
        reasoning_trace_id: None,
        prompt: None,
    })
    .await
    .unwrap()
}

async fn pending_ids(s: &PgStore, ws: &uuid::Uuid) -> Vec<String> {
    s.get_workspace_recommendations(ws, Some("pending"))
        .await
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect()
}

/// A detected pattern in `folder`, as the rework detector names it.
async fn pattern(
    s: &PgStore,
    ws: &uuid::Uuid,
    folder: Option<&uuid::Uuid>,
    name: &str,
) -> uuid::Uuid {
    let (id,): (uuid::Uuid,) = query_as(
        "INSERT INTO inference.detected_patterns(workspace_id, folder_id, name, is_anti_pattern)
         VALUES ($1, $2, $3, true) RETURNING id",
    )
    .bind(ws)
    .bind(folder)
    .bind(name)
    .fetch_one(s.pool())
    .await
    .unwrap();
    id
}

#[tokio::test]
#[ignore = "pending: actionitems"]
async fn a_snoozed_recommendation_leaves_the_pending_read_until_it_wakes() {
    let s = pg_store().await;
    let ws = s.create_workspace("_test:snooze_wakes", None, None).await.unwrap();
    let kept = rec(&s, &ws, "_test:kept", "cross_workspace", serde_json::json!({})).await;
    let snoozed = rec(&s, &ws, "_test:snoozed", "cross_workspace", serde_json::json!({})).await;

    let before = chrono::Utc::now();
    let until =
        s.snooze_recommendation(&ws, &snoozed, 7).await.unwrap().expect("a pending rec snoozes");
    let days = (until - before).num_hours();
    assert!((7 * 24 - 1..=7 * 24).contains(&days), "wakes 7 days out, got {days}h");

    assert_eq!(
        pending_ids(&s, &ws).await,
        vec![kept.to_string()],
        "a snoozed rec is not pending-shown"
    );
    // Still pending in status: the unfiltered read (Impact, history) keeps it.
    let all = s.get_workspace_recommendations(&ws, None).await.unwrap();
    let r = all.iter().find(|r| r["id"] == snoozed.to_string()).expect("unfiltered read keeps it");
    assert_eq!(r["status"], "pending");

    // Woken: the snooze has passed, so it is back.
    s.execute_raw(&format!(
        "UPDATE inference.recommendations SET snoozed_until = now() - interval '1 minute' WHERE id = '{snoozed}'"
    ))
    .await
    .unwrap();
    let mut back = pending_ids(&s, &ws).await;
    back.sort();
    let mut want = vec![kept.to_string(), snoozed.to_string()];
    want.sort();
    assert_eq!(back, want, "a woken rec is pending again");
    s.delete_workspace(&ws).await.unwrap();
}

#[tokio::test]
#[ignore = "pending: actionitems"]
async fn a_snooze_reaches_only_its_own_workspaces_pending_recommendation() {
    let s = pg_store().await;
    let ws = s.create_workspace("_test:snooze_scope_a", None, None).await.unwrap();
    let other = s.create_workspace("_test:snooze_scope_b", None, None).await.unwrap();
    let r = rec(&s, &ws, "_test:scoped", "cross_workspace", serde_json::json!({})).await;

    assert_eq!(
        s.snooze_recommendation(&other, &r, 7).await.unwrap(),
        None,
        "another workspace cannot snooze it"
    );
    assert_eq!(pending_ids(&s, &ws).await, vec![r.to_string()], "and it is untouched");

    s.reject_recommendation(&r).await.unwrap();
    assert_eq!(
        s.snooze_recommendation(&ws, &r, 7).await.unwrap(),
        None,
        "a decided rec does not snooze"
    );
    assert_eq!(
        s.snooze_recommendation(&ws, &uuid::Uuid::new_v4(), 7).await.unwrap(),
        None,
        "an unknown rec is a miss, not an error"
    );
    s.delete_workspace(&ws).await.unwrap();
    s.delete_workspace(&other).await.unwrap();
}

/// The rework detector names its pattern `rework: <file>` with the file as the
/// session saw it: repository-relative or absolute. The read resolves it to an
/// absolute path through the pattern's folder, so the action opens a real file.
#[tokio::test]
#[ignore = "pending: actionitems"]
async fn a_rework_recommendation_carries_its_file_resolved_through_its_pattern() {
    let s = pg_store().await;
    let ws = s.create_workspace("_test:rework_file", None, None).await.unwrap();
    let suffix = format!("rework_file_{}", uuid::Uuid::new_v4().simple());
    let folder = create_test_folder(&s, &suffix).await;

    let rel = pattern(&s, &ws, Some(&folder), "rework: src/api/server.rs").await;
    let abs = pattern(&s, &ws, Some(&folder), "rework: /elsewhere/lib.rs").await;
    let other = pattern(&s, &ws, Some(&folder), "correction-prone").await;
    let no_folder = pattern(&s, &ws, None, "rework: src/orphan.rs").await;

    let r_rel =
        rec(&s, &ws, "_test:rel", "audit_stale", serde_json::json!({ "patterns": [rel] })).await;
    let r_abs =
        rec(&s, &ws, "_test:abs", "audit_stale", serde_json::json!({ "patterns": [abs] })).await;
    let r_other =
        rec(&s, &ws, "_test:other", "write_skill", serde_json::json!({ "patterns": [other] }))
            .await;
    let r_none = rec(&s, &ws, "_test:none", "audit_stale", serde_json::json!({})).await;
    let r_gone = rec(
        &s,
        &ws,
        "_test:gone",
        "audit_stale",
        serde_json::json!({ "patterns": [uuid::Uuid::new_v4()] }),
    )
    .await;
    let r_nofolder = rec(
        &s,
        &ws,
        "_test:nofolder",
        "audit_stale",
        serde_json::json!({ "patterns": [no_folder] }),
    )
    .await;

    let recs = s.get_workspace_recommendations(&ws, Some("pending")).await.unwrap();
    let file = |id: &uuid::Uuid| {
        recs.iter().find(|r| r["id"] == id.to_string()).unwrap_or_else(|| panic!("{id} read"))["file"].clone()
    };
    assert_eq!(file(&r_rel), serde_json::json!(format!("/_test/{suffix}/src/api/server.rs")));
    assert_eq!(file(&r_abs), serde_json::json!("/elsewhere/lib.rs"));
    // Not about one file, no pattern, a pattern since replaced, or a relative file
    // with no folder to resolve it against: no file, never a guessed path.
    for id in [&r_other, &r_none, &r_gone, &r_nofolder] {
        assert_eq!(file(id), serde_json::Value::Null, "{id}");
    }
    s.delete_workspace(&ws).await.unwrap();
}
