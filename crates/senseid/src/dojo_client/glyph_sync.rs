//! The glyph preference, synced between this machine and the dōjō (#133).
//!
//! The user's rule (2026-10-27): one kanji/icon choice across the app and the
//! dōjō, synced when it changes. Each side keeps the mode and WHEN it was
//! chosen; the later choice wins, by when it was made, not when it arrived — so
//! syncing either way, for any number of signed-in personas, converges and never
//! undoes a newer choice.

use serde::{Deserialize, Serialize};

/// The config key the app's Settings → General writes.
pub const GLYPHS_KEY: &str = "ui.glyphs";
/// When `ui.glyphs` was last chosen (RFC 3339), written beside it.
pub const GLYPHS_SET_AT_KEY: &str = "ui.glyphs.set_at";

/// A glyph choice and when it was made; `None`: never chosen, so any choice wins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlyphPref {
    pub glyphs: String,
    pub glyphs_set_at: Option<String>,
}

/// Which way one sync moves the choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sync {
    /// The dōjō's is later: take it here.
    Pull(GlyphPref),
    /// Ours is later: send it.
    Push(GlyphPref),
    Nothing,
}

fn when(p: &GlyphPref) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    p.glyphs_set_at.as_deref().and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
}

/// The later choice wins; one never made loses to any; equal times move nothing.
pub fn reconcile(local: &GlyphPref, remote: &GlyphPref) -> Sync {
    match (when(local), when(remote)) {
        (None, None) => Sync::Nothing,
        (None, Some(_)) => Sync::Pull(remote.clone()),
        (Some(_), None) => Sync::Push(local.clone()),
        (Some(l), Some(r)) if r > l => Sync::Pull(remote.clone()),
        (Some(l), Some(r)) if l > r => Sync::Push(local.clone()),
        _ => Sync::Nothing,
    }
}

/// This machine's choice, from its config. Kanji when nothing is set.
pub async fn local(pg: &crate::db::pg_store::PgStore) -> Result<GlyphPref, String> {
    let glyphs =
        pg.get_config(GLYPHS_KEY).await?.filter(|g| g == "icon").unwrap_or_else(|| "kanji".into());
    let glyphs_set_at = pg.get_config(GLYPHS_SET_AT_KEY).await?.filter(|t| !t.is_empty());
    Ok(GlyphPref { glyphs, glyphs_set_at })
}

/// Bring this machine and the dōjō, as `token`'s person, to the later choice.
/// Returns which way it moved.
pub async fn sync(
    pg: &crate::db::pg_store::PgStore,
    token: &str,
    plane: &dyn super::user_plane::UserPlane,
) -> Result<Sync, String> {
    let mine = local(pg).await?;
    let theirs = plane.preferences(token).await?;
    let action = reconcile(&mine, &theirs);
    match &action {
        Sync::Pull(p) => {
            pg.set_config(GLYPHS_KEY, &p.glyphs).await?;
            pg.set_config(GLYPHS_SET_AT_KEY, p.glyphs_set_at.as_deref().unwrap_or_default())
                .await?;
        }
        Sync::Push(p) => {
            plane.put_glyphs(token, p).await?;
        }
        Sync::Nothing => {}
    }
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(glyphs: &str, at: Option<&str>) -> GlyphPref {
        GlyphPref { glyphs: glyphs.into(), glyphs_set_at: at.map(str::to_string) }
    }

    #[test]
    fn the_later_choice_wins_either_way() {
        let early = p("kanji", Some("2026-10-27T09:00:00Z"));
        let late = p("icon", Some("2026-10-27T10:00:00+00:00"));
        assert_eq!(reconcile(&early, &late), Sync::Pull(late.clone()));
        assert_eq!(reconcile(&late, &early), Sync::Push(late.clone()));
        assert_eq!(reconcile(&p("kanji", None), &late), Sync::Pull(late.clone()));
        assert_eq!(reconcile(&late, &p("kanji", None)), Sync::Push(late.clone()));
        assert_eq!(reconcile(&p("kanji", None), &p("kanji", None)), Sync::Nothing);
        assert_eq!(reconcile(&late, &late.clone()), Sync::Nothing);
    }
}
