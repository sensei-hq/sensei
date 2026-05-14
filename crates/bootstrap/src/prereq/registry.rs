//! Component registry — defines all 10 bootstrap components and their dependency graph.
//!
//! Each `ComponentSpec` carries checker/fixer factory functions that accept a `BuildContext`
//! and return boxed trait objects. The engine calls these at runtime to build the actual
//! checkers and fixers, allowing the engine to inject mocks via `BootstrapContext` overrides.

use std::sync::Arc;
use crate::platform::PlatformProvider;
use crate::config::SenseiConfig;
use super::checker::{BinaryChecker, Checker, PortChecker, VersionedBinaryChecker, DatabaseChecker};
use super::fixer::{BrewBundleFixer, DaemonFixer, DatabaseSetupFixer, Fixer, HumanActionFixer, NoopFixer, ServiceStartFixer};
use super::GateKind;
use super::ids;
use crate::{POSTGRES_PORT, OLLAMA_PORT};
use crate::config::HOMEBREW_BREWFILE_URL;

/// Lightweight context passed to checker_fn / fixer_fn when building components.
/// Does NOT contain test-injection maps — those live in BootstrapContext (engine.rs).
pub struct BuildContext<'a> {
    pub platform:    &'a Arc<dyn PlatformProvider>,
    pub config:      &'a SenseiConfig,
    pub app_version: &'a str,
}

/// A single bootstrap component — id, dependency edges, and factory functions.
pub struct ComponentSpec {
    pub id:               &'static str,
    pub label:            &'static str,
    pub depends_on:       &'static [&'static str],
    /// If Some, component participates in a BrewBundleFixer batch with others sharing the same group.
    pub fix_group:        Option<&'static str>,
    /// After this component is successfully fixed, force these ids into the fix plan.
    pub post_fix_trigger: &'static [&'static str],
    pub gate_kind:        GateKind,
    pub checker_fn:       fn(&BuildContext) -> Box<dyn Checker>,
    pub fixer_fn:         fn(&BuildContext) -> Box<dyn Fixer>,
}

fn detect_brew_path() -> Option<String> {
    crate::config::BREW_PATHS
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|s| s.to_string())
}

/// Returns a `BrewBundleFixer` when Homebrew is found, or a `NoopFixer` otherwise.
fn brew_bundle_fixer() -> Box<dyn Fixer> {
    match detect_brew_path() {
        Some(brew) => Box::new(BrewBundleFixer::new(brew, HOMEBREW_BREWFILE_URL)),
        None       => Box::new(NoopFixer::new("Homebrew not found")),
    }
}

/// Returns the standard fixer for a Sensei binary component:
/// - Dev mode: `HumanActionFixer` (run `make install-dev`)
/// - Prod mode: `BrewBundleFixer` (or `NoopFixer` if brew not found)
fn sensei_binary_fixer(component_id: &'static str, ctx: &BuildContext) -> Box<dyn Fixer> {
    if ctx.config.is_dev() {
        Box::new(HumanActionFixer {
            component_id,
            title:   "Build dev binaries",
            command: "make install-dev",
            url:     None,
        })
    } else {
        brew_bundle_fixer()
    }
}

/// Homebrew checker that delegates to PlatformProvider::check_package_manager.
struct HomebrewChecker(Arc<dyn PlatformProvider>);
impl Checker for HomebrewChecker {
    fn check(&self) -> super::CheckResult {
        let status = self.0.check_package_manager();
        if status.is_ready() {
            super::CheckResult::ok(status.version.as_deref().unwrap_or("unknown"))
        } else {
            super::CheckResult::fail(match &status.state {
                crate::types::ComponentState::Failed { error } => error.clone(),
                _ => "homebrew not found".to_string(),
            })
        }
    }
}

pub static COMPONENTS: &[ComponentSpec] = &[
    // ── Homebrew ─────────────────────────────────────────────────────────────
    ComponentSpec {
        id:               ids::HOMEBREW,
        label:            "Homebrew",
        depends_on:       &[],
        fix_group:        None,
        post_fix_trigger: &[],
        gate_kind:        GateKind::Install,
        checker_fn: |ctx| Box::new(HomebrewChecker(Arc::clone(ctx.platform))),
        fixer_fn:   |_ctx| Box::new(HumanActionFixer {
            component_id: ids::HOMEBREW,
            title:        "Install Homebrew",
            command:      "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"",
            url:          Some("https://brew.sh"),
        }),
    },
    // ── PostgreSQL (binary) ──────────────────────────────────────────────────
    ComponentSpec {
        id:               ids::POSTGRESQL,
        label:            "PostgreSQL",
        depends_on:       &[ids::HOMEBREW],
        fix_group:        Some("bundle"),
        post_fix_trigger: &[],
        gate_kind:        GateKind::Install,
        checker_fn: |_ctx| Box::new(BinaryChecker::new("postgres", "--version")),
        fixer_fn:   |_ctx| brew_bundle_fixer(),
    },
    // ── Ollama (binary) ──────────────────────────────────────────────────────
    ComponentSpec {
        id:               ids::OLLAMA,
        label:            "Ollama",
        depends_on:       &[ids::HOMEBREW],
        fix_group:        Some("bundle"),
        post_fix_trigger: &[],
        gate_kind:        GateKind::Install,
        checker_fn: |_ctx| Box::new(BinaryChecker::new("ollama", "--version")),
        fixer_fn:   |_ctx| brew_bundle_fixer(),
    },
    // ── Sensei CLI ───────────────────────────────────────────────────────────
    ComponentSpec {
        id:               ids::SENSEI,
        label:            "Sensei CLI",
        depends_on:       &[ids::HOMEBREW],
        fix_group:        Some("bundle"),
        post_fix_trigger: &[],
        gate_kind:        GateKind::Install,
        checker_fn: |ctx| {
            let bin = ctx.config.sensei_binary();
            Box::new(VersionedBinaryChecker::new(bin, "--version", ctx.app_version))
        },
        fixer_fn: |ctx| sensei_binary_fixer(ids::SENSEI, ctx),
    },
    // ── Sensei Daemon (binary) ───────────────────────────────────────────────
    ComponentSpec {
        id:               ids::SENSEID,
        label:            "Sensei Daemon",
        depends_on:       &[ids::HOMEBREW],
        fix_group:        Some("bundle"),
        post_fix_trigger: &[ids::DATABASE],
        gate_kind:        GateKind::Install,
        checker_fn: |ctx| {
            let bin = ctx.config.senseid_binary();
            Box::new(VersionedBinaryChecker::new(bin, "--version", ctx.app_version))
        },
        fixer_fn: |ctx| sensei_binary_fixer(ids::SENSEID, ctx),
    },
    // ── Sensei MCP (binary) ──────────────────────────────────────────────────
    ComponentSpec {
        id:               ids::SENSEI_MCP,
        label:            "Sensei MCP",
        depends_on:       &[ids::HOMEBREW],
        fix_group:        Some("bundle"),
        post_fix_trigger: &[],
        gate_kind:        GateKind::Install,
        checker_fn: |ctx| {
            let bin = ctx.config.sensei_mcp_binary();
            Box::new(BinaryChecker::new(bin, "--version"))
        },
        fixer_fn: |ctx| sensei_binary_fixer(ids::SENSEI_MCP, ctx),
    },
    // ── PostgreSQL Service ───────────────────────────────────────────────────
    ComponentSpec {
        id:               ids::POSTGRESQL_SERVICE,
        label:            "PostgreSQL Service",
        depends_on:       &[ids::POSTGRESQL],
        fix_group:        None,
        post_fix_trigger: &[],
        gate_kind:        GateKind::Service,
        checker_fn: |_ctx| Box::new(PortChecker::new("postgresql", POSTGRES_PORT)),
        fixer_fn:   |ctx| Box::new(ServiceStartFixer::new(Arc::clone(ctx.platform), "postgresql", POSTGRES_PORT)),
    },
    // ── Ollama Service ───────────────────────────────────────────────────────
    ComponentSpec {
        id:               ids::OLLAMA_SERVICE,
        label:            "Ollama Service",
        depends_on:       &[ids::OLLAMA],
        fix_group:        None,
        post_fix_trigger: &[],
        gate_kind:        GateKind::Service,
        checker_fn: |_ctx| Box::new(PortChecker::new("ollama", OLLAMA_PORT)),
        fixer_fn:   |ctx| Box::new(ServiceStartFixer::new(Arc::clone(ctx.platform), "ollama", OLLAMA_PORT)),
    },
    // ── Sensei Database ──────────────────────────────────────────────────────
    ComponentSpec {
        id:               ids::DATABASE,
        label:            "Sensei Database",
        depends_on:       &[ids::POSTGRESQL_SERVICE, ids::SENSEID],
        fix_group:        None,
        post_fix_trigger: &[],
        gate_kind:        GateKind::Install,
        checker_fn: |_ctx| Box::new(DatabaseChecker),
        fixer_fn:   |ctx| Box::new(DatabaseSetupFixer::new(ctx.app_version)),
    },
    // ── Sensei Daemon Service ────────────────────────────────────────────────
    ComponentSpec {
        id:               ids::DAEMON,
        label:            "Sensei Daemon Service",
        depends_on:       &[ids::SENSEID, ids::DATABASE],
        fix_group:        None,
        post_fix_trigger: &[],
        gate_kind:        GateKind::Service,
        checker_fn: |ctx| Box::new(PortChecker::new("daemon", ctx.config.daemon_port)),
        fixer_fn:   |ctx| Box::new(DaemonFixer::new(
            Arc::clone(ctx.platform),
            ctx.config.daemon_port,
            ctx.config.is_dev(),
            ctx.config.senseid_binary(),
        )),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_ids_are_unique() {
        let mut seen: std::collections::HashSet<&str> = Default::default();
        for c in COMPONENTS {
            assert!(seen.insert(c.id), "duplicate component id: {}", c.id);
        }
    }

    #[test]
    fn depends_on_ids_are_valid() {
        let valid_ids: std::collections::HashSet<&str> = COMPONENTS.iter().map(|c| c.id).collect();
        for c in COMPONENTS {
            for dep in c.depends_on {
                assert!(
                    valid_ids.contains(dep),
                    "component '{}' depends_on '{}' which does not exist",
                    c.id, dep
                );
            }
        }
    }

    #[test]
    fn post_fix_trigger_ids_are_valid() {
        let valid_ids: std::collections::HashSet<&str> = COMPONENTS.iter().map(|c| c.id).collect();
        for c in COMPONENTS {
            for tid in c.post_fix_trigger {
                assert!(
                    valid_ids.contains(tid),
                    "component '{}' post_fix_trigger '{}' is not a valid id",
                    c.id, tid
                );
            }
        }
    }

    #[test]
    fn homebrew_has_no_dependencies() {
        let homebrew = COMPONENTS.iter().find(|c| c.id == ids::HOMEBREW).unwrap();
        assert!(homebrew.depends_on.is_empty());
        assert!(homebrew.fix_group.is_none());
    }

    #[test]
    fn senseid_triggers_database() {
        let senseid = COMPONENTS.iter().find(|c| c.id == ids::SENSEID).unwrap();
        assert!(
            senseid.post_fix_trigger.contains(&ids::DATABASE),
            "senseid must trigger database"
        );
    }

    #[test]
    fn build_context_constructs_checkers_and_fixers() {
        use crate::{platform, config::SenseiConfig};
        let platform = Arc::from(platform::detect());
        let config = SenseiConfig::from_env();
        let ctx = BuildContext {
            platform:    &platform,
            config:      &config,
            app_version: "0.1.0",
        };
        for spec in COMPONENTS {
            // Must not panic
            let _checker = (spec.checker_fn)(&ctx);
            let _fixer   = (spec.fixer_fn)(&ctx);
        }
    }
}
