//! Installer — marketplace fetch/cache, hook/skill/command installation.
//!
//! Both CLI and desktop delegate here via HTTP endpoints.
//! Binary copying is the only step that stays in the CLI (needs to happen
//! before the daemon is available).

mod catalog;
mod install;
mod marketplace;
mod removal;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Paths (delegated to crate::paths) ────────────────────────────────────────

pub(super) fn home() -> PathBuf { crate::paths::home() }
pub(super) fn sensei_dir() -> PathBuf { crate::paths::sensei_dir() }
pub(super) fn plugin_dir() -> PathBuf { crate::paths::plugin_dir() }
pub(super) fn cache_dir() -> PathBuf { crate::paths::cache_dir() }

// ── Constants ────────────────────────────────────────────────────────────────

pub(super) use crate::paths::MARKETPLACE_RAW_URL as MARKETPLACE_REPO;
pub(super) const MARKETPLACE_CATALOG: &str = "catalog.json";

// ── Catalog types ────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct Catalog {
    pub version: Option<String>,
    pub items: Vec<CatalogItem>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CatalogItem {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub recommended_for: Vec<String>,
    #[serde(default)]
    pub stage: Vec<String>,
}

// ── Install result ───────────────────────────────────────────────────────────

#[derive(Serialize, Default)]
pub struct InstallResult {
    pub hooks_installed: u32,
    pub skills_installed: u32,
    pub commands_installed: u32,
    pub stale_commands_removed: u32,
    pub stale_skills_removed: u32,
    pub acps_configured: Vec<String>,
    /// Hard failures — a step did not complete.
    pub errors: Vec<String>,
    /// Soft failures — the install completed, but a side-effect (e.g. dev hook
    /// entries) didn't. Surfaced separately so UI/CLI can decide whether to
    /// surface loudly or just log.
    #[serde(default)]
    pub warnings: Vec<String>,
    pub marketplace_version: String,
}

// ── Remove types ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct RemoveRequest {
    #[serde(default)]
    pub purge: bool,
}

#[derive(Serialize, Default)]
pub struct RemoveResult {
    pub acps_removed: Vec<String>,
    pub plugin_removed: bool,
    pub commands_removed: u32,
    pub skills_removed: u32,
    pub agents_removed: u32,
    pub hooks_removed: bool,
    pub cache_cleared: bool,
    pub projects_cleaned: Vec<String>,
    pub errors: Vec<String>,
}

// ── Installed item ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct InstalledItem {
    pub name: String,
    pub kind: String,
    pub path: String,
}

// ── Re-exports ───────────────────────────────────────────────────────────────

pub use catalog::fetch_catalog;
pub use install::{install, install_hooks_only, install_item, list_installed, remove_item};
pub use removal::remove;
