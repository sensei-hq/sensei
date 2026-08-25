//! Cross-platform desktop notifications for the daemon, behind a trait so unit
//! tests never fire a real OS notification. NOTE: the module is `notifications`
//! (not `notify`) because the `notify` crate name is taken by the fs-watcher.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyLevel {
    Info,
    Warn,
    Critical,
}

pub trait Notifier: Send + Sync {
    fn notify(&self, level: NotifyLevel, title: &str, body: &str);
}

/// Production notifier backed by notify-rust. Failures are logged, never
/// propagated — a missing notification daemon must not break the watchdog.
pub struct DesktopNotifier;

impl Notifier for DesktopNotifier {
    fn notify(&self, level: NotifyLevel, title: &str, body: &str) {
        let prefix = match level {
            NotifyLevel::Info => "✓",
            NotifyLevel::Warn => "⚠",
            NotifyLevel::Critical => "✗",
        };
        let result = notify_rust::Notification::new()
            .summary(&format!("{prefix} sensei: {title}"))
            .body(body)
            .appname("sensei")
            .show();
        match result {
            Ok(_) => tracing::info!(level = ?level, %title, "desktop notification shown"),
            Err(e) => {
                tracing::warn!(level = ?level, %title, error = %e, "desktop notification failed")
            }
        }
    }
}

/// No-op notifier for tests.
pub struct NoopNotifier;
impl Notifier for NoopNotifier {
    fn notify(&self, _l: NotifyLevel, _t: &str, _b: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Records calls so watchdog tests can assert on what was notified.
    pub struct RecordingNotifier(pub Mutex<Vec<(NotifyLevel, String)>>);
    impl Notifier for RecordingNotifier {
        fn notify(&self, level: NotifyLevel, title: &str, _body: &str) {
            self.0.lock().unwrap().push((level, title.to_string()));
        }
    }

    #[test]
    fn noop_notifier_does_nothing() {
        NoopNotifier.notify(NotifyLevel::Info, "t", "b"); // must not panic
    }

    #[test]
    fn recording_notifier_captures_calls() {
        let r = RecordingNotifier(Mutex::new(vec![]));
        r.notify(NotifyLevel::Critical, "boom", "body");
        assert_eq!(r.0.lock().unwrap()[0], (NotifyLevel::Critical, "boom".to_string()));
    }
}
