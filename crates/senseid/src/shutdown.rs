//! The signals that mean "stop serving".
//!
//! SIGINT alone is not enough: it only arrives when a human presses Ctrl-C in
//! a foreground terminal, and the daemon is never run that way in anger. Both
//! real stop paths reach SIGTERM — `brew services stop sensei` leaves it to
//! launchd, and `stop_daemon` shells out to `kill` with the pid from
//! `serve.pid` AFTER first trying `POST /stop` and finding no handler for it.

use tokio::signal::unix::{SignalKind, signal};

/// Completes when the process is asked to stop, by SIGINT or SIGTERM,
/// whichever arrives first.
///
/// If a handler cannot be registered the daemon keeps serving — losing
/// graceful shutdown is not a reason to refuse to start — but that arm then
/// waits forever instead of completing, because a handler we failed to
/// install must never be mistaken for a signal that arrived: completing would
/// tear the server down the moment it came up.
pub async fn shutdown_signal() {
    tokio::select! {
        _ = wait_for(SignalKind::interrupt(), "SIGINT") => {}
        _ = wait_for(SignalKind::terminate(), "SIGTERM") => {}
    }
}

/// Completes once `kind` is delivered; otherwise never.
async fn wait_for(kind: SignalKind, name: &'static str) {
    match signal(kind) {
        Ok(mut stream) => {
            if stream.recv().await.is_some() {
                tracing::info!(signal = name, "shutdown signal received");
                return;
            }
            // `recv()` yielding None means the runtime's signal driver is
            // gone. Parking is the conservative read: a closed stream is not a
            // signal, and completing here would tear the server down for a
            // reason nobody asked for.
            tracing::warn!(signal = name, "signal stream closed before any signal arrived");
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                signal = name,
                "could not register shutdown handler — this signal will kill the daemon outright instead of draining"
            );
        }
    }
    std::future::pending::<()>().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::signal::unix::{SignalKind, signal};

    /// Send a signal to our own process the way `sensei stop` does — this
    /// crate has no `libc` dependency, and `kill(1)` is the same path
    /// `stop_daemon` takes to reach a running daemon.
    fn raise(flag: &str) {
        let status = std::process::Command::new("kill")
            .arg(flag)
            .arg(std::process::id().to_string())
            .status()
            .expect("kill(1) must be on PATH");
        assert!(status.success(), "kill {flag} failed");
    }

    /// Drive one signal end to end: raise it until `shutdown_signal()` has
    /// completed, or give up.
    ///
    /// `shutdown_signal()` registers its handlers on its first poll, so a
    /// signal raised before the spawned task is scheduled is simply missed —
    /// hence re-raising rather than firing once and hoping.
    async fn completes_on(flag: &str) -> bool {
        let task = tokio::spawn(shutdown_signal());
        tokio::time::timeout(Duration::from_secs(5), async {
            while !task.is_finished() {
                raise(flag);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .is_ok()
    }

    /// Both phases live in one test on purpose: run as two tests they could
    /// overlap, and then the SIGTERM one would satisfy the SIGINT one's future
    /// as well — a green that survives deleting the SIGINT arm entirely.
    #[tokio::test]
    async fn completes_on_sigterm_and_on_sigint() {
        // Install the OS-level handlers before anything is raised. The default
        // disposition of both signals is to kill the process, which would take
        // the whole test binary down in the window before `shutdown_signal()`
        // registers its own.
        let _term = signal(SignalKind::terminate()).expect("register SIGTERM");
        let _int = signal(SignalKind::interrupt()).expect("register SIGINT");

        assert!(
            completes_on("-TERM").await,
            "shutdown_signal() never completed under SIGTERM — this is issue #212: \
             `sensei stop` and launchd both send SIGTERM, not SIGINT"
        );
        assert!(
            completes_on("-INT").await,
            "shutdown_signal() never completed under SIGINT — Ctrl-C in a foreground \
             terminal would no longer shut the daemon down gracefully"
        );
    }

    /// SIGKILL cannot be caught, so `signal()` refuses it — the one handle we
    /// have on the registration-failure branch without mocking the OS.
    #[tokio::test]
    async fn an_unregisterable_handler_waits_instead_of_completing() {
        const SIGKILL: i32 = 9;
        let uncatchable = SignalKind::from_raw(SIGKILL);
        assert!(
            signal(uncatchable).is_err(),
            "SIGKILL registered successfully — this test is no longer exercising the failure branch"
        );

        let outcome =
            tokio::time::timeout(Duration::from_millis(200), wait_for(uncatchable, "SIGKILL"))
                .await;

        assert!(
            outcome.is_err(),
            "wait_for() completed on a handler it never installed — the daemon would shut \
             itself down the instant it finished starting up"
        );
    }

    #[test]
    fn the_future_is_send_and_static() {
        // `axum::serve(..).with_graceful_shutdown(f)` requires exactly this of
        // `f`; a non-Send future here would only surface at the call site.
        fn requires<F: std::future::Future<Output = ()> + Send + 'static>(_: F) {}
        requires(shutdown_signal());
    }
}
