//! What the assistant actually costs — from a SUBSCRIPTION, not from tokens.
//!
//! Token counts cannot price a subscription plan. Under a flat monthly fee the
//! marginal cost of a token is zero: spending more tokens does not cost more, and
//! a `tokens_per_day` figure read as money is wrong in both magnitude and
//! direction. (Measured on real transcripts, the naive input total is ~8x the
//! billable-equivalent because ~98% of it is cache reads, which bill far cheaper —
//! and it goes UP when caching gets better.)
//!
//! So cost is user-supplied: the plan's price and period, configured once
//! (`cost.subscription`), and allocated to a day. Tokens stay what they honestly
//! are — a UTILIZATION signal (how much context the work consumed), not a price.
//!
//! Metered API billing is the other half of this and is deliberately left as a
//! separate variant to add later: when per-token rates are known the same
//! `DailyCost` can be derived from usage instead of a flat rate, and the consumers
//! don't change.
//!
//! Everything here is pure — no DB, no clock — so the arithmetic is testable on
//! its own. `None` everywhere means "not configured"; nothing fabricates a price.

use serde::{Deserialize, Serialize};

/// Config key holding the JSON [`Subscription`]. One key, JSON-valued, matching
/// the `setup.preferences` precedent rather than scattering four scalar keys.
pub const SUBSCRIPTION_CONFIG_KEY: &str = "cost.subscription";

/// How often the subscription renews.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingPeriod {
    Monthly,
    Yearly,
}

impl BillingPeriod {
    /// Days the period covers, for allocating a flat fee to one day.
    ///
    /// 365/12 ≈ 30.4375 rather than "the days in THIS month": a per-day cost that
    /// changes because February is short would make the metric move when nothing
    /// about the work changed. A constant divisor keeps day-over-day comparable,
    /// which is the entire point of a daily series.
    pub fn days(self) -> f64 {
        match self {
            BillingPeriod::Monthly => 365.0 / 12.0,
            BillingPeriod::Yearly => 365.0,
        }
    }
}

/// A user's assistant subscription. `amount` is in whole currency units (200.0 =
/// $200), `currency` an ISO-4217 code carried through for display only — no
/// conversion is attempted, because guessing an FX rate would be fabricating a
/// number the user never gave us.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subscription {
    pub amount: f64,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub period: BillingPeriod,
    /// Free-text plan label ("Max 20x"), for display only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
}

fn default_currency() -> String {
    "USD".to_string()
}

impl Subscription {
    /// Parse from the config value. `None` for absent/blank/unparseable input, or
    /// for a non-positive amount — a zero or negative price is not a real reading,
    /// and admitting "not configured" beats publishing a cost of 0.00 that a
    /// caller cannot distinguish from a genuinely free plan.
    pub fn parse(raw: Option<&str>) -> Option<Self> {
        let raw = raw?.trim();
        if raw.is_empty() {
            return None;
        }
        let s: Subscription = serde_json::from_str(raw).ok()?;
        (s.amount.is_finite() && s.amount > 0.0).then_some(s)
    }

    /// The flat fee allocated to a single day.
    pub fn daily_rate(&self) -> f64 {
        self.amount / self.period.days()
    }

    /// Cost attributable to one delivered result over `window_days` — the number a
    /// subscription can honestly answer: "what did each shipped thing cost me?".
    ///
    /// A WINDOW, not a single day, and that is a correctness matter rather than a
    /// smoothing preference. Delivery is lumpy: measured on real data, only 2 of
    /// the last 14 days carried any result at all, so a per-day figure would be
    /// null ~86% of the time and wildly volatile on the days it existed. A trailing
    /// window also matches how the fee is actually incurred — you are billed for a
    /// period, not a day.
    ///
    /// `None` when the window delivered nothing: dividing by zero is infinite, and
    /// charging the whole window's fee to "nothing" would call an idle stretch
    /// infinitely expensive rather than simply idle.
    pub fn cost_per_result(&self, results: u32, window_days: u32) -> Option<f64> {
        (results > 0 && window_days > 0)
            .then(|| self.daily_rate() * f64::from(window_days) / f64::from(results))
    }

    /// The fee attributable to a stretch of `window_days`.
    pub fn window_cost(&self, window_days: u32) -> f64 {
        self.daily_rate() * f64::from(window_days)
    }
}

/// Trailing window for the cost metric, in days. Thirty ≈ the monthly billing
/// period, so the question it answers ("what did a result cost me this month?")
/// matches the bill the user actually receives.
pub const COST_WINDOW_DAYS: u32 = 30;

#[cfg(test)]
mod tests {
    use super::*;

    fn max20() -> Subscription {
        Subscription {
            amount: 200.0,
            currency: "USD".into(),
            period: BillingPeriod::Monthly,
            plan: Some("Max 20x".into()),
        }
    }

    #[test]
    fn parses_a_configured_plan() {
        let s = Subscription::parse(Some(
            r#"{"amount":200,"currency":"USD","period":"monthly","plan":"Max 20x"}"#,
        ))
        .expect("valid config parses");
        assert_eq!(s, max20());
    }

    #[test]
    fn currency_defaults_and_plan_is_optional() {
        let s = Subscription::parse(Some(r#"{"amount":20,"period":"monthly"}"#)).unwrap();
        assert_eq!(s.currency, "USD");
        assert_eq!(s.plan, None);
    }

    #[test]
    fn unconfigured_is_none_rather_than_a_zero_cost() {
        // Every one of these means "the user has not told us" — publishing 0.00
        // would be indistinguishable from a genuinely free plan.
        for raw in [None, Some(""), Some("   "), Some("not json"), Some("{}")] {
            assert!(Subscription::parse(raw).is_none(), "{raw:?} must not parse");
        }
    }

    #[test]
    fn a_non_positive_or_non_finite_amount_is_not_a_reading() {
        for amt in ["0", "-5", "1e999"] {
            let raw = format!(r#"{{"amount":{amt},"period":"monthly"}}"#);
            assert!(Subscription::parse(Some(&raw)).is_none(), "amount {amt} must not parse");
        }
    }

    #[test]
    fn daily_rate_uses_a_constant_month_so_the_series_is_comparable() {
        // 200 / (365/12) — NOT 200/28 in February and 200/31 in March, which would
        // move the metric when nothing about the work changed.
        let got = max20().daily_rate();
        assert!((got - 200.0 / (365.0 / 12.0)).abs() < 1e-9, "got {got}");
        assert!((got - 6.5753).abs() < 1e-3, "≈ $6.58/day, got {got}");
    }

    #[test]
    fn a_yearly_plan_allocates_over_the_year() {
        let s = Subscription { period: BillingPeriod::Yearly, amount: 2400.0, ..max20() };
        assert!((s.daily_rate() - 2400.0 / 365.0).abs() < 1e-9);
    }

    #[test]
    fn cost_per_result_divides_the_window_fee_across_what_shipped() {
        let s = max20();
        let four = s.cost_per_result(4, COST_WINDOW_DAYS).unwrap();
        // $200/mo over a 30-day window ≈ the monthly fee, split four ways.
        assert!((four - s.window_cost(COST_WINDOW_DAYS) / 4.0).abs() < 1e-9);
        assert!((four - 49.3).abs() < 0.5, "≈ $49 per result, got {four}");
        // Twice the delivery for the same fee halves the unit cost — the direction
        // that makes this a genuine efficiency signal, unlike a token count.
        let eight = s.cost_per_result(8, COST_WINDOW_DAYS).unwrap();
        assert!(eight < four, "{eight} !< {four}");
    }

    #[test]
    fn a_window_that_delivered_nothing_has_no_cost_per_result() {
        // Not infinity, and not the whole window's fee — an idle stretch is idle,
        // not infinitely expensive.
        assert_eq!(max20().cost_per_result(0, COST_WINDOW_DAYS), None);
        // A zero-length window is not a reading either.
        assert_eq!(max20().cost_per_result(4, 0), None);
    }

    #[test]
    fn a_longer_window_costs_proportionally_more() {
        let s = max20();
        let thirty = s.window_cost(30);
        assert!((s.window_cost(60) - 2.0 * thirty).abs() < 1e-9);
        assert!((thirty - 200.0).abs() < 3.0, "≈ one month's fee, got {thirty}");
    }
}
