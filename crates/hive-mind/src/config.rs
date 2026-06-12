//! Runtime config for sensei-hive (env-driven, with sane defaults).
use std::path::PathBuf;

pub struct HiveConfig { pub data_dir: PathBuf, pub database_dir: PathBuf, pub bind: String }

impl HiveConfig {
    pub fn from_env() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let data_dir = std::env::var("SENSEI_HIVE_DATA_DIR").map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join(".sensei-hive/pg"));
        let database_dir = std::env::var("SENSEI_HIVE_DDL_DIR").map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database"));
        let bind = std::env::var("SENSEI_HIVE_BIND").unwrap_or_else(|_| "127.0.0.1:7755".into());
        Self { data_dir, database_dir, bind }
    }
}
