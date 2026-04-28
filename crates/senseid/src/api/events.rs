//! StateEvent types for SSE — matches the app's StateEvent<T> interface.
//!
//! Single event shape for all SSE streams. Entity field routes to the right
//! state context on the client side.

use serde::Serialize;

/// Generic state event matching the app's StateEvent<T> interface.
#[derive(Debug, Clone, Serialize)]
pub struct StateEvent {
    pub action: Action,
    pub entity: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Add,
    Update,
    Remove,
    Set,
}

// ── Project entity ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ScanProject {
    pub id: String,
    pub name: String,
    pub status: ProjectStatus,
    pub folders: Vec<ScanProjectFolder>,
    #[serde(rename = "autoDetected")]
    pub auto_detected: bool,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Scanning,
    Indexing,
    Active,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanProjectFolder {
    pub id: String,
    pub name: String,
    pub path: String,
    pub stack: Vec<String>,
    #[serde(rename = "filesTotal")]
    pub files_total: u32,
    #[serde(rename = "filesCompleted")]
    pub files_completed: u32,
    pub status: FolderStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderStatus {
    Discovered,
    Queued,
    Indexing,
    Indexed,
    Failed,
}

// ── Activity entity ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ActivityEvent {
    pub id: String,
    pub level: ActivityLevel,
    pub message: String,
    pub elapsed: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityLevel {
    Discover,
    Queue,
    Process,
    Info,
    Success,
    Error,
}

// ── Constructors ────────────────────────────────────────────────────────────

impl StateEvent {
    pub fn project_add(project: ScanProject) -> Self {
        Self {
            action: Action::Add,
            entity: "project".into(),
            data: serde_json::to_value(&project).unwrap(),
        }
    }

    pub fn project_update(project: ScanProject) -> Self {
        Self {
            action: Action::Update,
            entity: "project".into(),
            data: serde_json::to_value(&project).unwrap(),
        }
    }

    pub fn project_remove(id: &str) -> Self {
        Self {
            action: Action::Remove,
            entity: "project".into(),
            data: serde_json::json!({ "id": id }),
        }
    }

    pub fn activity(event: ActivityEvent) -> Self {
        Self {
            action: Action::Add,
            entity: "activity".into(),
            data: serde_json::to_value(&event).unwrap(),
        }
    }
}

impl ActivityEvent {
    pub fn new(level: ActivityLevel, message: &str, elapsed: f64) -> Self {
        Self {
            id: format!("a{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
            level,
            message: message.to_string(),
            elapsed,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_event_serializes() {
        let evt = ActivityEvent::new(ActivityLevel::Discover, "~/code · found root", 0.12);
        let json = serde_json::to_value(&evt).unwrap();
        assert_eq!(json["level"], "discover");
        assert_eq!(json["message"], "~/code · found root");
        assert!(json["elapsed"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn state_event_activity_format() {
        let evt = StateEvent::activity(
            ActivityEvent::new(ActivityLevel::Queue, "app · 842 files queued", 0.38),
        );
        let json = serde_json::to_value(&evt).unwrap();
        assert_eq!(json["action"], "add");
        assert_eq!(json["entity"], "activity");
        assert_eq!(json["data"]["level"], "queue");
    }

    #[test]
    fn state_event_project_add() {
        let proj = ScanProject {
            id: "p1".into(),
            name: "Lumen".into(),
            status: ProjectStatus::Scanning,
            folders: vec![
                ScanProjectFolder {
                    id: "f1".into(), name: "app".into(), path: "/code/app".into(),
                    stack: vec!["typescript".into()], files_total: 842, files_completed: 0,
                    status: FolderStatus::Discovered,
                },
            ],
            auto_detected: true,
            confidence: Confidence::High,
        };
        let evt = StateEvent::project_add(proj);
        let json = serde_json::to_value(&evt).unwrap();
        assert_eq!(json["action"], "add");
        assert_eq!(json["entity"], "project");
        assert_eq!(json["data"]["name"], "Lumen");
        assert_eq!(json["data"]["folders"][0]["filesTotal"], 842);
        assert_eq!(json["data"]["autoDetected"], true);
    }

    #[test]
    fn state_event_project_remove() {
        let evt = StateEvent::project_remove("p1");
        let json = serde_json::to_value(&evt).unwrap();
        assert_eq!(json["action"], "remove");
        assert_eq!(json["entity"], "project");
        assert_eq!(json["data"]["id"], "p1");
    }

    #[test]
    fn folder_status_serializes_snake_case() {
        let json = serde_json::to_value(FolderStatus::Indexed).unwrap();
        assert_eq!(json, "indexed");
    }

    #[test]
    fn project_status_serializes_snake_case() {
        let json = serde_json::to_value(ProjectStatus::Active).unwrap();
        assert_eq!(json, "active");
    }

    #[tokio::test]
    async fn broadcast_channel_delivers_events() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<StateEvent>(64);

        tx.send(StateEvent::activity(
            ActivityEvent::new(ActivityLevel::Discover, "~/code · found root", 0.12),
        )).unwrap();
        tx.send(StateEvent::activity(
            ActivityEvent::new(ActivityLevel::Discover, "~/code/app · found git repo", 0.18),
        )).unwrap();
        tx.send(StateEvent::project_add(ScanProject {
            id: "p1".into(), name: "test".into(), status: ProjectStatus::Scanning,
            folders: vec![ScanProjectFolder {
                id: "f1".into(), name: "app".into(), path: "/code/app".into(),
                stack: vec!["rust".into()], files_total: 5, files_completed: 0,
                status: FolderStatus::Discovered,
            }],
            auto_detected: true, confidence: Confidence::High,
        })).unwrap();
        tx.send(StateEvent::activity(
            ActivityEvent::new(ActivityLevel::Success, "scan complete", 0.50),
        )).unwrap();

        // Drain and validate
        let mut events = vec![];
        while let Ok(evt) = rx.try_recv() {
            events.push(evt);
        }

        assert_eq!(events.len(), 4);

        // Activity events
        let activities: Vec<_> = events.iter().filter(|e| e.entity == "activity").collect();
        assert_eq!(activities.len(), 3);
        assert_eq!(activities[0].data["level"], "discover");
        assert_eq!(activities[2].data["level"], "success");

        // Project events
        let projects: Vec<_> = events.iter().filter(|e| e.entity == "project").collect();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].data["name"], "test");
        assert_eq!(projects[0].data["folders"][0]["stack"][0], "rust");
    }

    #[tokio::test]
    async fn events_write_to_jsonl() {
        let events = vec![
            StateEvent::activity(ActivityEvent::new(ActivityLevel::Discover, "found root", 0.1)),
            StateEvent::activity(ActivityEvent::new(ActivityLevel::Queue, "app · 5 files", 0.2)),
            StateEvent::project_add(ScanProject {
                id: "p1".into(), name: "proj".into(), status: ProjectStatus::Active,
                folders: vec![], auto_detected: true, confidence: Confidence::High,
            }),
            StateEvent::activity(ActivityEvent::new(ActivityLevel::Success, "done", 0.5)),
        ];

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut content = String::new();
        for evt in &events {
            content.push_str(&serde_json::to_string(evt).unwrap());
            content.push('\n');
        }
        std::fs::write(tmp.path(), &content).unwrap();

        // Read back and validate
        let lines: Vec<&str> = content.trim().lines().collect();
        assert_eq!(lines.len(), 4);

        for line in &lines {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(parsed["action"].is_string());
            assert!(parsed["entity"].is_string());
            assert!(parsed["data"].is_object());
        }
    }
}
