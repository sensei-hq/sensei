//! The metric registry seed (`database/import/staging/metrics.jsonl`) against what
//! the metrics actually measure, and the catalog against the registry. Pure: it
//! reads the files, so no database is needed.

use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn registry() -> Vec<serde_json::Value> {
    let text = std::fs::read_to_string(repo_root().join("database/import/staging/metrics.jsonl"))
        .expect("metrics.jsonl");
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn metric(key: &str) -> serde_json::Value {
    registry()
        .into_iter()
        .find(|m| m["key"] == key)
        .unwrap_or_else(|| panic!("{key} is not registered"))
}

fn text(m: &serde_json::Value, field: &str) -> String {
    m[field].as_str().unwrap_or_default().to_string()
}

/// The rating rule `sensei.metric_rating_facts` applies: how many thresholds the
/// value reaches — `threshold >= value` for lower_better, `<=` for higher_better.
fn rating(m: &serde_json::Value, value: f64) -> usize {
    let scale: Vec<f64> =
        m["rating_scale"].as_array().unwrap().iter().map(|t| t.as_f64().unwrap()).collect();
    match m["direction"].as_str().unwrap() {
        "lower_better" => scale.iter().filter(|t| **t >= value).count(),
        _ => scale.iter().filter(|t| **t <= value).count(),
    }
}

/// #154: the value is stored in SECONDS, but the scale read as minutes, so any
/// session slower than a minute scored 0 — 87% of readings.
#[test]
fn time_to_useful_result_is_rated_on_a_seconds_scale() {
    let m = metric("time_to_useful_result");
    assert_eq!(text(&m, "unit"), "seconds");
    assert_eq!(rating(&m, 3.0 * 60.0), 5, "3 minutes to a usable result is excellent");
    assert!(rating(&m, 10.0 * 60.0) >= 2, "10 minutes still scores");
    assert_eq!(rating(&m, 2.0 * 3600.0), 0, "2 hours scores nothing");
}

/// #153: churn_rate is counted from git, but its registry text described the
/// retired indexing feed (`process_file task_executions`).
#[test]
fn churn_rate_describes_the_git_source_it_is_computed_from() {
    let m = metric("churn_rate");
    assert_eq!(text(&m, "unit"), "files/day");
    for field in ["formula", "description", "how_to_read"] {
        let t = text(&m, field);
        assert!(!t.contains("task_executions") && !t.contains("process_file"), "{field}: {t}");
        assert!(!t.contains("rescan"), "{field} still discounts a rescan bug git cannot have: {t}");
    }
    assert!(text(&m, "formula").contains("git"), "the formula names its source");
}

/// #177: the fresh tokens are the ones that track spend, and they have a metric.
/// 2026-10-26 (user): cache reads are counted on their own, never as tokens —
/// summed in, they were ~all of tokens_per_day and inflated any cost read off it.
#[test]
fn cache_reads_are_counted_apart_from_tokens() {
    let fresh = metric("tokens_fresh_per_day");
    assert_eq!(text(&fresh, "unit"), "tokens");
    let reads = metric("cache_read_per_day");
    assert_eq!(text(&reads, "unit"), "tokens");
    assert_eq!(text(&reads, "task_name"), text(&metric("tokens_per_day"), "task_name"));

    let formula = text(&metric("tokens_per_day"), "formula");
    assert!(formula.contains("tokens_fresh + cache_write + tokens_out"), "{formula}");
    assert!(!formula.contains("tokens_in"), "tokens_in carries the cache reads: {formula}");
}

/// #157: the catalog documents exactly the registered metrics — it had drifted to
/// 18 documented-but-unregistered and 13 registered-but-undocumented.
#[test]
fn the_catalog_documents_exactly_the_registered_metrics() {
    let catalog =
        std::fs::read_to_string(repo_root().join("docs/features/metrics/catalog.md")).unwrap();
    let documented: BTreeSet<String> = catalog
        .lines()
        .filter(|l| l.starts_with("### "))
        .flat_map(|l| l.split('`').skip(1).step_by(2).map(str::to_string).collect::<Vec<_>>())
        .collect();
    let registered: BTreeSet<String> =
        registry().iter().map(|m| m["key"].as_str().unwrap().to_string()).collect();
    let missing: Vec<_> = registered.difference(&documented).collect();
    let extra: Vec<_> = documented.difference(&registered).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "undocumented: {missing:?}; unregistered: {extra:?}"
    );
}
