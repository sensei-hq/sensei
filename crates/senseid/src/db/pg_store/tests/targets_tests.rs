//! TG1 · gating eligibility (docs/spec/dojo/targets-and-gates.md §TG1), read from
//! the seeded `sensei.metric_gating` (`import/staging/metric_gating.jsonl`).
use super::*;

#[tokio::test]
async fn gating_reads_eligible_metrics_with_their_comparator() {
    let s = pg_store().await;
    let rows = s.metric_gating().await.unwrap();
    let coverage = rows.iter().find(|r| r.metric_key == "coverage").expect("coverage is gated");
    assert!(coverage.eligible, "coverage is target 2");
    assert!(!coverage.delta_scoped, "an absolute, ratchetable reading");
    assert_eq!(coverage.min_coverage, 0.5);
    assert_eq!(
        coverage.comparator.as_deref(),
        Some(">="),
        "from metrics.direction = higher_better"
    );

    let ftr = rows.iter().find(|r| r.metric_key == "ftr").expect("ftr has a row saying why not");
    assert!(!ftr.eligible, "an insight, never a target");
    assert_eq!(ftr.unit.as_deref(), Some("%"), "the unit is the catalogue's");
}

/// The comparator follows the catalogue's direction, not a copy of it: a
/// lower-is-better metric reads `<=`, and a neutral one has none.
#[tokio::test]
async fn the_comparator_is_derived_from_the_metrics_direction() {
    let s = pg_store().await;
    s.execute_raw(
        "INSERT INTO sensei.metric_gating(metric_id, eligible, delta_scoped, min_coverage, note)
         SELECT id, direction <> 'neutral', false, 1.0, '_test' FROM sensei.metrics
          WHERE key IN ('churn_concentration')
             OR id = (SELECT id FROM sensei.metrics WHERE direction = 'lower_better' ORDER BY key LIMIT 1)
         ON CONFLICT (metric_id) DO NOTHING",
    )
    .await
    .unwrap();
    let (lower,): (String,) = query_as(
        "SELECT key FROM sensei.metrics WHERE direction = 'lower_better' ORDER BY key LIMIT 1",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    let rows = s.metric_gating().await.unwrap();
    let of =
        |k: &str| rows.iter().find(|r| r.metric_key == k).unwrap_or_else(|| panic!("{k} gated"));
    assert_eq!(of(&lower).comparator.as_deref(), Some("<="), "{lower} is lower_better");
    assert_eq!(of("churn_concentration").comparator, None, "neutral: no comparator");
    assert!(!of("churn_concentration").eligible);
}

/// A metric with no gating row is not eligible, and is not given a default row.
#[tokio::test]
async fn an_absent_gating_row_is_not_eligible() {
    let s = pg_store().await;
    let (ungated,): (String,) = query_as(
        "SELECT m.key FROM sensei.metrics m
          WHERE NOT EXISTS (SELECT 1 FROM sensei.metric_gating g WHERE g.metric_id = m.id)
          ORDER BY m.key LIMIT 1",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    let rows = s.metric_gating().await.unwrap();
    assert!(!rows.is_empty(), "the seed is read");
    assert!(
        rows.iter().all(|r| r.metric_key != ungated),
        "{ungated} has no gating row, so none is invented for it"
    );
}

/// D83 (review r1 M1): a gating read that fails is the caller's error, never an
/// empty list — TG4 reads this list as "which targets exist", so an empty one on
/// a failure would read as "no target can miss". The read is first shown to work,
/// so this cannot pass on a read that always fails.
#[tokio::test]
async fn a_failed_gating_read_is_an_error_not_an_empty_list() {
    let s = pg_store().await;
    assert!(!s.metric_gating().await.unwrap().is_empty(), "the seeded read works first");
    // This test's own clone: the table is gone, so the read cannot be made.
    s.execute_raw("DROP TABLE sensei.metric_gating CASCADE").await.unwrap();
    let read = s.metric_gating().await;
    assert!(read.is_err(), "a failed read is an error, got {read:?}");
}
