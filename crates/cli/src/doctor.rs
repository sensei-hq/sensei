//! `sensei doctor` — bootstrap diagnostic with a clean, coloured trail.
//!
//! Two information streams collapse into one readable timeline:
//!
//!   * Typed `HealthEvent`s from the bootstrap pipeline → printed as
//!     coloured one-liners (green when something works, red when it
//!     doesn't, yellow when the orchestrator wants the human to do
//!     something).
//!
//!   * `tracing::info!`/`debug!` events from the library — off by default
//!     to keep the timeline scannable. Set `RUST_LOG=sensei_bootstrap=info`
//!     (or `=debug`/`=trace`) to surface the underlying probes, brew
//!     shell-outs, and per-stage cascade decisions.
//!
//! Exit code: 0 if the terminal payload is Ok, 1 otherwise.

use sensei_bootstrap::{self as bootstrap, HealthEvent, HealthPayload, HealthStatus, ComponentStatus};
use tracing_subscriber::EnvFilter;

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── ANSI helpers ────────────────────────────────────────────────────────────

const RESET:   &str = "\x1b[0m";
const BOLD:    &str = "\x1b[1m";
const DIM:     &str = "\x1b[2m";
const GREEN:   &str = "\x1b[32m";
const RED:     &str = "\x1b[31m";
const YELLOW:  &str = "\x1b[33m";
const BLUE:    &str = "\x1b[34m";
const CYAN:    &str = "\x1b[36m";

/// Per-component verb for the `Installing` status. The library uses a
/// single `ComponentStatus::Installing` value, but the resolver actions
/// behind it differ:
///   * service-style deps (postgres, ollama, daemon) — cascade tries
///     `brew services start` first, so "starting" is accurate in the
///     common case; only on stage-3 fallback is it actually installing.
///   * sensei — pure brew install (no service), "installing" fits.
///   * database — dbd-core schema setup, "setting up" is closer.
fn installing_verb(id: &str) -> &'static str {
    match id {
        "postgres" | "ollama" | "daemon" => "starting",
        "database"                       => "setting up",
        _                                => "installing",
    }
}

fn green(s: &str)  -> String { format!("{GREEN}{s}{RESET}") }
fn red(s: &str)    -> String { format!("{RED}{s}{RESET}") }
fn yellow(s: &str) -> String { format!("{YELLOW}{s}{RESET}") }
fn blue(s: &str)   -> String { format!("{BLUE}{s}{RESET}") }
fn cyan(s: &str)   -> String { format!("{CYAN}{s}{RESET}") }
fn dim(s: &str)    -> String { format!("{DIM}{s}{RESET}") }
fn bold(s: &str)   -> String { format!("{BOLD}{s}{RESET}") }

// ── Entry point ─────────────────────────────────────────────────────────────

pub fn run() -> i32 {
    install_tracing();

    println!("{}", bold(&format!("sensei doctor  {}", dim(&format!("v{CARGO_PKG_VERSION}")))));
    println!("{}", dim("Diagnoses your bootstrap dependencies. Set RUST_LOG=sensei_bootstrap=info"));
    println!("{}", dim("for verbose library tracing under each line."));
    println!();

    let terminal: HealthPayload = bootstrap::health::check_and_resolve(
        CARGO_PKG_VERSION,
        &print_event,
    );

    println!();
    print_terminal(&terminal);

    if terminal.status == HealthStatus::Ok { 0 } else { 1 }
}

// ── Subscriber: quiet by default ────────────────────────────────────────────

fn install_tracing() {
    // Default: warn-or-worse only, so the structured timeline isn't drowned
    // out. Users opt into deeper detail with RUST_LOG=sensei_bootstrap=info,
    // =debug, or =trace.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("sensei_bootstrap=warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .compact()
        .try_init();
}

// ── Event printer ───────────────────────────────────────────────────────────

fn print_event(ev: HealthEvent) {
    match ev {
        HealthEvent::Phase { phase } => {
            let label = match phase {
                HealthStatus::Checking   => cyan("● checking"),
                HealthStatus::Resolving  => yellow("● resolving"),
                HealthStatus::Ok         => green("● ok"),
                HealthStatus::NeedsAction => red("● needs action"),
            };
            println!("\n{label}");
        }
        HealthEvent::Component { id, patch } => {
            let status = patch.status;
            let detail = patch.detail.flatten();
            let version = patch.version.flatten();
            let icon_status = match status {
                Some(ComponentStatus::Ready)      => green("✓"),
                Some(ComponentStatus::Installing) => yellow("…"),
                Some(ComponentStatus::Failed)     => red("✗"),
                Some(ComponentStatus::Pending) | Some(ComponentStatus::Checking) | None => dim("·"),
            };
            let id_col = format!("{:<10}", id);
            let status_word = match status {
                Some(ComponentStatus::Ready)      => green("ready"),
                Some(ComponentStatus::Installing) => yellow(installing_verb(&id)),
                Some(ComponentStatus::Failed)     => red("failed"),
                Some(ComponentStatus::Pending)    => dim("pending"),
                Some(ComponentStatus::Checking)   => cyan("checking"),
                None                              => dim("—"),
            };
            let mut tail = String::new();
            if let Some(v) = version.filter(|v| !v.is_empty()) {
                tail.push_str(&dim(&format!(" {v}")));
            }
            if let Some(d) = detail.filter(|d| !d.is_empty()) {
                tail.push_str(&format!(" {}", dim(&d)));
            }
            println!("  {icon_status} {id_col} {status_word}{tail}");
        }
        HealthEvent::Remedy { remedy } => {
            // Per-component remedy emitted during resolve. The final
            // consolidated one is printed by print_terminal at the bottom,
            // so here we just hint that one was proposed.
            println!("  {} {}", yellow("⚠"), dim("remedy proposed — see summary below"));
            let _ = remedy;  // we don't print the script per-component
        }
        HealthEvent::Report { payload } => {
            // The first Report is the initial check's verdict — render
            // every component so the user sees green/red per-row right
            // away (the library's check() doesn't emit per-component
            // events; only resolve does).
            for c in &payload.components {
                let (icon, status_word) = match c.status {
                    ComponentStatus::Ready      => (green("✓"), green("ready")),
                    ComponentStatus::Installing => (yellow("…"), yellow(installing_verb(&c.id))),
                    ComponentStatus::Failed     => (red("✗"),    red("failed")),
                    ComponentStatus::Pending    => (dim("·"),    dim("pending")),
                    ComponentStatus::Checking   => (cyan("·"),   cyan("checking")),
                };
                let mut tail = String::new();
                if let Some(v) = c.version.as_deref().filter(|v| !v.is_empty()) {
                    tail.push_str(&dim(&format!("  {v}")));
                }
                if let Some(d) = c.detail.as_deref().filter(|d| !d.is_empty()) {
                    tail.push_str(&format!("  {}", dim(d)));
                }
                println!("  {icon} {:<10} {status_word}{tail}", c.id);
            }
            let label = match payload.status {
                HealthStatus::Ok          => green("ok"),
                HealthStatus::NeedsAction => red("needs action"),
                HealthStatus::Checking    => cyan("checking"),
                HealthStatus::Resolving   => yellow("resolving"),
            };
            println!("  {} {}", blue("⤳"), bold(&label));
        }
    }
}

// ── Terminal summary ────────────────────────────────────────────────────────

fn print_terminal(t: &HealthPayload) {
    let bar = "─".repeat(60);
    println!("{}", dim(&bar));
    let header = match t.status {
        HealthStatus::Ok          => green("✓ All five components healthy."),
        HealthStatus::NeedsAction => red("✗ One or more components need attention."),
        _                          => yellow(&format!("{:?}", t.status)),
    };
    println!("{}", bold(&header));
    println!();

    // Per-component table.
    for c in &t.components {
        let (icon, status_word) = match c.status {
            ComponentStatus::Ready      => (green("✓"), green("ready")),
            ComponentStatus::Installing => (yellow("…"), yellow(installing_verb(&c.id))),
            ComponentStatus::Failed     => (red("✗"),   red("failed")),
            ComponentStatus::Pending    => (dim("·"),   dim("pending")),
            ComponentStatus::Checking   => (cyan("·"),  cyan("checking")),
        };
        let mut tail = String::new();
        if let Some(v) = c.version.as_deref().filter(|v| !v.is_empty()) {
            tail.push_str(&dim(&format!("  {v}")));
        }
        if let Some(d) = c.detail.as_deref().filter(|d| !d.is_empty()) {
            tail.push_str(&format!("  {}", dim(d)));
        }
        println!("  {icon} {:<10} {status_word}{tail}", c.id);
    }

    // Consolidated remedy (if any).
    if let Some(r) = &t.remedy {
        println!();
        println!("{}", bold(&yellow("Remedy")));
        for line in r.message.lines() {
            println!("  {line}");
        }
        println!();
        println!("{}", dim("Run:"));
        for line in r.script.lines() {
            println!("    {}", bold(line));
        }
        println!();
        println!("{}", dim("After running the script above, re-run `sensei doctor` to verify."));
    } else if t.status == HealthStatus::Ok {
        println!();
        println!("{}", dim("Nothing else to do."));
    }
}
