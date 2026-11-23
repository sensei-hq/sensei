//! The editor a session runs in, from its environment (#512).
//!
//! One table, shared: the MCP binary's `detect_host` (client name first,
//! then env) and the daemon's hook fold (env only) map through this, so the
//! mapping cannot drift between the two. Detection never guesses: anything
//! the table does not name reads as none.

/// The editor `$TERM_PROGRAM` names, with `$CURSOR_TRACE_ID` disambiguating
/// Cursor's `vscode`. `cursor_trace_set` is presence only — the value never
/// leaves the machine. `None` for a blank/absent program and for anything the
/// table does not name (`Apple_Terminal` included): detection never guesses.
pub fn detect_host_from_env(
    term_program: Option<&str>,
    cursor_trace_set: bool,
) -> Option<&'static str> {
    match term_program.map(str::trim).filter(|v| !v.is_empty()) {
        Some("zed") => Some("zed"),
        Some("vscode") if cursor_trace_set => Some("cursor"),
        Some("vscode") => Some("vscode"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn term_program_maps_to_its_editor_and_nothing_else() {
        assert_eq!(detect_host_from_env(Some("zed"), false), Some("zed"));
        assert_eq!(detect_host_from_env(Some("vscode"), true), Some("cursor"));
        assert_eq!(detect_host_from_env(Some("vscode"), false), Some("vscode"));
        assert_eq!(detect_host_from_env(Some("Apple_Terminal"), false), None);
        assert_eq!(detect_host_from_env(Some("  "), false), None);
        assert_eq!(detect_host_from_env(None, false), None);
    }
}
