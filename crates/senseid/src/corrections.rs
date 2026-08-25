//! Corrections aggregation (#65 step 5) — pure clustering of recurring corrective
//! prompts into project-tagged groups. No IO and no gateway here: the handler
//! (`tasks/handlers/corrections.rs`) supplies embeddings and persistence so this
//! logic is unit-testable over plain data.

/// Cosine-similarity threshold: two corrective prompts join the same cluster when
/// their embeddings are at least this similar. Default; caller can override via
/// `sensei.config.corrections.similarity_threshold` (0.0..=1.0).
pub const SIMILARITY_THRESHOLD: f32 = 0.82;

/// A cluster must reach this many members to surface as a "recurring" correction.
/// Default; caller can override via `sensei.config.corrections.cluster_min` (≥1).
///
/// Gap 2 tuning note: the current corpus (114K events across 4 projects)
/// surfaces exactly one cluster at threshold 0.82 + min 2, which suggests
/// corrective prompts are more scattered than the default assumes. Lowering
/// the threshold to ~0.75 or the min to 2 (already the default) is worth
/// trying via config before code-editing the constant.
pub const CORRECTION_CLUSTER_MIN: usize = 2;

/// Parse a `SIMILARITY_THRESHOLD` override from config text. Rejects values
/// outside `[0.0, 1.0]` and falls back to the default on parse failure.
/// Pure so callers can unit-test without a DB round-trip.
pub fn parse_similarity_threshold(raw: Option<&str>) -> f32 {
    raw.and_then(|s| s.trim().parse::<f32>().ok())
        .filter(|v| (0.0..=1.0).contains(v))
        .unwrap_or(SIMILARITY_THRESHOLD)
}

/// Parse a `CORRECTION_CLUSTER_MIN` override from config text. Rejects zero
/// and negative values.
pub fn parse_cluster_min(raw: Option<&str>) -> usize {
    raw.and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|v| *v >= 1)
        .unwrap_or(CORRECTION_CLUSTER_MIN)
}

/// Max chars of the normalized snippet / canonical-text fallback.
const SNIPPET_MAX: usize = 200;

/// One corrective prompt projected to the fields clustering needs.
#[derive(Debug, Clone)]
pub struct CorrItem {
    pub project_id: uuid::Uuid,
    pub session_id: String,
    pub ts: i64, // epoch ms (activity.assistant_events.ts)
    pub prompt: String,
}

/// A derived cluster of similar corrective prompts.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    pub signature: String,
    pub representative_text: String, // normalized snippet of the seed (earliest) member
    pub count: usize,
    pub last_seen_ms: i64,            // max ts across members
    pub project_ids: Vec<uuid::Uuid>, // distinct, sorted
    pub member_idxs: Vec<usize>,      // indices into the input slice
}

/// A row ready to upsert into `inference.corrections`. Plain data carrier shared
/// between the handler (which fills text/suggestion/memory from the LLM) and
/// `pg_store::upsert_correction`.
#[derive(Debug, Clone)]
pub struct CorrectionRow {
    pub signature: String,
    pub text: String,
    pub suggestion: Option<String>,
    pub count: i32,
    pub project_ids: Vec<uuid::Uuid>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub memory_id: Option<uuid::Uuid>,
    pub instances: serde_json::Value,
}

/// Normalize a prompt for lexical comparison + signature stability: lowercase,
/// collapse runs of whitespace/punctuation to single spaces, drop other symbols,
/// cap length on a char boundary.
pub fn normalize(prompt: &str) -> String {
    let lowered = prompt.trim().to_lowercase();
    let mut s = String::with_capacity(lowered.len());
    let mut prev_space = false;
    for ch in lowered.chars() {
        if ch.is_alphanumeric() {
            s.push(ch);
            prev_space = false;
        } else if !prev_space && !s.is_empty() {
            // any non-alphanumeric run (whitespace, ASCII or Unicode punctuation,
            // symbols like em-dash/ellipsis/curly quotes) collapses to one space
            s.push(' ');
            prev_space = true;
        }
    }
    let trimmed = s.trim_end();
    trimmed.chars().take(SNIPPET_MAX).collect()
}

/// Cosine similarity of two equal-length vectors. Returns 0.0 for mismatched or
/// zero-norm inputs (can't compare).
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Deterministic signature from a cluster's seed (its earliest-by-ts member).
/// FNV-1a over "session:normalized_prompt" → hex. Independent of LLM output, so
/// re-runs upsert the same row whether or not models ran.
pub fn signature(seed_session: &str, seed_prompt: &str) -> String {
    let key = format!("{}:{}", seed_session, normalize(seed_prompt));
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in key.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("corr-{:016x}", hash)
}

/// Internal accumulator while building a cluster.
struct ClusterAcc {
    seed_idx: usize,
    members: Vec<usize>,
    last_seen_ms: i64,
    projects: Vec<uuid::Uuid>,
}

impl ClusterAcc {
    fn new(seed_idx: usize, item: &CorrItem) -> Self {
        Self {
            seed_idx,
            members: vec![seed_idx],
            last_seen_ms: item.ts,
            projects: vec![item.project_id],
        }
    }
    fn push(&mut self, idx: usize, item: &CorrItem) {
        self.members.push(idx);
        if item.ts > self.last_seen_ms {
            self.last_seen_ms = item.ts;
        }
        if !self.projects.contains(&item.project_id) {
            self.projects.push(item.project_id);
        }
    }
    fn finish(mut self, items: &[CorrItem]) -> Cluster {
        let seed = &items[self.seed_idx];
        self.projects.sort();
        Cluster {
            signature: signature(&seed.session_id, &seed.prompt),
            representative_text: normalize(&seed.prompt),
            count: self.members.len(),
            last_seen_ms: self.last_seen_ms,
            project_ids: self.projects,
            member_idxs: self.members,
        }
    }
}

/// Greedy, deterministic clustering. `items` and `embeddings` are parallel and
/// `items` MUST be sorted by `ts` ascending so the earliest member seeds each
/// cluster. Each item joins the first existing cluster whose **seed** embedding is
/// within `threshold`, else it seeds a new cluster.
pub fn cluster(items: &[CorrItem], embeddings: &[Vec<f32>], threshold: f32) -> Vec<Cluster> {
    debug_assert_eq!(
        items.len(),
        embeddings.len(),
        "cluster: embeddings must be parallel to items"
    );
    let mut accs: Vec<ClusterAcc> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let mut joined = false;
        if let Some(emb) = embeddings.get(i) {
            for acc in accs.iter_mut() {
                if let Some(seed_emb) = embeddings.get(acc.seed_idx)
                    && cosine(emb, seed_emb) >= threshold
                {
                    acc.push(i, item);
                    joined = true;
                    break;
                }
            }
        }
        if !joined {
            accs.push(ClusterAcc::new(i, item));
        }
    }
    accs.into_iter().map(|a| a.finish(items)).collect()
}

/// Lexical fallback when embeddings are unavailable: group by normalized text.
/// Deterministic. `items` must be sorted by `ts` ascending.
pub fn lexical_cluster(items: &[CorrItem]) -> Vec<Cluster> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, item) in items.iter().enumerate() {
        groups.entry(normalize(&item.prompt)).or_default().push(i);
    }
    groups
        .into_values()
        .map(|idxs| {
            let mut acc = ClusterAcc::new(idxs[0], &items[idxs[0]]);
            for &i in &idxs[1..] {
                acc.push(i, &items[i]);
            }
            acc.finish(items)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(pid: uuid::Uuid, session: &str, ts: i64, prompt: &str) -> CorrItem {
        CorrItem { project_id: pid, session_id: session.into(), ts, prompt: prompt.into() }
    }

    // ── parse_similarity_threshold ──────────────────────────────────────
    #[test]
    fn similarity_default_when_config_missing() {
        assert_eq!(parse_similarity_threshold(None), SIMILARITY_THRESHOLD);
    }

    #[test]
    fn similarity_default_when_config_unparseable() {
        assert_eq!(parse_similarity_threshold(Some("not a number")), SIMILARITY_THRESHOLD);
        assert_eq!(parse_similarity_threshold(Some("")), SIMILARITY_THRESHOLD);
    }

    #[test]
    fn similarity_default_when_out_of_range() {
        assert_eq!(parse_similarity_threshold(Some("-0.1")), SIMILARITY_THRESHOLD);
        assert_eq!(parse_similarity_threshold(Some("1.1")), SIMILARITY_THRESHOLD);
    }

    #[test]
    fn similarity_honors_valid_override() {
        assert_eq!(parse_similarity_threshold(Some("0.75")), 0.75);
        assert_eq!(parse_similarity_threshold(Some(" 0.9 ")), 0.9); // trims whitespace
    }

    // ── parse_cluster_min ───────────────────────────────────────────────
    #[test]
    fn cluster_min_default_when_config_missing() {
        assert_eq!(parse_cluster_min(None), CORRECTION_CLUSTER_MIN);
    }

    #[test]
    fn cluster_min_default_when_zero_or_negative() {
        // usize can't be negative, but zero is invalid — reject it.
        assert_eq!(parse_cluster_min(Some("0")), CORRECTION_CLUSTER_MIN);
    }

    #[test]
    fn cluster_min_honors_valid_override() {
        assert_eq!(parse_cluster_min(Some("3")), 3);
        assert_eq!(parse_cluster_min(Some("10")), 10);
    }

    #[test]
    fn normalize_lowercases_strips_punct_collapses_ws() {
        assert_eq!(normalize("  Use `$state(…)`, please!! "), "use state please");
        // idempotent
        let once = normalize("No, that's WRONG -- revert it.");
        assert_eq!(normalize(&once), once);
    }

    #[test]
    fn cosine_identity_orthogonal_and_mismatch() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine(&[1.0, 0.0], &[1.0]), 0.0, "mismatched length");
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 1.0]), 0.0, "zero norm");
    }

    #[test]
    fn signature_is_deterministic_and_text_sensitive() {
        let a = signature("s1", "No, revert that change");
        assert_eq!(a, signature("s1", "no,  revert   that change"), "stable under normalization");
        assert_ne!(a, signature("s2", "No, revert that change"), "session-sensitive");
        assert_ne!(a, signature("s1", "use $state instead"), "text-sensitive");
    }

    #[test]
    fn cluster_merges_similar_and_separates_distinct() {
        let p = uuid::Uuid::new_v4();
        let items = vec![
            item(p, "s1", 100, "use $state"),
            item(p, "s2", 200, "use $state please"),
            item(p, "s3", 300, "add a retry"),
        ];
        // embeddings: first two near-identical, third orthogonal
        let embs = vec![vec![1.0, 0.0], vec![0.99, 0.14], vec![0.0, 1.0]];
        let mut clusters = cluster(&items, &embs, 0.82);
        clusters.sort_by_key(|c| c.count);
        assert_eq!(clusters.len(), 2);
        let big = clusters.iter().find(|c| c.count == 2).unwrap();
        assert_eq!(big.last_seen_ms, 200, "max ts of members");
        assert_eq!(big.project_ids, vec![p]);
    }

    #[test]
    fn cluster_seed_is_earliest_and_signature_stable_as_it_grows() {
        let p = uuid::Uuid::new_v4();
        let base = vec![item(p, "s1", 100, "use $state"), item(p, "s2", 200, "use $state please")];
        let embs2 = vec![vec![1.0, 0.0], vec![0.99, 0.14]];
        let c2 = cluster(&base, &embs2, 0.82);
        assert_eq!(c2.len(), 1);
        let sig2 = c2[0].signature.clone();

        // A later similar member joins; seed (earliest ts=100) unchanged → same signature.
        let grown = vec![
            item(p, "s1", 100, "use $state"),
            item(p, "s2", 200, "use $state please"),
            item(p, "s3", 300, "please use $state here"),
        ];
        let embs3 = vec![vec![1.0, 0.0], vec![0.99, 0.14], vec![0.98, 0.2]];
        let c3 = cluster(&grown, &embs3, 0.82);
        assert_eq!(c3.len(), 1);
        assert_eq!(c3[0].count, 3);
        assert_eq!(c3[0].signature, sig2, "signature stable as the cluster grows");
    }

    #[test]
    fn cluster_dedups_projects() {
        let p1 = uuid::Uuid::new_v4();
        let p2 = uuid::Uuid::new_v4();
        let items = vec![
            item(p1, "s1", 100, "use $state"),
            item(p2, "s2", 200, "use $state please"),
            item(p1, "s3", 300, "use $state now"),
        ];
        let embs = vec![vec![1.0, 0.0], vec![0.99, 0.14], vec![0.98, 0.2]];
        let c = cluster(&items, &embs, 0.82);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].project_ids.len(), 2, "two distinct projects, deduped");
    }

    #[test]
    fn normalize_treats_unicode_symbols_as_word_boundaries() {
        assert_eq!(super::normalize("fix—this"), "fix this", "em-dash is a boundary, not dropped");
        assert_eq!(
            super::normalize("use \u{201c}state\u{201d} now"),
            "use state now",
            "curly quotes"
        );
        assert_eq!(super::normalize("a…b"), "a b", "ellipsis is a boundary");
    }

    #[test]
    fn lexical_cluster_groups_by_normalized_text() {
        let p = uuid::Uuid::new_v4();
        let items = vec![
            item(p, "s1", 100, "Revert that!"),
            item(p, "s2", 200, "revert  that"),
            item(p, "s3", 300, "add a retry"),
        ];
        let mut c = lexical_cluster(&items);
        c.sort_by_key(|x| x.count);
        assert_eq!(c.len(), 2);
        assert_eq!(c.last().unwrap().count, 2);
    }
}
