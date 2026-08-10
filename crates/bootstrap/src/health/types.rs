use serde::{Deserialize, Serialize};

// Enums (closed sets, serialized lowercase / kebab-case to match TS)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Macos,
    Linux,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HealthStatus {
    Checking,
    Resolving,
    Ok,
    NeedsAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentStatus {
    Pending,
    Checking,
    Installing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentId {
    Postgres,
    Ollama,
    Sensei,
    Database,
    Daemon,
}

pub const COMPONENT_ORDER: [ComponentId; 5] = [
    ComponentId::Postgres,
    ComponentId::Ollama,
    ComponentId::Sensei,
    ComponentId::Database,
    ComponentId::Daemon,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManagerId {
    Homebrew,
    Winget,
}

/// The daemon's own database-connection mode, as reported by the daemon that
/// served this payload. Distinct from the Postgres *component* status: the
/// component probe answers "is Postgres reachable / is the schema deployed",
/// while this answers "does the running daemon currently have a working pool".
/// A daemon that lost the cold-boot race can have `Postgres: ready` yet be
/// `Degraded` here until its background self-heal reconnects. `None` on a
/// payload produced outside a daemon (e.g. the app's bootstrap sidecar check),
/// where the notion doesn't apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DaemonDbMode {
    Full,
    Degraded,
}

// Component & Remedy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub id: String,
    pub label: String,
    pub note: Option<String>,
    pub status: ComponentStatus,
    pub version: Option<String>,
    pub detail: Option<String>,
    /// Verb to display when status is Installing. Derived from
    /// `DependencySpec::installing_verb` and carried on the wire so the
    /// frontend doesn't have to duplicate the mapping.
    pub installing_verb: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remedy {
    pub message: String,
    pub script: String,
    pub url: Option<String>,
}

// HealthPayload — top-level wire shape with serde-named fields
// Note: serde renames uptime_seconds to camelCase for JSON to match the TS shape
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthPayload {
    pub version: String,
    pub uptime_seconds: u64,
    /// The Postgres database name the running daemon is connected to.
    /// Sourced from `SenseiConfig::from_env().db_name` at health-check
    /// time, so a daemon spawned with `--instance=e2e` reports
    /// `sensei_e2e` here. Lets e2e globalSetup assert isolation before
    /// any test runs against the wrong DB — and lets the UI surface
    /// "you're talking to the e2e daemon" if instance mismatch occurs.
    pub db_name: String,
    pub platform: Platform,
    pub package_manager: Component,
    pub components: Vec<Component>,
    pub status: HealthStatus,
    pub remedy: Option<Remedy>,
    /// The daemon's DB-connection mode. Set by the daemon's `/health` handler;
    /// `None` (omitted from the wire) when the payload isn't daemon-produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daemon_db_mode: Option<DaemonDbMode>,
}

impl HealthPayload {
    /// Runtime invariant guard mirroring the TS apply() checks (INV-1/2/3).
    pub fn validate(&self) -> Result<(), String> {
        // INV-1: needs-action ⇔ remedy.is_some()
        match (self.status, self.remedy.as_ref()) {
            (HealthStatus::NeedsAction, None) => {
                return Err("HealthPayload: needs-action requires a remedy".to_string())
            }
            (s, Some(_)) if s != HealthStatus::NeedsAction => {
                return Err(format!(
                    "HealthPayload: status={:?} must not carry a remedy",
                    s
                ))
            }
            _ => {}
        }
        // INV-2: exactly 5 components in canonical order
        if self.components.len() != COMPONENT_ORDER.len() {
            return Err(format!(
                "HealthPayload: expected {} components, got {}",
                COMPONENT_ORDER.len(),
                self.components.len()
            ));
        }
        for (i, expected) in COMPONENT_ORDER.iter().enumerate() {
            let want = super::ids::component_id_str(*expected);
            if self.components[i].id != want {
                return Err(format!(
                    "HealthPayload: components[{}].id must be \"{}\", got \"{}\"",
                    i, want, self.components[i].id
                ));
            }
        }
        // INV-3: platform/packageManager pairing
        let want = match self.platform {
            Platform::Windows => "winget",
            _ => "homebrew",
        };
        if self.package_manager.id != want {
            return Err(format!(
                "HealthPayload: platform={:?} expects packageManager.id=\"{}\", got \"{}\"",
                self.platform, want, self.package_manager.id
            ));
        }
        Ok(())
    }
}

// HealthEvent — streaming events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum HealthEvent {
    Phase { phase: HealthStatus },
    Component { id: String, patch: ComponentPatch },
    Remedy { remedy: Remedy },
    Report { payload: HealthPayload },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ComponentStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Option<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Enum serialization ──

    #[test]
    fn health_status_serializes_kebab_case() {
        assert_eq!(
            serde_json::to_string(&HealthStatus::NeedsAction).unwrap(),
            "\"needs-action\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Checking).unwrap(),
            "\"checking\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Resolving).unwrap(),
            "\"resolving\""
        );
        assert_eq!(serde_json::to_string(&HealthStatus::Ok).unwrap(), "\"ok\"");
    }

    #[test]
    fn component_status_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&ComponentStatus::Ready).unwrap(),
            "\"ready\""
        );
        assert_eq!(
            serde_json::to_string(&ComponentStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&ComponentStatus::Checking).unwrap(),
            "\"checking\""
        );
        assert_eq!(
            serde_json::to_string(&ComponentStatus::Installing).unwrap(),
            "\"installing\""
        );
        assert_eq!(
            serde_json::to_string(&ComponentStatus::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[test]
    fn component_id_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&ComponentId::Postgres).unwrap(),
            "\"postgres\""
        );
        assert_eq!(
            serde_json::to_string(&ComponentId::Ollama).unwrap(),
            "\"ollama\""
        );
        assert_eq!(
            serde_json::to_string(&ComponentId::Sensei).unwrap(),
            "\"sensei\""
        );
        assert_eq!(
            serde_json::to_string(&ComponentId::Database).unwrap(),
            "\"database\""
        );
        assert_eq!(
            serde_json::to_string(&ComponentId::Daemon).unwrap(),
            "\"daemon\""
        );
    }

    #[test]
    fn package_manager_id_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&PackageManagerId::Homebrew).unwrap(),
            "\"homebrew\""
        );
        assert_eq!(
            serde_json::to_string(&PackageManagerId::Winget).unwrap(),
            "\"winget\""
        );
    }

    #[test]
    fn platform_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&Platform::Macos).unwrap(),
            "\"macos\""
        );
        assert_eq!(
            serde_json::to_string(&Platform::Linux).unwrap(),
            "\"linux\""
        );
        assert_eq!(
            serde_json::to_string(&Platform::Windows).unwrap(),
            "\"windows\""
        );
    }

    // ── HealthEvent tagged-union ──

    #[test]
    fn health_event_phase_serializes_with_kind_tag() {
        let ev = HealthEvent::Phase {
            phase: HealthStatus::Checking,
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert_eq!(s, r#"{"kind":"phase","phase":"checking"}"#);
    }

    #[test]
    fn health_event_component_serializes_with_patch() {
        let ev = HealthEvent::Component {
            id: "postgres".to_string(),
            patch: ComponentPatch {
                status: Some(ComponentStatus::Installing),
                ..Default::default()
            },
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&ev).unwrap()).unwrap();
        assert_eq!(json["kind"], "component");
        assert_eq!(json["id"], "postgres");
        assert_eq!(json["patch"]["status"], "installing");
    }

    #[test]
    fn health_event_remedy_carries_remedy_object() {
        let ev = HealthEvent::Remedy {
            remedy: Remedy {
                message: "m".into(),
                script: "s".into(),
                url: None,
            },
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&ev).unwrap()).unwrap();
        assert_eq!(json["kind"], "remedy");
        assert_eq!(json["remedy"]["message"], "m");
    }

    // ── HealthPayload field names (camelCase via serde) ──

    fn mock_ok_payload() -> HealthPayload {
        let mk = |id: &str, label: &str, s: ComponentStatus| Component {
            id: id.into(),
            label: label.into(),
            note: None,
            status: s,
            version: None,
            detail: None,
            installing_verb: "installing".into(),
        };
        HealthPayload {
            version: "0.0.0-test".into(),
            uptime_seconds: 42,
            db_name: "sensei_test".into(),
            platform: Platform::Macos,
            package_manager: mk("homebrew", "Homebrew", ComponentStatus::Ready),
            components: vec![
                mk("postgres", "PostgreSQL", ComponentStatus::Ready),
                mk("ollama", "Ollama", ComponentStatus::Ready),
                mk("sensei", "Sensei components", ComponentStatus::Ready),
                mk("database", "Database & schema", ComponentStatus::Ready),
                mk("daemon", "Background daemon", ComponentStatus::Ready),
            ],
            status: HealthStatus::Ok,
            remedy: None,
            daemon_db_mode: None,
        }
    }

    #[test]
    fn payload_serializes_with_camel_case_field_names() {
        let p = mock_ok_payload();
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(json["uptimeSeconds"], 42);
        assert_eq!(json["packageManager"]["id"], "homebrew");
        assert!(
            json.get("uptime_seconds").is_none(),
            "snake_case must NOT appear"
        );
    }

    #[test]
    fn daemon_db_mode_omitted_when_none_and_camel_case_when_set() {
        let mut p = mock_ok_payload();
        // None → field absent from the wire (skip_serializing_if).
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert!(json.get("daemonDbMode").is_none(), "None must be omitted");

        // Some(Degraded) → camelCase key, lowercase variant.
        p.daemon_db_mode = Some(DaemonDbMode::Degraded);
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(json["daemonDbMode"], "degraded");
    }

    // ── validate() invariants ──

    #[test]
    fn validate_ok_payload_passes() {
        assert!(mock_ok_payload().validate().is_ok());
    }

    #[test]
    fn validate_needs_action_without_remedy_fails() {
        let mut p = mock_ok_payload();
        p.status = HealthStatus::NeedsAction;
        p.remedy = None;
        let err = p.validate().unwrap_err();
        assert!(err.contains("needs-action requires a remedy"));
    }

    #[test]
    fn validate_non_needs_action_with_remedy_fails() {
        let mut p = mock_ok_payload();
        p.remedy = Some(Remedy {
            message: "x".into(),
            script: "y".into(),
            url: None,
        });
        let err = p.validate().unwrap_err();
        assert!(err.contains("must not carry a remedy"));
    }

    #[test]
    fn validate_components_length_must_be_five() {
        let mut p = mock_ok_payload();
        p.components.pop();
        assert!(p
            .validate()
            .unwrap_err()
            .contains("expected 5 components, got 4"));
    }

    #[test]
    fn validate_components_must_be_in_canonical_order() {
        let mut p = mock_ok_payload();
        p.components.swap(0, 1);
        assert!(p
            .validate()
            .unwrap_err()
            .contains("components[0].id must be \"postgres\""));
    }

    #[test]
    fn validate_macos_must_use_homebrew() {
        let mut p = mock_ok_payload();
        p.package_manager.id = "winget".into();
        assert!(p
            .validate()
            .unwrap_err()
            .contains("packageManager.id=\"homebrew\""));
    }

    #[test]
    fn validate_windows_must_use_winget() {
        let mut p = mock_ok_payload();
        p.platform = Platform::Windows;
        p.package_manager.id = "homebrew".into();
        assert!(p
            .validate()
            .unwrap_err()
            .contains("packageManager.id=\"winget\""));
    }
}
