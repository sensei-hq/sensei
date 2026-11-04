//! A workspace's SDLC metric settings (spec S1, D3): a project-level hourly rate,
//! the minutes a story point is expected to take, and the idle gap that ends a
//! work slice. Never per person.

/// The settings as they apply: a workspace with no row reads as the defaults.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MetricSettings {
    pub hourly_rate: Option<f64>,
    pub minutes_per_point: Option<f64>,
    pub idle_threshold_minutes: i32,
}

/// The documented default idle gap (spec S1): a setting, not a measurement.
pub const DEFAULT_IDLE_THRESHOLD_MINUTES: i32 = 15;

impl Default for MetricSettings {
    fn default() -> Self {
        MetricSettings {
            hourly_rate: None,
            minutes_per_point: None,
            idle_threshold_minutes: DEFAULT_IDLE_THRESHOLD_MINUTES,
        }
    }
}

/// What a `PUT` changes. The outer `Option` is "named in the body"; the inner one,
/// for the two clearable keys, is the value or `null`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SettingsPatch {
    pub hourly_rate: Option<Option<f64>>,
    pub minutes_per_point: Option<Option<f64>>,
    pub idle_threshold_minutes: Option<i32>,
}

impl SettingsPatch {
    pub fn apply(&self, to: &MetricSettings) -> MetricSettings {
        MetricSettings {
            hourly_rate: self.hourly_rate.unwrap_or(to.hourly_rate),
            minutes_per_point: self.minutes_per_point.unwrap_or(to.minutes_per_point),
            idle_threshold_minutes: self
                .idle_threshold_minutes
                .unwrap_or(to.idle_threshold_minutes),
        }
    }
}

/// Parse a `PUT` body, or name the key that is wrong (`body` for a body that is
/// not an object). Bounds match the table's checks, so the database never has to
/// refuse what the API accepted.
pub fn parse_patch(body: &serde_json::Value) -> Result<SettingsPatch, String> {
    let obj = body.as_object().ok_or("body")?;
    let mut patch = SettingsPatch::default();
    for (key, v) in obj {
        let bad = || key.clone();
        match key.as_str() {
            "hourly_rate" => patch.hourly_rate = Some(bounded(v, 10_000.0).ok_or_else(bad)?),
            "minutes_per_point" => {
                patch.minutes_per_point = Some(bounded(v, 10_080.0).ok_or_else(bad)?)
            }
            "idle_threshold_minutes" => {
                let n = v.as_i64().filter(|n| (1..=240).contains(n)).ok_or_else(bad)?;
                patch.idle_threshold_minutes = Some(n as i32);
            }
            _ => return Err(bad()),
        }
    }
    Ok(patch)
}

/// `null` (clear), or a finite number in `(0, max]`; `None` for anything else.
fn bounded(v: &serde_json::Value, max: f64) -> Option<Option<f64>> {
    if v.is_null() {
        return Some(None);
    }
    v.as_f64().filter(|x| x.is_finite() && *x > 0.0 && *x <= max).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_subset_sets_only_its_keys() {
        assert_eq!(
            parse_patch(&json!({"hourly_rate": 50})),
            Ok(SettingsPatch { hourly_rate: Some(Some(50.0)), ..Default::default() })
        );
        assert_eq!(
            parse_patch(&json!({"minutes_per_point": 90.5, "idle_threshold_minutes": 20})),
            Ok(SettingsPatch {
                minutes_per_point: Some(Some(90.5)),
                idle_threshold_minutes: Some(20),
                ..Default::default()
            })
        );
        assert_eq!(parse_patch(&json!({})), Ok(SettingsPatch::default()));
    }

    #[test]
    fn null_clears_a_rate_but_never_the_threshold() {
        assert_eq!(
            parse_patch(&json!({"hourly_rate": null, "minutes_per_point": null})),
            Ok(SettingsPatch {
                hourly_rate: Some(None),
                minutes_per_point: Some(None),
                ..Default::default()
            })
        );
        assert_eq!(
            parse_patch(&json!({"idle_threshold_minutes": null})),
            Err("idle_threshold_minutes".into())
        );
    }

    #[test]
    fn a_bad_value_names_its_key() {
        for (body, key) in [
            (json!({"hourly_rate": -1}), "hourly_rate"),
            (json!({"hourly_rate": 0}), "hourly_rate"),
            (json!({"hourly_rate": 10001}), "hourly_rate"),
            (json!({"hourly_rate": "NaN"}), "hourly_rate"),
            (json!({"hourly_rate": "50"}), "hourly_rate"),
            (json!({"minutes_per_point": 10081}), "minutes_per_point"),
            (json!({"idle_threshold_minutes": 0}), "idle_threshold_minutes"),
            (json!({"idle_threshold_minutes": 241}), "idle_threshold_minutes"),
            (json!({"idle_threshold_minutes": 15.5}), "idle_threshold_minutes"),
            (json!({"x": 1}), "x"),
            (json!([1]), "body"),
        ] {
            assert_eq!(parse_patch(&body), Err(key.to_string()), "{body}");
        }
    }

    #[test]
    fn applying_keeps_what_the_patch_does_not_name() {
        let now = MetricSettings {
            hourly_rate: Some(40.0),
            minutes_per_point: Some(60.0),
            idle_threshold_minutes: 15,
        };
        let patch = SettingsPatch {
            hourly_rate: Some(None),
            idle_threshold_minutes: Some(30),
            ..Default::default()
        };
        assert_eq!(
            patch.apply(&now),
            MetricSettings {
                hourly_rate: None,
                minutes_per_point: Some(60.0),
                idle_threshold_minutes: 30
            }
        );
    }

    #[test]
    fn the_bounds_themselves_are_accepted() {
        assert_eq!(
            parse_patch(
                &json!({"hourly_rate": 10000, "minutes_per_point": 10080, "idle_threshold_minutes": 240})
            ),
            Ok(SettingsPatch {
                hourly_rate: Some(Some(10000.0)),
                minutes_per_point: Some(Some(10080.0)),
                idle_threshold_minutes: Some(240)
            })
        );
        assert_eq!(
            parse_patch(&json!({"idle_threshold_minutes": 1})).map(|p| p.idle_threshold_minutes),
            Ok(Some(1))
        );
    }
}
