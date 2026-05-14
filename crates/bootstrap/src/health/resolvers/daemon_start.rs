//! DaemonStartResolver — runs `senseid start` to bring the daemon up.

use std::process::Command;
use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::types::{ComponentId, Remedy};

pub struct DaemonStartResolver;

impl Resolver for DaemonStartResolver {
    fn id(&self) -> &'static str { "daemon_start" }
    fn resolves(&self) -> &'static [ComponentId] { &[ComponentId::Daemon] }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        let senseid = match crate::util::which_binary("senseid") {
            Some(p) => p,
            None    => return ResolveOutcome::NeedsHumanAction(missing_remedy()),
        };
        let status = Command::new(senseid).args(["start"]).status();
        match status {
            Ok(s) if s.success() => ResolveOutcome::Resolved,
            Ok(s)  => ResolveOutcome::NeedsHumanAction(
                         failed_remedy(format!("senseid start exited {s}"))),
            Err(e) => ResolveOutcome::NeedsHumanAction(failed_remedy(format!("senseid: {e}"))),
        }
    }
}

fn missing_remedy() -> Remedy {
    Remedy {
        message: "The `senseid` binary isn't installed. Run brew bundle to install all sensei binaries.".to_string(),
        script:  "brew bundle --file==https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile".to_string(),
        url:     None,
    }
}

fn failed_remedy(detail: String) -> Remedy {
    Remedy {
        message: format!("Couldn't start the daemon automatically ({detail}). Run it yourself."),
        script:  "senseid start".to_string(),
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
}
