//! OllamaInstallResolver — resolves ComponentId::Ollama via
//! `brew install ollama` AND `brew services start ollama`. Install alone
//! installs the CLI but doesn't run the server; the PortChecker for 11434
//! would then keep failing.

use std::process::Command;

use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::resolvers::service_cascade::{resolve_service_cascade, ServiceCascadeSpec};
use crate::health::types::{ComponentId, Remedy};

pub struct OllamaInstallResolver;

const FORMULA: &str = "ollama";
const SERVICE: &str = "ollama";
const TARGETS: &[ComponentId] = &[ComponentId::Ollama];

/// `ollama serve` is the canonical foreground starter — works whether the
/// binary came from brew or a manual install. Stage 2 falls back to this
/// when brew doesn't recognize the service.
fn direct_launcher() -> Command {
    let mut c = Command::new(FORMULA);
    c.arg("serve");
    c
}

const SPEC: ServiceCascadeSpec = ServiceCascadeSpec {
    formula: FORMULA,
    service: SERVICE,
    install_args: &[],
    direct_launcher: Some(direct_launcher),
};

impl Resolver for OllamaInstallResolver {
    fn id(&self) -> &'static str { "ollama_install" }
    fn resolves(&self) -> &'static [ComponentId] { TARGETS }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        resolve_service_cascade(&SPEC)
    }

    fn fallback_remedy(&self) -> Remedy { ollama_fallback_remedy() }
}

/// Surfaces when `brew install ollama && brew services start ollama` both
/// returned exit 0 but the port still isn't responding by the time the
/// re-check runs. Restarting the service is the cleanest fix — it forces
/// launchd to relaunch the daemon and bind the port.
fn ollama_fallback_remedy() -> Remedy {
    Remedy {
        message: "Ollama installed but the server isn't responding on its port. Restart the brew service to bring it online.".to_string(),
        script: format!("brew services restart {SERVICE}"),
        url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_ollama_install() {
        assert_eq!(OllamaInstallResolver.id(), "ollama_install");
    }

    #[test]
    fn resolves_only_ollama() {
        assert_eq!(OllamaInstallResolver.resolves(), &[ComponentId::Ollama]);
    }

    #[test]
    fn does_not_cover_others() {
        let t = OllamaInstallResolver.resolves();
        assert!(!t.contains(&ComponentId::Postgres));
        assert!(!t.contains(&ComponentId::Sensei));
        assert!(!t.contains(&ComponentId::Database));
        assert!(!t.contains(&ComponentId::Daemon));
    }

    #[test]
    fn fallback_remedy_restarts_service() {
        let r = OllamaInstallResolver.fallback_remedy();
        assert_eq!(r.script, format!("brew services restart {SERVICE}"));
        assert!(r.message.to_lowercase().contains("ollama"));
    }
}
