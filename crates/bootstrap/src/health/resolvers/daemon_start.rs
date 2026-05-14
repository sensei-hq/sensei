//! DaemonStartResolver — runs the senseid daemon to bring it up. The
//! binary name is mode-aware via [`SenseiConfig::senseid_binary`] —
//! `senseid` in prod, `senseid-dev` in dev.

use std::process::Command;
use crate::config::SenseiConfig;
use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::types::{ComponentId, Remedy};

pub struct DaemonStartResolver;

impl Resolver for DaemonStartResolver {
    fn id(&self) -> &'static str { "daemon_start" }
    fn resolves(&self) -> &'static [ComponentId] { &[ComponentId::Daemon] }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        let bin = SenseiConfig::from_env().senseid_binary();
        let senseid = match crate::util::which_binary(bin) {
            Some(p) => p,
            None    => return ResolveOutcome::NeedsHumanAction(missing_remedy(bin)),
        };
        let status = Command::new(senseid).args(["start"]).status();
        match status {
            Ok(s) if s.success() => ResolveOutcome::Resolved,
            Ok(s)  => ResolveOutcome::NeedsHumanAction(
                         failed_remedy(bin, format!("{bin} start exited {s}"))),
            Err(e) => ResolveOutcome::NeedsHumanAction(
                         failed_remedy(bin, format!("{bin}: {e}"))),
        }
    }
}

fn missing_remedy(bin: &str) -> Remedy {
    Remedy {
        message: format!("The `{bin}` binary isn't installed. Run the script below to install all sensei binaries."),
        script:  SenseiConfig::from_env().brew_install_script(),
        url:     None,
    }
}

fn failed_remedy(bin: &str, detail: String) -> Remedy {
    Remedy {
        message: format!("Couldn't start the daemon automatically ({detail}). Run it yourself."),
        script:  format!("{bin} start"),
        url:     None,
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
    fn failed_remedy_script_uses_current_mode_binary() {
        let bin = SenseiConfig::from_env().senseid_binary();
        let r = failed_remedy(bin, "test".to_string());
        assert_eq!(r.script, format!("{bin} start"));
    }
}
