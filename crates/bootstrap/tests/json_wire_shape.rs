//! Lock the wire-shape contract.
//! These JSON shapes are what the TypeScript HealthState consumes.
//! If a field name or enum variant changes, these tests catch it.

use sensei_bootstrap::*;

fn mock_payload(status: HealthStatus, with_remedy: bool) -> HealthPayload {
    let mk = |id: &str, label: &str, note: Option<&str>, s: ComponentStatus| Component {
        id: id.to_string(),
        label: label.to_string(),
        note: note.map(str::to_string),
        status: s,
        version: None,
        detail: None,
    };
    HealthPayload {
        version:        "0.0.0-test".into(),
        uptime_seconds: 42,
        platform:       Platform::Macos,
        package_manager: mk("homebrew", "Homebrew", Some("which brew"), ComponentStatus::Ready),
        components: vec![
            mk("postgres", "PostgreSQL", None, ComponentStatus::Ready),
            mk("ollama",   "Ollama", None, ComponentStatus::Ready),
            mk("sensei",   "Sensei components", Some("cli · mcp · daemon"), ComponentStatus::Ready),
            mk("database", "Database & schema", Some("pgvector · sensei tables"), ComponentStatus::Ready),
            mk("daemon",   "Background daemon", None, ComponentStatus::Ready),
        ],
        status,
        remedy: if with_remedy {
            Some(Remedy { message: "msg".into(), script: "cmd".into(), url: None })
        } else { None },
    }
}

#[test]
fn ok_payload_serializes_to_expected_json() {
    let p = mock_payload(HealthStatus::Ok, false);
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["remedy"], serde_json::Value::Null);
    assert_eq!(json["platform"], "macos");
    assert_eq!(json["packageManager"]["id"], "homebrew");
    assert_eq!(json["packageManager"]["label"], "Homebrew");
    assert_eq!(json["packageManager"]["note"], "which brew");
    assert_eq!(json["packageManager"]["status"], "ready");
    assert_eq!(json["components"][0]["id"], "postgres");
    assert_eq!(json["components"][0]["label"], "PostgreSQL");
    assert_eq!(json["components"][2]["note"], "cli · mcp · daemon");
    assert_eq!(json["version"], "0.0.0-test");
    assert_eq!(json["uptimeSeconds"], 42);
    // CRITICAL: NEVER snake_case at the top level.
    assert!(json.get("uptime_seconds").is_none(), "must use camelCase 'uptimeSeconds'");
    assert!(json.get("package_manager").is_none(), "must use camelCase 'packageManager'");
}

#[test]
fn needs_action_payload_serializes_with_remedy() {
    let p = mock_payload(HealthStatus::NeedsAction, true);
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(json["status"], "needs-action");
    assert_eq!(json["remedy"]["message"], "msg");
    assert_eq!(json["remedy"]["script"], "cmd");
    assert_eq!(json["remedy"]["url"], serde_json::Value::Null);
}

#[test]
fn checking_status_serializes_lowercase() {
    let p = mock_payload(HealthStatus::Checking, false);
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(json["status"], "checking");
}

#[test]
fn resolving_status_serializes_lowercase() {
    let p = mock_payload(HealthStatus::Resolving, false);
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(json["status"], "resolving");
}

#[test]
fn windows_platform_with_winget_pm() {
    let mut p = mock_payload(HealthStatus::Ok, false);
    p.platform = Platform::Windows;
    p.package_manager.id = "winget".into();
    p.package_manager.label = "winget".into();
    p.package_manager.note = Some("winget --version".into());
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(json["platform"], "windows");
    assert_eq!(json["packageManager"]["id"], "winget");
}

#[test]
fn health_event_phase_serializes_correctly() {
    let ev = HealthEvent::Phase { phase: HealthStatus::Resolving };
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&ev).unwrap()).unwrap();
    assert_eq!(json["kind"],  "phase");
    assert_eq!(json["phase"], "resolving");
}

#[test]
fn health_event_component_patch_serializes_correctly() {
    let ev = HealthEvent::Component {
        id: "postgres".to_string(),
        patch: ComponentPatch {
            status: Some(ComponentStatus::Installing),
            ..Default::default()
        },
    };
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&ev).unwrap()).unwrap();
    assert_eq!(json["kind"], "component");
    assert_eq!(json["id"],   "postgres");
    assert_eq!(json["patch"]["status"], "installing");
    // Optional fields skip when None — so label/note/version/detail must be absent.
    assert!(json["patch"].get("label").is_none());
    assert!(json["patch"].get("note").is_none());
}

#[test]
fn health_event_remedy_serializes_correctly() {
    let ev = HealthEvent::Remedy {
        remedy: Remedy { message: "m".into(), script: "s".into(), url: Some("https://x".into()) },
    };
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&ev).unwrap()).unwrap();
    assert_eq!(json["kind"], "remedy");
    assert_eq!(json["remedy"]["message"], "m");
    assert_eq!(json["remedy"]["url"],     "https://x");
}

#[test]
fn health_event_report_carries_full_payload() {
    let p = mock_payload(HealthStatus::Ok, false);
    let ev = HealthEvent::Report { payload: p };
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&ev).unwrap()).unwrap();
    assert_eq!(json["kind"], "report");
    assert_eq!(json["payload"]["status"], "ok");
    assert_eq!(json["payload"]["packageManager"]["id"], "homebrew");
}

#[test]
fn health_event_round_trips_via_serde() {
    let original = HealthEvent::Component {
        id: "postgres".to_string(),
        patch: ComponentPatch {
            status:  Some(ComponentStatus::Ready),
            version: Some(Some("17.2".into())),
            detail:  Some(None), // double-Option: explicitly set to JSON null
            ..Default::default()
        },
    };
    let s = serde_json::to_string(&original).unwrap();
    let parsed: HealthEvent = serde_json::from_str(&s).unwrap();
    match parsed {
        HealthEvent::Component { id, patch } => {
            assert_eq!(id, "postgres");
            assert!(matches!(patch.status, Some(ComponentStatus::Ready)));
        }
        _ => panic!("expected Component event"),
    }
}
