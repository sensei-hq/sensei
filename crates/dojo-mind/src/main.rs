use std::sync::Arc;
use clap::{Parser, Subcommand};
use dojo_mind::api::{build_router_with_jwt, SharedState};
use dojo_mind::auth::JwtConfig;
use dojo_mind::config::DojoConfig;
use dojo_mind::db::DojoDb;
use dojo_mind::keygen::generate_key;
use dojo_mind::provision::provision;
use dojo_mind::store::DojoStore;

#[derive(Parser)]
#[command(name = "sensei-dojo", about = "sensei dojo-mind shared-brain service")]
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
    /// Provision a Dōjō tenant + membership + API key (bootstrap a tenant).
    Provision {
        /// Tenant origin: `github` (backed by a GitHub org) or `org` (custom).
        #[arg(long)] origin: String,
        /// The GitHub org id or custom org name.
        #[arg(long)] org: String,
        /// Optional sub-Dōjō path (e.g. a per-client engagement).
        #[arg(long)] dojo: Option<String>,
        /// The member's identity (email) — becomes the API-key member name.
        #[arg(long)] user: String,
        /// Dojo authority: contributor | maintainer | admin.
        #[arg(long)] role: String,
        /// Tenant visibility: private (default) | global.
        #[arg(long, default_value = "private")] scope: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();
    let cli = Cli::parse();
    let cfg = DojoConfig::from_env().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let db = DojoDb::bootstrap(cfg.supabase_db_url.clone(), cfg.data_dir.clone(), cfg.database_dir.clone()).await?;
    let store = DojoStore::new(db.pool().clone());
    match cli.cmd.unwrap_or(Cmd::Serve) {
        Cmd::Keygen { name, role, label } => {
            let key = generate_key(&store, &name, &role, label.as_deref()).await?;
            println!("API key for {name} ({role}) — store it now, shown once:\n{key}");
        }
        Cmd::Provision { origin, org, dojo, user, role, scope } => {
            let p = provision(&store, &origin, &org, dojo.as_deref(), &user, &role, &scope).await?;
            println!("provisioned dojo tenant — store the token now, shown once:");
            println!("  tenant_key    {}", p.tenant_key);
            println!("  tenant_id     {}", p.tenant_id);
            println!("  membership_id {}", p.membership_id);
            println!("  member_role     {}", p.member_role);
            println!("  token         {}", p.token);
        }
        Cmd::Serve => {
            let jwt = JwtConfig {
                secret: cfg.supabase_jwt_secret.clone(),
                audience: cfg.supabase_jwt_aud.clone(),
            };
            let app = build_router_with_jwt(Arc::new(SharedState { store }), jwt);
            let listener = tokio::net::TcpListener::bind(&cfg.bind).await?;
            tracing::info!(bind = %cfg.bind, "sensei-dojo listening");
            axum::serve(listener, app).with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; }).await?;
        }
    }
    Ok(())
}
