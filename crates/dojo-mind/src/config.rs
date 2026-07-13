//! Runtime config for sensei-dojo (env-driven, with sane defaults).
use crate::auth::{DEFAULT_SUPABASE_JWT_AUD, DEFAULT_SUPABASE_JWT_SECRET};
use std::path::PathBuf;

pub struct DojoConfig {
    pub data_dir: PathBuf,
    pub database_dir: PathBuf,
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
    /// Build config from env. Requires either `HOME` (for the default
    /// `~/.sensei-dojo/pg` data dir) or an explicit `SENSEI_DOJO_DATA_DIR`.
    /// Refusing to fall back to `./.sensei-dojo/pg` avoids silently placing
    /// an embedded Postgres cluster in whatever directory the service was
    /// launched from.
    pub fn from_env() -> Result<Self, String> {
        let data_dir = match std::env::var("SENSEI_DOJO_DATA_DIR") {
            Ok(v) => PathBuf::from(v),
            Err(_) => {
                let home = std::env::var("HOME").map_err(|_| {
                    "neither SENSEI_DOJO_DATA_DIR nor HOME is set; refusing to \
                     default the embedded Postgres data dir to the current \
                     directory. Set SENSEI_DOJO_DATA_DIR=/abs/path/to/dojo/pg."
                        .to_string()
                })?;
                PathBuf::from(home).join(".sensei-dojo/pg")
            }
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

        unsafe {
            // 1. HOME set, no override → data_dir under HOME.
            std::env::remove_var("SENSEI_DOJO_DATA_DIR");
            std::env::set_var("HOME", "/tmp/dojo-home");
        }
        let cfg = DojoConfig::from_env().expect("HOME set → ok");
        assert_eq!(cfg.data_dir, PathBuf::from("/tmp/dojo-home/.sensei-dojo/pg"));

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

        unsafe {
            match saved_home { Some(v) => std::env::set_var("HOME", v), None => std::env::remove_var("HOME") }
            match saved_data { Some(v) => std::env::set_var("SENSEI_DOJO_DATA_DIR", v), None => std::env::remove_var("SENSEI_DOJO_DATA_DIR") }
        }
    }
}
