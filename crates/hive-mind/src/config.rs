//! Runtime config for sensei-hive (env-driven, with sane defaults).
use std::path::PathBuf;

pub struct HiveConfig { pub data_dir: PathBuf, pub database_dir: PathBuf, pub bind: String }

impl HiveConfig {
    /// Build config from env. Requires either `HOME` (for the default
    /// `~/.sensei-hive/pg` data dir) or an explicit `SENSEI_HIVE_DATA_DIR`.
    /// Refusing to fall back to `./.sensei-hive/pg` avoids silently placing
    /// an embedded Postgres cluster in whatever directory the service was
    /// launched from.
    pub fn from_env() -> Result<Self, String> {
        let data_dir = match std::env::var("SENSEI_HIVE_DATA_DIR") {
            Ok(v) => PathBuf::from(v),
            Err(_) => {
                let home = std::env::var("HOME").map_err(|_| {
                    "neither SENSEI_HIVE_DATA_DIR nor HOME is set; refusing to \
                     default the embedded Postgres data dir to the current \
                     directory. Set SENSEI_HIVE_DATA_DIR=/abs/path/to/hive/pg."
                        .to_string()
                })?;
                PathBuf::from(home).join(".sensei-hive/pg")
            }
        };
        let database_dir = std::env::var("SENSEI_HIVE_DDL_DIR").map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database"));
        let bind = std::env::var("SENSEI_HIVE_BIND").unwrap_or_else(|_| "127.0.0.1:7755".into());
        Ok(Self { data_dir, database_dir, bind })
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
        let saved_data = std::env::var("SENSEI_HIVE_DATA_DIR").ok();

        unsafe {
            // 1. HOME set, no override → data_dir under HOME.
            std::env::remove_var("SENSEI_HIVE_DATA_DIR");
            std::env::set_var("HOME", "/tmp/hive-home");
        }
        let cfg = HiveConfig::from_env().expect("HOME set → ok");
        assert_eq!(cfg.data_dir, PathBuf::from("/tmp/hive-home/.sensei-hive/pg"));

        // 2. Explicit override wins over HOME.
        unsafe { std::env::set_var("SENSEI_HIVE_DATA_DIR", "/var/lib/hive/pg"); }
        let cfg = HiveConfig::from_env().expect("override set → ok");
        assert_eq!(cfg.data_dir, PathBuf::from("/var/lib/hive/pg"));

        // 3. HOME unset AND no override → hard error, no ./ fallback.
        unsafe {
            std::env::remove_var("SENSEI_HIVE_DATA_DIR");
            std::env::remove_var("HOME");
        }
        let err = match HiveConfig::from_env() {
            Ok(_) => panic!("HOME unset + no override should have errored"),
            Err(e) => e,
        };
        assert!(err.contains("SENSEI_HIVE_DATA_DIR"), "err surfaces the fix: {err}");
        assert!(err.contains("HOME"), "err names the missing var: {err}");

        unsafe {
            match saved_home { Some(v) => std::env::set_var("HOME", v), None => std::env::remove_var("HOME") }
            match saved_data { Some(v) => std::env::set_var("SENSEI_HIVE_DATA_DIR", v), None => std::env::remove_var("SENSEI_HIVE_DATA_DIR") }
        }
    }
}
