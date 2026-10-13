//! Boot-time check of the indexer's compiler engines (sensei-bootstrap's
//! `engines`): the missing ones are installed in the background, and the
//! daemon starts regardless of the outcome.

use sensei_bootstrap::engines::{self, EngineStatus, Ensured};

/// Whether boot installs the engines that are missing. Never on an isolated
/// instance: the engines are machine-wide, outside the instance's data dir.
fn installs_engines(isolated_instance: bool) -> bool {
    !isolated_instance
}

/// Check the engines off the async runtime; install what is missing; log it.
pub fn spawn() {
    let isolated = sensei_bootstrap::SenseiConfig::from_env().is_isolated_instance();
    tokio::spawn(async move {
        let work = tokio::task::spawn_blocking(move || {
            if installs_engines(isolated) {
                log_ensured(&engines::ensure());
            } else {
                for status in engines::check().iter().filter(|s| !s.is_ready()) {
                    tracing::warn!(
                        engine = status.engine.label(),
                        causes = %causes(status),
                        "engines: isolated instance — not installing a missing engine"
                    );
                }
            }
        });
        if let Err(e) = work.await {
            tracing::warn!(error = %e, "engines: the boot check did not finish");
        }
    });
}

fn causes(status: &EngineStatus) -> String {
    status.missing.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ")
}

fn log_ensured(ensured: &Ensured) {
    if ensured.installed.is_empty() {
        tracing::info!("engines: every indexing engine is ready");
        return;
    }
    for failure in &ensured.failures {
        tracing::warn!(
            engine = failure.engine.label(),
            cause = %failure.cause,
            "engines: install failed"
        );
    }
    for status in &ensured.after {
        if status.is_ready() {
            tracing::info!(engine = status.engine.label(), "engines: ready");
        } else {
            tracing::warn!(
                engine = status.engine.label(),
                causes = %causes(status),
                "engines: still missing after install — its languages' calls are not placed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_real_install_installs_missing_engines_an_instance_does_not() {
        assert!(installs_engines(false));
        assert!(!installs_engines(true));
    }
}
