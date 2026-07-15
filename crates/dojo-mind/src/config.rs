//! Runtime config for sensei-dojo (env-driven, with sane defaults).
use crate::auth::{DEFAULT_SUPABASE_JWT_AUD, DEFAULT_SUPABASE_JWT_SECRET};
use std::path::PathBuf;

pub struct DojoConfig {
    /// Embedded-Postgres data dir — the zero-config local fallback, used only
    /// when `supabase_db_url` is `None`.
    pub data_dir: PathBuf,
    pub database_dir: PathBuf,
    /// Postgres URL for the dojo data layer (`SUPABASE_DB_URL`). Point it at a
    /// local Supabase (self-hosted dojo) or a hosted Supabase (SaaS). When unset,
    /// dojo falls back to an embedded Postgres at `data_dir` for local dev.
    pub supabase_db_url: Option<String>,
    pub bind: String,
    /// Secret used to verify Supabase JWTs on the dojo routes. Defaults to the
    /// Supabase local-dev secret so tests (and a bare local run) work without a
    /// running Supabase; production sets `SUPABASE_JWT_SECRET`.
    pub supabase_jwt_secret: String,
    /// Expected audience on Supabase JWTs (`SUPABASE_JWT_AUD`, default
    /// `authenticated`).
    pub supabase_jwt_aud: String,
}

impl DojoConfig {
    /// Build config from env. `SUPABASE_DB_URL` selects the Supabase data layer;
    /// when it's unset dojo falls back to an embedded Postgres, which needs a data
    /// dir — either `SENSEI_DOJO_DATA_DIR` or `HOME` (for `~/.sensei-dojo/pg`).
    /// Refusing to default that dir to the launch directory avoids silently
    /// placing a cluster there. With `SUPABASE_DB_URL` set, the data dir is unused
    /// so a missing `HOME` is not an error.
    pub fn from_env() -> Result<Self, String> {
        let supabase_db_url = std::env::var("SUPABASE_DB_URL").ok().filter(|s| !s.is_empty());
        let data_dir = match std::env::var("SENSEI_DOJO_DATA_DIR") {
            Ok(v) => PathBuf::from(v),
            Err(_) => match std::env::var("HOME") {
                Ok(home) => PathBuf::from(home).join(".sensei-dojo/pg"),
                // Only the embedded fallback needs a data dir; with Supabase it's unused.
                Err(_) if supabase_db_url.is_some() => PathBuf::new(),
                Err(_) => return Err(
                    "neither SENSEI_DOJO_DATA_DIR nor HOME is set; refusing to \
                     default the embedded Postgres data dir to the current \
                     directory. Set SENSEI_DOJO_DATA_DIR=/abs/path/to/dojo/pg, or \
                     set SUPABASE_DB_URL to use Supabase instead.".to_string()),
            },
        };
        let database_dir = std::env::var("SENSEI_DOJO_DDL_DIR").map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database"));
        let bind = std::env::var("SENSEI_DOJO_BIND").unwrap_or_else(|_| "127.0.0.1:7755".into());
        let supabase_jwt_secret = std::env::var("SUPABASE_JWT_SECRET")
            .unwrap_or_else(|_| DEFAULT_SUPABASE_JWT_SECRET.into());
        let supabase_jwt_aud = std::env::var("SUPABASE_JWT_AUD")
            .unwrap_or_else(|_| DEFAULT_SUPABASE_JWT_AUD.into());
        Ok(Self {
            data_dir,
            database_dir,
            supabase_db_url,
            bind,
            supabase_jwt_secret,
            supabase_jwt_aud,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The env-var swaps below are process-global. Run serially inside one
    /// test so parallel test threads can't observe half-applied state. The
    /// `unsafe` blocks are required by Rust 2024: env mutation is unsound
    /// under multithreaded access (other threads may race a getenv). Only
    /// this test touches these vars, and cargo runs each `#[test]` on its
    /// own; the other tests in this crate live in `tests/` and hit a real DB.
    #[test]
    fn from_env_paths() {
        let saved_home = std::env::var("HOME").ok();
        let saved_data = std::env::var("SENSEI_DOJO_DATA_DIR").ok();
        let saved_url = std::env::var("SUPABASE_DB_URL").ok();
        unsafe { std::env::remove_var("SUPABASE_DB_URL"); }

        unsafe {
            // 1. HOME set, no override → data_dir under HOME.
            std::env::remove_var("SENSEI_DOJO_DATA_DIR");
            std::env::set_var("HOME", "/tmp/dojo-home");
        }
        let cfg = DojoConfig::from_env().expect("HOME set → ok");
        assert_eq!(cfg.data_dir, PathBuf::from("/tmp/dojo-home/.sensei-dojo/pg"));
        assert_eq!(cfg.supabase_db_url, None, "no SUPABASE_DB_URL → embedded fallback");

        // 2. Explicit override wins over HOME.
        unsafe { std::env::set_var("SENSEI_DOJO_DATA_DIR", "/var/lib/dojo/pg"); }
        let cfg = DojoConfig::from_env().expect("override set → ok");
        assert_eq!(cfg.data_dir, PathBuf::from("/var/lib/dojo/pg"));

        // 3. HOME unset AND no override → hard error, no ./ fallback.
        unsafe {
            std::env::remove_var("SENSEI_DOJO_DATA_DIR");
            std::env::remove_var("HOME");
        }
        let err = match DojoConfig::from_env() {
            Ok(_) => panic!("HOME unset + no override should have errored"),
            Err(e) => e,
        };
        assert!(err.contains("SENSEI_DOJO_DATA_DIR"), "err surfaces the fix: {err}");
        assert!(err.contains("HOME"), "err names the missing var: {err}");

        // 4. SUPABASE_DB_URL set → picked up, and no HOME/data-dir is required.
        unsafe { std::env::set_var("SUPABASE_DB_URL", "postgresql://postgres:postgres@127.0.0.1:54322/postgres"); }
        let cfg = DojoConfig::from_env().expect("SUPABASE_DB_URL set → ok even without HOME");
        assert_eq!(cfg.supabase_db_url.as_deref(), Some("postgresql://postgres:postgres@127.0.0.1:54322/postgres"));

        unsafe {
            match saved_home { Some(v) => std::env::set_var("HOME", v), None => std::env::remove_var("HOME") }
            match saved_data { Some(v) => std::env::set_var("SENSEI_DOJO_DATA_DIR", v), None => std::env::remove_var("SENSEI_DOJO_DATA_DIR") }
            match saved_url { Some(v) => std::env::set_var("SUPABASE_DB_URL", v), None => std::env::remove_var("SUPABASE_DB_URL") }
        }
    }
}
