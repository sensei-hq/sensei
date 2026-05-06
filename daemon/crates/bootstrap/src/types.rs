//! Core types for bootstrap health checks and hardware detection.

use serde::{Deserialize, Serialize};

/// State of a single component during bootstrap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ComponentState {
    Detecting,
    Installing,
    Starting,
    Upgrading,
    Pulling {
        progress_pct: u8,
        size_mb: u32,
    },
    Ready,
    Failed {
        error: String,
    },
    Skipped,
}

/// Status of a single bootstrap component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub name: String,
    pub state: ComponentState,
    pub version: Option<String>,
    pub detail: Option<String>,
}

impl ComponentStatus {
    pub fn ready(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            state: ComponentState::Ready,
            version: Some(version.to_string()),
            detail: None,
        }
    }

    pub fn failed(name: &str, error: &str) -> Self {
        Self {
            name: name.to_string(),
            state: ComponentState::Failed { error: error.to_string() },
            version: None,
            detail: None,
        }
    }

    pub fn missing(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: ComponentState::Failed { error: "not installed".to_string() },
            version: None,
            detail: None,
        }
    }

    pub fn detecting(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: ComponentState::Detecting,
            version: None,
            detail: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.state, ComponentState::Ready)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.state, ComponentState::Failed { .. })
    }
}

/// Hardware capabilities of the host machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub ram_gb: u32,
    pub cpu_cores: u32,
    pub gpu: Option<String>,
    pub metal_support: bool,
    pub recommended_tier: ModelTier,
}

/// Model tier based on hardware capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    /// 8GB RAM — gemma3:12b only
    Minimum,
    /// 16GB RAM — gemma3:27b + qwen3:14b
    Recommended,
    /// 32GB+ RAM — full MOE panel
    Full,
}

impl ModelTier {
    pub fn from_ram(ram_gb: u32) -> Self {
        match ram_gb {
            0..=15 => ModelTier::Minimum,
            16..=31 => ModelTier::Recommended,
            _ => ModelTier::Full,
        }
    }
}

/// Result of a full bootstrap run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResult {
    pub components: Vec<ComponentStatus>,
    pub hardware: HardwareInfo,
    pub ready: bool,
}

impl BootstrapResult {
    pub fn from_checks(components: Vec<ComponentStatus>, hardware: HardwareInfo) -> Self {
        let ready = components.iter().all(|c| {
            matches!(c.state, ComponentState::Ready | ComponentState::Skipped)
        });
        Self { components, hardware, ready }
    }
}

/// Action classification for a diagnostic trace step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceAction {
    /// Passive probe — binary present, port open, DB exists.
    Check,
    /// Active remediation — install, start service, create DB.
    Resolve,
    /// Human action required — displayed as an instruction.
    Instruct,
}

/// A single diagnostic step captured during bootstrap.
/// Pure data — no file I/O, no side effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapTrace {
    pub id:            String,          // counter-based id, e.g. "trace-00000001"
    pub ts:            String,          // ISO 8601, e.g. "2026-05-01T10:00:00Z"
    pub action_type:   TraceAction,
    pub step:          String,          // snake_case step name, e.g. "postgres_port"
    pub desc:          String,          // human-readable label
    pub cmd:           String,          // command string or "tcp probe host:port"
    pub exit:          Option<i32>,     // process exit code (None for TCP probes)
    pub out:           String,          // stdout (trimmed)
    pub err:           String,          // stderr (trimmed)
    pub ms:            u64,             // wall-clock duration in milliseconds
    pub ok:            bool,            // did this step pass?
    pub fix_attempted: bool,            // did bootstrap attempt a fix?
    pub fix_approach:  Option<String>,  // command/strategy used to fix
    pub fix_ok:        Option<bool>,    // did the fix succeed?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_status_ready_constructor() {
        let s = ComponentStatus::ready("postgresql", "17.2");
        assert!(s.is_ready());
        assert!(!s.is_failed());
        assert_eq!(s.name, "postgresql");
        assert_eq!(s.version.as_deref(), Some("17.2"));
    }

    #[test]
    fn component_status_failed_constructor() {
        let s = ComponentStatus::failed("ollama", "connection refused");
        assert!(s.is_failed());
        assert!(!s.is_ready());
        assert_eq!(s.name, "ollama");
    }

    #[test]
    fn component_status_missing_constructor() {
        let s = ComponentStatus::missing("homebrew");
        assert!(s.is_failed());
        assert_eq!(s.version, None);
    }

    #[test]
    fn bootstrap_result_ready_when_all_ready() {
        let hw = HardwareInfo {
            ram_gb: 16, cpu_cores: 8, gpu: None,
            metal_support: false, recommended_tier: ModelTier::Recommended,
        };
        let components = vec![
            ComponentStatus::ready("homebrew", "4.4.2"),
            ComponentStatus::ready("postgresql", "17.2"),
        ];
        let result = BootstrapResult::from_checks(components, hw);
        assert!(result.ready);
    }

    #[test]
    fn bootstrap_result_not_ready_when_any_failed() {
        let hw = HardwareInfo {
            ram_gb: 16, cpu_cores: 8, gpu: None,
            metal_support: false, recommended_tier: ModelTier::Recommended,
        };
        let components = vec![
            ComponentStatus::ready("homebrew", "4.4.2"),
            ComponentStatus::failed("postgresql", "not running"),
        ];
        let result = BootstrapResult::from_checks(components, hw);
        assert!(!result.ready);
    }

    #[test]
    fn bootstrap_result_ready_with_skipped() {
        let hw = HardwareInfo {
            ram_gb: 8, cpu_cores: 4, gpu: None,
            metal_support: false, recommended_tier: ModelTier::Minimum,
        };
        let components = vec![
            ComponentStatus::ready("homebrew", "4.4.2"),
            ComponentStatus {
                name: "ollama".into(),
                state: ComponentState::Skipped,
                version: None,
                detail: None,
            },
        ];
        let result = BootstrapResult::from_checks(components, hw);
        assert!(result.ready);
    }

    #[test]
    fn model_tier_from_ram() {
        assert_eq!(ModelTier::from_ram(4), ModelTier::Minimum);
        assert_eq!(ModelTier::from_ram(8), ModelTier::Minimum);
        assert_eq!(ModelTier::from_ram(16), ModelTier::Recommended);
        assert_eq!(ModelTier::from_ram(32), ModelTier::Full);
        assert_eq!(ModelTier::from_ram(64), ModelTier::Full);
    }

    #[test]
    fn component_status_serializes_to_json() {
        let s = ComponentStatus::ready("sensei", "0.9.4");
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["name"], "sensei");
        assert_eq!(json["state"]["state"], "ready");
        assert_eq!(json["version"], "0.9.4");
    }

    #[test]
    fn failed_state_includes_error_in_json() {
        let s = ComponentStatus::failed("daemon", "port in use");
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["state"]["state"], "failed");
        assert_eq!(json["state"]["error"], "port in use");
    }

    #[test]
    fn bootstrap_trace_serializes() {
        let t = BootstrapTrace {
            id:            "trace-00000001".to_string(),
            ts:            "2026-05-01T10:00:00Z".to_string(),
            action_type:   TraceAction::Check,
            step:          "postgres_binary".to_string(),
            desc:          "Locate postgres binary".to_string(),
            cmd:           "which postgres".to_string(),
            exit:          Some(0),
            out:           "/opt/homebrew/bin/postgres".to_string(),
            err:           String::new(),
            ms:            4,
            ok:            true,
            fix_attempted: false,
            fix_approach:  None,
            fix_ok:        None,
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("\"step\":\"postgres_binary\""));
        assert!(json.contains("\"ok\":true"));
        let round_trip: BootstrapTrace = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip.step, t.step);
        assert!(matches!(round_trip.action_type, TraceAction::Check));
    }
}
