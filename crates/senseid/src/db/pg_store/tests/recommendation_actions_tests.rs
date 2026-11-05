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
/// absolute path through the pattern's folder, so the action opens a real file —
/// and only a file that is still there (live, 2026-11-03: 118 of 297 hotspot
/// files had since been moved or deleted).
#[tokio::test]
async fn a_rework_recommendation_carries_its_file_resolved_through_its_pattern() {
    let s = pg_store().await;
    let ws = s.create_workspace("_test:rework_file", None, None).await.unwrap();
    let suffix = format!("rework_file_{}", uuid::Uuid::new_v4().simple());
    let folder = create_test_folder(&s, &suffix).await;
    // The folder is a real directory holding the hotspot's file.
    let repo = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(repo.path().join("src/api")).unwrap();
    std::fs::write(repo.path().join("src/api/server.rs"), "fn main() {}").unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let abs_file = elsewhere.path().join("lib.rs");
    std::fs::write(&abs_file, "").unwrap();
    let root = repo.path().to_string_lossy().to_string();
    query_as::<_, (uuid::Uuid,)>(
        "UPDATE sensei.folders SET abs_path = $2 WHERE id = $1 RETURNING id",
    )
    .bind(folder)
    .bind(&root)
    .fetch_one(s.pool())
    .await
    .unwrap();

    let rel = pattern(&s, &ws, Some(&folder), "rework: src/api/server.rs").await;
    let abs = pattern(&s, &ws, Some(&folder), &format!("rework: {}", abs_file.display())).await;
    let deleted = pattern(&s, &ws, Some(&folder), "rework: src/api/removed.rs").await;
    let other = pattern(&s, &ws, Some(&folder), "correction-prone").await;
    let no_folder = pattern(&s, &ws, None, "rework: src/orphan.rs").await;
    let escapes = pattern(&s, &ws, Some(&folder), "rework: ../../etc/passwd").await;

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
    let r_escapes =
        rec(&s, &ws, "_test:escapes", "audit_stale", serde_json::json!({ "patterns": [escapes] }))
            .await;
    let r_deleted =
        rec(&s, &ws, "_test:deleted", "audit_stale", serde_json::json!({ "patterns": [deleted] }))
            .await;

    let recs = s.get_workspace_recommendations(&ws, Some("pending")).await.unwrap();
    let file = |id: &uuid::Uuid| {
        recs.iter().find(|r| r["id"] == id.to_string()).unwrap_or_else(|| panic!("{id} read"))["file"].clone()
    };
    assert_eq!(file(&r_rel), serde_json::json!(format!("{root}/src/api/server.rs")));
    // An absolute file outside the repository is not offered (actionitems-lows LOW-1).
    assert_eq!(file(&r_abs), serde_json::Value::Null, "an absolute file elsewhere: not offered");
    // Not about one file, no pattern, a pattern since replaced, a relative file
    // with no folder to resolve it against, one that climbs out of the
    // repository with `..`, or one no longer on disk: no file, never a guessed
    // path (the item is snoozed instead of offering a button that fails).
    for id in [&r_other, &r_none, &r_gone, &r_nofolder, &r_escapes, &r_deleted] {
        assert_eq!(file(id), serde_json::Value::Null, "{id}");
    }
    s.delete_workspace(&ws).await.unwrap();
}

/// A re-snooze replaces the wake time; it never keeps the later of the two.
#[tokio::test]
async fn a_re_snooze_replaces_the_wake_time() {
    let s = pg_store().await;
    let ws = s.create_workspace("_test:resnooze", None, None).await.unwrap();
    let r = rec(&s, &ws, "_test:resnooze", "cross_workspace", serde_json::json!({})).await;
    let week = s.snooze_recommendation(&ws, &r, 7).await.unwrap().expect("snoozed");
    let two = s.snooze_recommendation(&ws, &r, 2).await.unwrap().expect("re-snoozed");
    assert!(two < week, "the second snooze replaces the first: {two} vs {week}");
    let hours = (two - chrono::Utc::now()).num_hours();
    assert!((47..=48).contains(&hours), "wakes 2 days out, got {hours}h");
    assert!(pending_ids(&s, &ws).await.is_empty(), "still hidden");
    s.delete_workspace(&ws).await.unwrap();
}

/// The focal "do first" pick (Overview, the ranking pass) never lands on a rec the
/// pending read hides: a snooze clears the rec's focal flag at once, and the
/// ranking pass reads only recs that are awake.
#[tokio::test]
async fn a_snoozed_rec_is_never_the_focal_pick() {
    let s = pg_store().await;
    let ws = s.create_workspace("_test:snooze_focal", None, None).await.unwrap();
    let top = rec(&s, &ws, "_test:top", "revise_rule", serde_json::json!({})).await;
    let next = rec(&s, &ws, "_test:next", "cross_workspace", serde_json::json!({})).await;
    s.set_recommendation_rank(&top, 90.0, true, &serde_json::json!({})).await.unwrap();
    s.set_recommendation_rank(&next, 40.0, false, &serde_json::json!({})).await.unwrap();

    s.snooze_recommendation(&ws, &top, 7).await.unwrap().expect("snoozed");
    let (focal,): (bool,) = query_as("SELECT focal FROM inference.recommendations WHERE id = $1")
        .bind(top)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert!(!focal, "a snoozed rec stops being the focal pick at once");

    let ranked: Vec<uuid::Uuid> =
        s.get_pending_recs_for_ranking(&ws).await.unwrap().into_iter().map(|r| r.0).collect();
    assert_eq!(ranked, vec![next], "the ranking pass never sees a snoozed rec");
    s.delete_workspace(&ws).await.unwrap();
}

/// A repository folder on disk (`<repo>/inside.rs`), a directory outside it
/// (`<elsewhere>/outside.rs`), and `<repo>/link → <elsewhere>`: the fixture the
/// confinement tests share. The tempdirs live while the returned guards do.
async fn confinement_fixture(
    s: &PgStore,
    tag: &str,
) -> (uuid::Uuid, uuid::Uuid, tempfile::TempDir, tempfile::TempDir) {
    let ws = s.create_workspace(&format!("_test:{tag}"), None, None).await.unwrap();
    let folder = create_test_folder(s, &format!("{tag}_{}", uuid::Uuid::new_v4().simple())).await;
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join("inside.rs"), "").unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::write(elsewhere.path().join("outside.rs"), "").unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), repo.path().join("link")).unwrap();
    query_as::<_, (uuid::Uuid,)>(
        "UPDATE sensei.folders SET abs_path = $2 WHERE id = $1 RETURNING id",
    )
    .bind(folder)
    .bind(repo.path().to_string_lossy().to_string())
    .fetch_one(s.pool())
    .await
    .unwrap();
    (ws, folder, repo, elsewhere)
}

/// The `file` the pending read offers for a rework pattern named `rework: <name>`.
async fn offered_file(
    s: &PgStore,
    ws: &uuid::Uuid,
    folder: &uuid::Uuid,
    name: &str,
) -> serde_json::Value {
    let p = pattern(s, ws, Some(folder), &format!("rework: {name}")).await;
    let r =
        rec(s, ws, &format!("_test:{name}"), "audit_stale", serde_json::json!({ "patterns": [p] }))
            .await;
    let recs = s.get_workspace_recommendations(ws, Some("pending")).await.unwrap();
    recs.iter().find(|x| x["id"] == r.to_string()).unwrap()["file"].clone()
}

/// ActionItems review r1 LOW-1: a hotspot file outside the repository (the
/// session edited `~/.zshrc`, or another repository's file) is not offered, even
/// though it exists; one inside the repository by its absolute path is, as
/// written (not its canonical `/private/var/…` form on macOS).
#[tokio::test]
async fn a_rework_file_outside_its_repository_is_not_offered() {
    let s = pg_store().await;
    let (ws, folder, repo, elsewhere) = confinement_fixture(&s, "rework_outside").await;
    let inside = repo.path().join("inside.rs").to_string_lossy().to_string();
    let outside = elsewhere.path().join("outside.rs").to_string_lossy().to_string();
    assert_eq!(offered_file(&s, &ws, &folder, &inside).await, serde_json::json!(inside));
    assert_eq!(
        offered_file(&s, &ws, &folder, &outside).await,
        serde_json::Value::Null,
        "outside the repository: not offered"
    );
    s.delete_workspace(&ws).await.unwrap();
}

/// A symlink inside the repository that points out of it does not make its
/// target inside (lanes review r1 LOW-1, L0: canonicalise BOTH the file and the
/// root; on macOS the tempdirs live under `/var` → `/private/var`, so
/// canonicalising one side only would also fail the inside case above).
#[tokio::test]
async fn an_absolute_path_through_an_in_repository_symlink_out_is_not_offered() {
    let s = pg_store().await;
    let (ws, folder, repo, _elsewhere) = confinement_fixture(&s, "rework_link_abs").await;
    let via_link = repo.path().join("link").join("outside.rs").to_string_lossy().to_string();
    assert_eq!(
        offered_file(&s, &ws, &folder, &via_link).await,
        serde_json::Value::Null,
        "an absolute path through a symlink out of the repository: not offered"
    );
    s.delete_workspace(&ws).await.unwrap();
}

/// The same escape written relative to the repository (lanes review r2 F1, L0:
/// required) — the common shape, and lexically inside, so a confinement check
/// made only on absolute paths would offer it.
#[tokio::test]
async fn a_relative_path_through_an_in_repository_symlink_out_is_not_offered() {
    let s = pg_store().await;
    let (ws, folder, _repo, _elsewhere) = confinement_fixture(&s, "rework_link_rel").await;
    assert_eq!(
        offered_file(&s, &ws, &folder, "link/outside.rs").await,
        serde_json::Value::Null,
        "a relative path through a symlink out of the repository: not offered"
    );
    s.delete_workspace(&ws).await.unwrap();
}
