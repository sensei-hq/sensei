//! DaemonStartResolver — bring up the senseid daemon. The brew-services
//! path is preferred (mirrors Postgres/Ollama and gets launchd's keep-alive
//! restart on crash); direct binary spawn is the fallback when brew isn't
//! available.

use crate::config::SenseiConfig;
use crate::health::resolver::{ResolveOutcome, Resolver};
use crate::health::resolvers::brew_helpers::{BrewError, brew_services_start};
use crate::health::types::{ComponentId, Remedy};
use std::process::Command;

pub struct DaemonStartResolver;

impl Resolver for DaemonStartResolver {
    fn id(&self) -> &'static str {
        "daemon_start"
    }
    fn resolves(&self) -> &'static [ComponentId] {
        &[ComponentId::Daemon]
    }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        let cfg = SenseiConfig::from_env();

        // Instance isolation: when SENSEI_INSTANCE is set (e2e tests), skip
        // brew services entirely. The brew-services launchd shim doesn't
        // inherit the caller's environment, so a daemon brought up via brew
        // would ignore SENSEI_INSTANCE and write to the default DB/dir —
        // defeating the point of isolation. Direct spawn inherits this
        // process's env, so the daemon honours the instance.
        let instance_set = std::env::var_os("SENSEI_INSTANCE").is_some_and(|v| !v.is_empty());

        if !instance_set {
            // Stage 1: brew services start sensei. Matches the postgres/ollama
            // resolver path. Gains launchd auto-restart so the daemon comes back
            // after a crash without the user noticing.
            let service = cfg.brew_service_name();
            tracing::info!(service, "stage 1: brew services start");
            match brew_services_start(service) {
                Ok(()) => {
                    tracing::info!(service, "stage 1: brew services start ok");
                    return ResolveOutcome::Resolved;
                }
                Err(BrewError::BrewNotFound) => {
                    tracing::warn!("brew not on PATH — falling back to direct daemon spawn");
                    // Fall through to stage 2.
                }
                Err(e) => {
                    tracing::info!(service, error = ?e, "stage 1 failed — falling back to direct spawn");
                }
            }
        } else {
            tracing::info!(
                "SENSEI_INSTANCE set — skipping brew services (cannot propagate env), using direct spawn"
            );
        }

        // Stage 2: direct binary spawn. The daemon daemonises itself on
        // start, so `status()` returns once the parent forks — port
        // readiness is the orchestrator's post-resolve re-check problem.
        //
        // When SENSEI_INSTANCE is set we forward it BOTH via inherited
        // env AND as an explicit `--instance` CLI arg. Either alone
        // would be enough in theory, but the daemon's self-re-spawn
        // chain (Tauri → senseid start → daemonized senseid) has had
        // env-loss issues in practice; the CLI arg can't be stripped.
        let bin = cfg.senseid_binary();
        let senseid = match crate::util::which_binary(bin) {
            Some(p) => p,
            None => return ResolveOutcome::NeedsHumanAction(missing_remedy(bin)),
        };
        tracing::info!(bin, "stage 2: direct daemon spawn");
        let mut cmd = Command::new(senseid);
        cmd.arg("start");
        if let Ok(inst) = std::env::var("SENSEI_INSTANCE")
            && !inst.is_empty()
        {
            cmd.args(["--instance", &inst]);
            tracing::info!(instance = %inst, "stage 2: passing --instance to daemon");
        }
        match cmd.status() {
            Ok(s) if s.success() => ResolveOutcome::Resolved,
            Ok(s) => ResolveOutcome::NeedsHumanAction(failed_remedy(
                bin,
                format!("{bin} start exited {s}"),
            )),
            Err(e) => ResolveOutcome::NeedsHumanAction(failed_remedy(bin, format!("{bin}: {e}"))),
        }
    }

    fn fallback_remedy(&self) -> Remedy {
        let cfg = SenseiConfig::from_env();
        let bin = cfg.senseid_binary();
        let service = cfg.brew_service_name();
        Remedy {
            message: "Daemon port still not listening after start. Try the brew-services restart and check `brew services list` for the service status.".to_string(),
            script:  format!("brew services restart {service} && {bin} status"),
            url:     None,
        }
    }
}

fn missing_remedy(bin: &str) -> Remedy {
    Remedy {
        message: format!(
            "The `{bin}` binary isn't installed. Run the script below to install all sensei binaries."
        ),
        script: SenseiConfig::from_env().brew_install_script(),
        url: None,
    }
}

fn failed_remedy(bin: &str, detail: String) -> Remedy {
    let service = SenseiConfig::from_env().brew_service_name();
    Remedy {
        message: format!(
            "Couldn't start `{bin}` automatically ({detail}). Try the brew-services route below — it gets keep-alive auto-restart from launchd."
        ),
        script: format!("brew services start {service}"),
        url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_daemon_start() {
        assert_eq!(DaemonStartResolver.id(), "daemon_start");
    }

    #[test]
    fn covers_daemon_only() {
        assert_eq!(DaemonStartResolver.resolves(), &[ComponentId::Daemon]);
    }

    #[test]
    fn missing_remedy_names_current_mode_binary() {
        let bin = SenseiConfig::from_env().senseid_binary();
        let r = missing_remedy(bin);
        assert!(r.message.contains(bin), "message '{}' should mention '{bin}'", r.message);
    }

    #[test]
    fn failed_remedy_script_uses_brew_services_start() {
        let cfg = SenseiConfig::from_env();
        let service = cfg.brew_service_name();
        let r = failed_remedy(cfg.senseid_binary(), "test".to_string());
        assert_eq!(r.script, format!("brew services start {service}"));
    }

    #[test]
    fn fallback_remedy_runs_brew_services_restart() {
        let cfg = SenseiConfig::from_env();
        let service = cfg.brew_service_name();
        let r = DaemonStartResolver.fallback_remedy();
        assert!(
            r.script.contains(&format!("brew services restart {service}")),
            "expected fallback to use brew services restart, got: {}",
            r.script
        );
    }
}
