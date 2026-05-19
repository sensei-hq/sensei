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

use owo_colors::{OwoColorize, Stream};
use sensei_bootstrap::{self as bootstrap, Component, HealthEvent, HealthPayload, HealthStatus, ComponentStatus};

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Colour helpers ──────────────────────────────────────────────────────────
// Use owo-colors with `if_supports_color(Stream::Stdout, ...)` so the
// palette auto-disables on no-TTY (pipes, CI logs) and respects NO_COLOR.
// Each helper returns a fully-rendered String so callers can compose them
// without fighting the opaque colour wrappers in match arms.

fn green(s: &str)  -> String { format!("{}", s.if_supports_color(Stream::Stdout, |t| t.green())) }
fn red(s: &str)    -> String { format!("{}", s.if_supports_color(Stream::Stdout, |t| t.red())) }
fn yellow(s: &str) -> String { format!("{}", s.if_supports_color(Stream::Stdout, |t| t.yellow())) }
fn blue(s: &str)   -> String { format!("{}", s.if_supports_color(Stream::Stdout, |t| t.blue())) }
fn cyan(s: &str)   -> String { format!("{}", s.if_supports_color(Stream::Stdout, |t| t.cyan())) }
fn dim(s: &str)    -> String { format!("{}", s.if_supports_color(Stream::Stdout, |t| t.dimmed())) }
fn bold(s: &str)   -> String { format!("{}", s.if_supports_color(Stream::Stdout, |t| t.bold())) }

fn verb_for_component(c: &Component) -> &str { &c.installing_verb }

// ── Entry point ─────────────────────────────────────────────────────────────

pub fn run() -> i32 {
    bootstrap::tracing_init::install_console("sensei_bootstrap=warn");

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
                Some(ComponentStatus::Installing) => yellow(bootstrap::installing_verb_for(&id)),
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
                    ComponentStatus::Installing => (yellow("…"), yellow(verb_for_component(c))),
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
            ComponentStatus::Installing => (yellow("…"), yellow(verb_for_component(c))),
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
