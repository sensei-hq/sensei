//! The seeded catalogue of supported tools and this machine's installs (#455).
use super::{AssistantInstallRow, CatalogueWordRow, PgStore};

impl PgStore {
    /// Every spelling of every supported tool (`sensei.assistant_catalogue_words`),
    /// one query. A failed read is an error, never an empty catalogue.
    pub async fn catalogue_words(&self) -> Result<Vec<CatalogueWordRow>, String> {
        todo!("#455 red")
    }

    /// The catalogue's words as [`crate::assistants::CatalogueNames`].
    pub async fn catalogue_names(&self) -> Result<crate::assistants::CatalogueNames, String> {
        todo!("#455 red")
    }

    /// Upsert one install row per probed tool. Returns the registry ids the
    /// catalogue does not know (not stored).
    pub async fn upsert_assistant_installs(
        &self,
        rows: &[AssistantInstallRow<'_>],
        sensei_version: &str,
    ) -> Result<Vec<String>, String> {
        let _ = (rows, sensei_version);
        todo!("#455 red")
    }

    /// This machine's install rows joined to the catalogue, in catalogue order.
    pub async fn list_assistant_installs(&self) -> Result<Vec<serde_json::Value>, String> {
        todo!("#455 red")
    }

    /// Whether no install row exists yet (first run).
    pub async fn assistant_installs_empty(&self) -> Result<bool, String> {
        todo!("#455 red")
    }
}
