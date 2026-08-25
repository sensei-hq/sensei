//! Pairwise correlation between metrics — which signals move together, and which
//! only *appear* to.
//!
//! ## Why suppression is the load-bearing part
//!
//! Ranking pairs by |rho| alone does not work. Measured on real data, the top of
//! an unfiltered list is arithmetic:
//!
//! ```text
//!   1.00  tokens_in_per_day / tokens_per_day     (the second CONTAINS the first)
//!   0.97  tokens_out_per_day / tokens_per_day
//!   0.92  session_duration / tokens_per_result   (longer session ⇒ more tokens)
//! ```
//!
//! while the findings worth surfacing sit below them:
//!
//! ```text
//!   0.77  throughput / incomplete_analysis_llm_rate   (faster ⇒ less analysis)
//!  -0.54  spec_depth / spec_deviation_rate            (deeper plan ⇒ less drift)
//!   0.46  interruption_rate / module_quality
//! ```
//!
//! So the registry carries a `derives_from` list per metric and those pairs are
//! dropped, not merely ranked lower — a definitional pair is not a weak insight,
//! it is not an insight at all.
//!
//! ## Spearman, not Pearson
//!
//! Rank correlation. The metrics have wildly different distributions (a token
//! count spans millions, a ratio sits in [0,1]) and a single outlier day would
//! dominate a Pearson coefficient.
//!
//! Everything here is pure — the caller supplies observations — so the statistics
//! are testable without a database.

/// One metric's value on one (project, day). The caller groups by cell.
pub type Cell = std::collections::HashMap<String, f64>;

/// A correlation worth reporting.
#[derive(Debug, Clone, PartialEq)]
pub struct Correlation {
    pub a: String,
    pub b: String,
    /// Spearman's rho in [-1, 1].
    pub rho: f64,
    /// Paired observations behind it — the honesty field. A rho from 8 points is
    /// not the same claim as one from 156, and the UI must be able to say so.
    pub n: usize,
}

/// Minimum paired observations before a pair is reported at all.
///
/// Below this, |rho| is dominated by chance: with n=5 a |rho| above 0.8 arises
/// easily from noise. Reporting it would manufacture confident-looking nonsense,
/// which is worse than staying quiet.
pub const MIN_PAIRS: usize = 20;

/// Minimum |rho| worth showing. Weak-but-real correlations exist, but at these
/// sample sizes they are indistinguishable from noise.
pub const MIN_RHO: f64 = 0.40;

/// Rank a slice, averaging ties (the standard correction — without it, tied
/// values would get arbitrary ranks and bias rho).
fn ranks(xs: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..xs.len()).collect();
    order.sort_by(|&i, &j| xs[i].partial_cmp(&xs[j]).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = vec![0.0; xs.len()];
    let mut i = 0;
    while i < order.len() {
        let mut j = i;
        while j + 1 < order.len() && xs[order[j + 1]] == xs[order[i]] {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for &k in &order[i..=j] {
            out[k] = avg;
        }
        i = j + 1;
    }
    out
}

/// Spearman's rho. `None` when either series is constant — every value tied means
/// no ordering to correlate, and 0/0 would otherwise surface as NaN.
pub fn spearman(xs: &[f64], ys: &[f64]) -> Option<f64> {
    if xs.len() != ys.len() || xs.len() < 2 {
        return None;
    }
    let (rx, ry) = (ranks(xs), ranks(ys));
    let n = xs.len() as f64;
    let (mx, my) = (rx.iter().sum::<f64>() / n, ry.iter().sum::<f64>() / n);
    let mut num = 0.0;
    let (mut dx, mut dy) = (0.0, 0.0);
    for i in 0..xs.len() {
        num += (rx[i] - mx) * (ry[i] - my);
        dx += (rx[i] - mx).powi(2);
        dy += (ry[i] - my).powi(2);
    }
    (dx > 0.0 && dy > 0.0).then(|| num / (dx.sqrt() * dy.sqrt()))
}

/// True when the pair is related by construction and must not be reported.
/// Symmetric: the registry only has to state the relationship on one side.
fn suppressed(a: &str, b: &str, derives: &std::collections::HashMap<String, Vec<String>>) -> bool {
    derives.get(a).is_some_and(|v| v.iter().any(|k| k == b))
        || derives.get(b).is_some_and(|v| v.iter().any(|k| k == a))
}

/// Every reportable correlation across `cells`, strongest first.
///
/// `derives` maps a metric key to the keys it is related to by construction (the
/// registry's `derives_from`). Pairs below [`MIN_PAIRS`] or [`MIN_RHO`], and any
/// suppressed pair, are excluded.
pub fn correlations(
    cells: &[Cell],
    derives: &std::collections::HashMap<String, Vec<String>>,
) -> Vec<Correlation> {
    let mut keys: Vec<&String> = cells.iter().flat_map(|c| c.keys()).collect();
    keys.sort();
    keys.dedup();

    let mut out = Vec::new();
    for (i, a) in keys.iter().enumerate() {
        for b in keys.iter().skip(i + 1) {
            if suppressed(a, b, derives) {
                continue;
            }
            let (mut xs, mut ys) = (Vec::new(), Vec::new());
            for c in cells {
                if let (Some(x), Some(y)) = (c.get(*a), c.get(*b)) {
                    xs.push(*x);
                    ys.push(*y);
                }
            }
            if xs.len() < MIN_PAIRS {
                continue;
            }
            if let Some(rho) = spearman(&xs, &ys)
                && rho.abs() >= MIN_RHO
            {
                out.push(Correlation { a: (*a).clone(), b: (*b).clone(), rho, n: xs.len() });
            }
        }
    }
    out.sort_by(|p, q| q.rho.abs().partial_cmp(&p.rho.abs()).unwrap_or(std::cmp::Ordering::Equal));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cell(pairs: &[(&str, f64)]) -> Cell {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    /// `n` cells where `b` tracks `a` (perfectly, or inverted).
    fn linked(n: usize, a: &str, b: &str, invert: bool) -> Vec<Cell> {
        (0..n)
            .map(|i| {
                let x = i as f64;
                cell(&[(a, x), (b, if invert { -x } else { x * 2.0 })])
            })
            .collect()
    }

    #[test]
    fn spearman_is_monotonic_not_linear() {
        // The reason for rank correlation: a perfectly monotonic but very
        // non-linear relationship is still 1.0, where Pearson would not be.
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0];
        let ys = [1.0, 4.0, 9.0, 16.0, 25.0];
        assert!((spearman(&xs, &ys).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_constant_series_has_no_correlation_rather_than_nan() {
        let xs = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(spearman(&xs, &[7.0; 4]), None);
        assert_eq!(spearman(&[7.0; 4], &xs), None);
    }

    #[test]
    fn ties_are_averaged_not_ordered_arbitrarily() {
        // Without the tie correction the two 2.0s would take ranks 2 and 3 by
        // input order, biasing rho.
        let r = ranks(&[1.0, 2.0, 2.0, 4.0]);
        assert_eq!(r, vec![1.0, 2.5, 2.5, 4.0]);
    }

    #[test]
    fn reports_a_strong_pair_with_its_sample_size() {
        let out = correlations(&linked(30, "ftr", "rework_ratio", true), &HashMap::new());
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].a.as_str(), out[0].b.as_str()), ("ftr", "rework_ratio"));
        assert!((out[0].rho + 1.0).abs() < 1e-9, "perfect inverse, got {}", out[0].rho);
        assert_eq!(out[0].n, 30, "the sample size travels with the claim");
    }

    #[test]
    fn a_pair_related_by_construction_is_dropped_not_ranked_lower() {
        // The whole point. tokens_in_per_day ⊂ tokens_per_day correlates 1.00 on
        // real data; it is not a weak insight, it is not an insight.
        let cells = linked(30, "tokens_in_per_day", "tokens_per_day", false);
        let mut d = HashMap::new();
        d.insert("tokens_per_day".to_string(), vec!["tokens_in_per_day".to_string()]);
        assert!(correlations(&cells, &d).is_empty());
    }

    #[test]
    fn suppression_is_symmetric() {
        // The registry states the relationship once; either direction must work.
        let cells = linked(30, "session_duration", "tokens_per_day", false);
        let mut fwd = HashMap::new();
        fwd.insert("session_duration".to_string(), vec!["tokens_per_day".to_string()]);
        let mut rev = HashMap::new();
        rev.insert("tokens_per_day".to_string(), vec!["session_duration".to_string()]);
        assert!(correlations(&cells, &fwd).is_empty());
        assert!(correlations(&cells, &rev).is_empty());
    }

    #[test]
    fn a_thin_sample_is_not_reported_however_strong_it_looks() {
        // n=5 with a perfect relationship still says nothing: |rho|=1 arises
        // easily from noise at that size.
        let out = correlations(&linked(MIN_PAIRS - 1, "a", "b", false), &HashMap::new());
        assert!(out.is_empty(), "below MIN_PAIRS must stay quiet, got {out:?}");
    }

    #[test]
    fn a_weak_correlation_is_not_reported() {
        // Alternating noise over enough cells: real n, no monotone signal.
        let cells: Vec<Cell> = (0..40)
            .map(|i| cell(&[("a", i as f64), ("b", if i % 2 == 0 { 1.0 } else { 0.0 })]))
            .collect();
        assert!(correlations(&cells, &HashMap::new()).iter().all(|c| c.rho.abs() >= MIN_RHO));
    }

    #[test]
    fn only_cells_carrying_both_metrics_count_toward_n() {
        // A metric present on days the other is absent must not inflate the
        // sample — that would overstate confidence in the pair.
        let mut cells = linked(25, "a", "b", false);
        cells.extend((0..50).map(|i| cell(&[("a", i as f64)])));
        let out = correlations(&cells, &HashMap::new());
        assert_eq!(out[0].n, 25, "unpaired observations excluded");
    }

    #[test]
    fn strongest_first_regardless_of_sign() {
        // A -0.9 outranks a +0.5: direction is the finding, magnitude is the rank.
        let mut cells = linked(30, "a", "b", true);
        for (i, c) in cells.iter_mut().enumerate() {
            c.insert("c".into(), if i % 3 == 0 { 0.0 } else { i as f64 });
        }
        let out = correlations(&cells, &std::collections::HashMap::new());
        assert!(out.windows(2).all(|w| w[0].rho.abs() >= w[1].rho.abs()));
        assert!(out[0].rho < 0.0, "the inverse pair leads");
    }
}
