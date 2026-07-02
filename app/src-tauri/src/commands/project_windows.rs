//! Project-window Tauri commands.
//!
//! Each project can open in its own dedicated window so a user can compare
//! multiple projects side-by-side. Windows are labelled `project-{id}` so
//! a subsequent `open_project_window(id)` for the same id focuses the
//! existing window rather than opening a duplicate.
//!
//! The frontend reads the `project_id` from the URL (route param) so this
//! command is the only piece of state passed at open time.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Open (or focus) a window for a specific project. Idempotent — repeat
/// calls with the same `project_id` focus the existing window.
///
/// The window loads `/projects/{project_id}/overview` so the sidebar shell
/// hydrates against the routed project directly.
#[tauri::command]
pub fn open_project_window(app: AppHandle, project_id: String) -> Result<(), String> {
    if project_id.trim().is_empty() {
        return Err("project_id required".into());
    }

    let label = window_label(&project_id);

    // Focus-existing: if a window for this project is already open, bring it
    // to the front instead of stacking a duplicate. Silence a set_focus
    // failure since it isn't fatal — the app is still usable.
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.set_focus();
        return Ok(());
    }

    // The project window's routing lives under `(project)/project/[id]/` —
    // load `/project/{id}/overview` so the sidebar shell hydrates the
    // requested project directly.
    let route = format!("/project/{project_id}/overview");
    let url = WebviewUrl::App(route.into());
    WebviewWindowBuilder::new(&app, label, url)
        .title(format!("Sensei · {project_id}"))
        .inner_size(1200.0, 780.0)
        .min_inner_size(900.0, 600.0)
        .build()
        .map_err(|e| format!("failed to open project window: {e}"))?;
    Ok(())
}

/// Deterministic window label — `project-{id}` — used for focus-existing
/// dedup and by tests that assert against the label.
fn window_label(project_id: &str) -> String {
    format!("project-{project_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_is_project_id_prefixed() {
        assert_eq!(window_label("abc-123"), "project-abc-123");
    }

    #[test]
    fn label_preserves_empty_ish_but_command_rejects_it() {
        // `window_label` is deterministic — the guard against empty input
        // lives in the command entry, not the label helper.
        assert_eq!(window_label(""), "project-");
    }
}
