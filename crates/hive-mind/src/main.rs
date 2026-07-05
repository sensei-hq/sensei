use std::sync::Arc;
use clap::{Parser, Subcommand};
use hive_mind::api::{build_router, SharedState};
use hive_mind::config::HiveConfig;
use hive_mind::db::HiveDb;
use hive_mind::keygen::generate_key;
use hive_mind::store::HiveStore;

#[derive(Parser)]
#[command(name = "sensei-hive", about = "sensei hive-mind shared-brain service")]
struct Cli { #[command(subcommand)] cmd: Option<Cmd> }

#[derive(Subcommand)]
enum Cmd {
    /// Run the federation service (default).
    Serve,
    /// Mint a member + API key (bootstrap the first admin).
    Keygen {
        #[arg(long)] name: String,
        #[arg(long, default_value = "member")] role: String,
        #[arg(long)] label: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();
    let cli = Cli::parse();
    let cfg = HiveConfig::from_env().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let db = HiveDb::bootstrap(cfg.data_dir.clone(), cfg.database_dir.clone()).await?;
    let store = HiveStore::new(db.pool().clone());
    match cli.cmd.unwrap_or(Cmd::Serve) {
        Cmd::Keygen { name, role, label } => {
            let key = generate_key(&store, &name, &role, label.as_deref()).await?;
            println!("API key for {name} ({role}) — store it now, shown once:\n{key}");
        }
        Cmd::Serve => {
            let app = build_router(Arc::new(SharedState { store }));
            let listener = tokio::net::TcpListener::bind(&cfg.bind).await?;
            tracing::info!(bind = %cfg.bind, "sensei-hive listening");
            axum::serve(listener, app).with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; }).await?;
        }
    }
    Ok(())
}
