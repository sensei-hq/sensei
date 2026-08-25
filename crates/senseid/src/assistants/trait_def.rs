use super::helpers::check_mcp_in_config;
use super::{AssistantPart, AssistantStatus};
use crate::assistants::health::{AdapterCheck, AdapterResolveReport, CheckStatus};
use std::path::PathBuf;

/// Result of configuring an Assistant. `plugin` is true when `claude plugin install` succeeded.
pub(crate) struct AssistantConfigureOk {
    pub plugin: bool,
    pub warnings: Vec<String>,
}

/// Each Assistant implements detect, configure, and remove.
pub(crate) trait Assistant {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn mcp_key(&self) -> &str;
    fn config_path(&self) -> PathBuf;
    fn detect(&self) -> bool;
    fn configure(&self, mcp_cmd: &str) -> Result<AssistantConfigureOk, String>;
    fn remove(&self) -> bool;

    /// Refresh this assistant's sensei integration after a sensei binary
    /// upgrade. Returns the same result shape as `configure()`.
    ///
    /// Default = a no-op that reports success with an explanatory note. The
    /// file-based MCP assistants (Cursor, Zed, VS Code, …) wire sensei through
    /// their own config file and re-read it whenever they restart, so a fresh
    /// sensei binary needs no per-assistant action here. Only
    /// [`ClaudeCodeAssistant`](super::claude_code) overrides this — its plugin
    /// is a copied-out install that must be advanced with
    /// `claude plugin update sensei`, otherwise a new sensei binary leaves the
    /// old plugin/MCP behind.
    fn upgrade(&self) -> Result<AssistantConfigureOk, String> {
        Ok(AssistantConfigureOk {
            plugin: false,
            warnings: vec![format!(
                "no upgrade action for {} — its sensei MCP config is re-read when the assistant restarts",
                self.id()
            )],
        })
    }

    /// Pure, filesystem-only health probes for this adapter. Default = one
    /// check derived from `is_configured()`. Override for richer adapters.
    fn config_health(&self) -> Vec<AdapterCheck> {
        let status = if self.is_configured() { CheckStatus::Ok } else { CheckStatus::Fail };
        let detail = (status == CheckStatus::Fail)
            .then(|| "sensei not configured in this assistant".to_string());
        vec![AdapterCheck::new("configured", "configured", status, detail)]
    }

    /// Auto-resolve: default = re-run `configure()` (the install/reinstall flow)
    /// and map its result. `mcp_cmd` is resolved by the caller.
    fn resolve(&self, mcp_cmd: &str) -> AdapterResolveReport {
        match self.configure(mcp_cmd) {
            Ok(ok) => AdapterResolveReport {
                adapter_id: self.id().to_string(),
                ok: true,
                actions: {
                    let mut a = vec![format!("configured {}", self.id())];
                    a.extend(ok.warnings);
                    a
                },
                errors: vec![],
            },
            Err(e) => AdapterResolveReport {
                adapter_id: self.id().to_string(),
                ok: false,
                actions: vec![],
                errors: vec![e],
            },
        }
    }

    /// Keep this assistant's integration alive against external janitorial
    /// processes. Default = no-op. Claude Code overrides it to refresh its
    /// plugin in-use marker so the editor's periodic sweep doesn't prune a
    /// plugin the daemon installed out-of-band.
    fn keep_alive(&self) {}

    /// Family ID for UI grouping. Assistants in the same family show as one card.
    /// Default: same as id (each Assistant is its own family).
    fn family(&self) -> &str {
        self.id()
    }

    /// Display name for the family (used when grouped).
    fn family_name(&self) -> &str {
        self.name()
    }

    /// Capability parts this Assistant configures. Drives the per-part chips
    /// shown on each AssistantCard in the setup wizard. Default = one `mcp`
    /// part, which matches every MCP-file integration. Claude Code overrides
    /// to expose the four sub-capabilities its plugin install lands.
    fn parts(&self) -> Vec<AssistantPart> {
        vec![AssistantPart { id: "mcp".into(), label: "mcp server".into() }]
    }

    /// Default integration check — looks for the sensei MCP entry in the
    /// assistant's config file. Override for assistants that integrate via
    /// a different mechanism (Claude Code uses `claude plugin install` and
    /// records membership in `~/.claude/plugins/installed_plugins.json`).
    fn is_configured(&self) -> bool {
        check_mcp_in_config(&self.config_path(), self.mcp_key())
    }

    fn status(&self) -> AssistantStatus {
        AssistantStatus {
            id: self.id().to_string(),
            name: self.name().to_string(),
            family: self.family().to_string(),
            installed: self.detect(),
            configured: self.is_configured(),
            config_path: self.config_path().to_string_lossy().into_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct StubAssistant {
        configured: bool,
        configure_fails: bool,
    }
    impl Assistant for StubAssistant {
        fn id(&self) -> &str {
            "stub"
        }
        fn name(&self) -> &str {
            "Stub"
        }
        fn mcp_key(&self) -> &str {
            "mcpServers"
        }
        fn config_path(&self) -> PathBuf {
            PathBuf::from("/dev/null")
        }
        fn detect(&self) -> bool {
            true
        }
        fn configure(&self, _mcp_cmd: &str) -> Result<AssistantConfigureOk, String> {
            if self.configure_fails {
                Err("configure boom".to_string())
            } else {
                Ok(AssistantConfigureOk { plugin: false, warnings: vec![] })
            }
        }
        fn remove(&self) -> bool {
            true
        }
        fn is_configured(&self) -> bool {
            self.configured
        }
    }

    #[test]
    fn default_config_health_reflects_is_configured() {
        assert_eq!(
            StubAssistant { configured: true, ..Default::default() }.config_health()[0].status,
            CheckStatus::Ok
        );
        assert_eq!(
            StubAssistant { configured: false, ..Default::default() }.config_health()[0].status,
            CheckStatus::Fail
        );
    }

    #[test]
    fn default_upgrade_is_noop_ok_with_note() {
        // A non-Claude assistant (StubAssistant doesn't override upgrade) gets
        // the trait default: Ok, no plugin action, and a note explaining why.
        // This is the shape the fan-out surfaces for every file-based MCP
        // assistant after a sensei upgrade.
        let ok = StubAssistant::default().upgrade().expect("default upgrade is Ok");
        assert!(!ok.plugin, "default upgrade must not claim a plugin action");
        assert!(
            ok.warnings.iter().any(|w| w.contains("no upgrade action") && w.contains("stub")),
            "default upgrade note should name the assistant; got {:?}",
            ok.warnings
        );
    }

    #[test]
    fn default_resolve_maps_configure_success() {
        let r = StubAssistant { configured: false, ..Default::default() }.resolve("sensei-mcp");
        assert!(r.ok);
        assert_eq!(r.adapter_id, "stub");
        assert!(r.errors.is_empty());
    }

    #[test]
    fn default_resolve_maps_configure_error() {
        // The ok=false / errors mapping is load-bearing: Task 6's watchdog reads
        // `ok` to decide whether auto-repair succeeded.
        let r = StubAssistant { configured: false, configure_fails: true }.resolve("sensei-mcp");
        assert!(!r.ok);
        assert_eq!(r.adapter_id, "stub");
        assert_eq!(r.errors, vec!["configure boom".to_string()]);
        assert!(r.actions.is_empty());
    }
}
