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
//! Exit code: 0 if the terminal payload is Ok, every assistant's capture is
//! healthy and every indexing engine is ready; 1 otherwise.

use owo_colors::{OwoColorize, Stream};
use sensei_bootstrap::{
    self as bootstrap, Component, ComponentStatus, HealthEvent, HealthPayload, HealthStatus,
    SenseiConfig,
    engines::{self, EngineStatus},
};

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Colour helpers ──────────────────────────────────────────────────────────
// Use owo-colors with `if_supports_color(Stream::Stdout, ...)` so the
// palette auto-disables on no-TTY (pipes, CI logs) and respects NO_COLOR.
// Each helper returns a fully-rendered String so callers can compose them
// without fighting the opaque colour wrappers in match arms.

fn green(s: &str) -> String {
    format!("{}", s.if_supports_color(Stream::Stdout, |t| t.green()))
}
fn red(s: &str) -> String {
    format!("{}", s.if_supports_color(Stream::Stdout, |t| t.red()))
}
fn yellow(s: &str) -> String {
    format!("{}", s.if_supports_color(Stream::Stdout, |t| t.yellow()))
}
fn blue(s: &str) -> String {
    format!("{}", s.if_supports_color(Stream::Stdout, |t| t.blue()))
}
fn cyan(s: &str) -> String {
    format!("{}", s.if_supports_color(Stream::Stdout, |t| t.cyan()))
}
fn dim(s: &str) -> String {
    format!("{}", s.if_supports_color(Stream::Stdout, |t| t.dimmed()))
}
fn bold(s: &str) -> String {
    format!("{}", s.if_supports_color(Stream::Stdout, |t| t.bold()))
}

fn verb_for_component(c: &Component) -> &str {
    &c.installing_verb
}

// ── Entry point ─────────────────────────────────────────────────────────────

pub fn run(fix: bool) -> i32 {
    bootstrap::tracing_init::install_console("sensei_bootstrap=warn");

    println!("{}", bold(&format!("sensei doctor  {}", dim(&format!("v{CARGO_PKG_VERSION}")))));
    println!(
        "{}",
        dim("Diagnoses your bootstrap dependencies. Set RUST_LOG=sensei_bootstrap=info")
    );
    println!("{}", dim("for verbose library tracing under each line."));
    println!();

    let terminal: HealthPayload =
        bootstrap::health::check_and_resolve(CARGO_PKG_VERSION, &print_event);

    println!();
    print_terminal(&terminal);

    let capture_pressure = print_capture_section(fix);
    let engines_pressure = print_engines_section(fix);
    let watchdog_pressure = print_watchdog_section();
    if terminal.status == HealthStatus::Ok
        && capture_pressure == 0
        && engines_pressure == 0
        && watchdog_pressure == 0
    {
        0
    } else {
        1
    }
}

// ── Event printer ───────────────────────────────────────────────────────────

fn print_event(ev: HealthEvent) {
    match ev {
        HealthEvent::Phase { phase } => {
            let label = match phase {
                HealthStatus::Checking => cyan("● checking"),
                HealthStatus::Resolving => yellow("● resolving"),
                HealthStatus::Ok => green("● ok"),
                HealthStatus::NeedsAction => red("● needs action"),
            };
            println!("\n{label}");
        }
        HealthEvent::Component { id, patch } => {
            let status = patch.status;
            let detail = patch.detail.flatten();
            let version = patch.version.flatten();
            let icon_status = match status {
                Some(ComponentStatus::Ready) => green("✓"),
                Some(ComponentStatus::Installing) => yellow("…"),
                Some(ComponentStatus::Failed) => red("✗"),
                Some(ComponentStatus::Pending) | Some(ComponentStatus::Checking) | None => dim("·"),
            };
            let id_col = format!("{:<10}", id);
            let status_word = match status {
                Some(ComponentStatus::Ready) => green("ready"),
                Some(ComponentStatus::Installing) => yellow(bootstrap::installing_verb_for(&id)),
                Some(ComponentStatus::Failed) => red("failed"),
                Some(ComponentStatus::Pending) => dim("pending"),
                Some(ComponentStatus::Checking) => cyan("checking"),
                None => dim("—"),
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
            let _ = remedy; // we don't print the script per-component
        }
        HealthEvent::Report { payload } => {
            // The first Report is the initial check's verdict — render
            // every component so the user sees green/red per-row right
            // away (the library's check() doesn't emit per-component
            // events; only resolve does).
            for c in &payload.components {
                let (icon, status_word) = match c.status {
                    ComponentStatus::Ready => (green("✓"), green("ready")),
                    ComponentStatus::Installing => (yellow("…"), yellow(verb_for_component(c))),
                    ComponentStatus::Failed => (red("✗"), red("failed")),
                    ComponentStatus::Pending => (dim("·"), dim("pending")),
                    ComponentStatus::Checking => (cyan("·"), cyan("checking")),
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
                HealthStatus::Ok => green("ok"),
                HealthStatus::NeedsAction => red("needs action"),
                HealthStatus::Checking => cyan("checking"),
                HealthStatus::Resolving => yellow("resolving"),
            };
            println!("  {} {}", blue("⤳"), bold(&label));
        }
    }
}

// ── Capture / Assistants section ────────────────────────────────────────────

/// Render the Capture / Assistants section by querying the daemon. Returns
/// extra exit-code pressure: 1 if any adapter is failing, else 0.
fn print_capture_section(fix: bool) -> i32 {
    let cfg = SenseiConfig::from_env();
    let base = format!("http://127.0.0.1:{}", cfg.daemon_port);
    println!();
    println!("{}", bold("Capture / Assistants"));

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("build http client");

    let health: serde_json::Value = match client.get(format!("{base}/api/assistants/health")).send()
    {
        Ok(r) => match r.json() {
            Ok(j) => j,
            Err(e) => {
                println!("  {} {}", red("✗"), dim(&format!("bad response: {e}")));
                return 1;
            }
        },
        Err(_) => {
            println!(
                "  {} {}",
                red("✗"),
                dim(
                    "daemon unreachable — cannot verify capture. Is senseid running? `sensei start`"
                )
            );
            return 1;
        }
    };

    let adapters = health["adapters"].as_array().cloned().unwrap_or_default();
    if adapters.is_empty() {
        println!("  {} {}", dim("·"), dim("no configured assistants detected"));
        return 0;
    }

    let mut worst_fail = 0;
    let mut failing_ids: Vec<String> = vec![];
    for a in &adapters {
        let id = a["adapter_id"].as_str().unwrap_or("?");
        let astatus = a["status"].as_str().unwrap_or("unknown");
        let (icon, _) = status_glyph(astatus);
        println!("  {} {}", icon, bold(id));
        for c in a["checks"].as_array().cloned().unwrap_or_default() {
            let cid = c["id"].as_str().unwrap_or("?");
            let cstatus = c["status"].as_str().unwrap_or("unknown");
            let detail = c["detail"].as_str().unwrap_or("");
            let (gi, _) = status_glyph(cstatus);
            println!("    {} {:<12} {}", gi, cid, dim(detail));
        }
        if astatus == "fail" {
            worst_fail = 1;
            failing_ids.push(id.to_string());
        }
    }

    if fix && !failing_ids.is_empty() {
        println!();
        for id in &failing_ids {
            println!("  {} resolving {}…", yellow("…"), id);
            let body = serde_json::json!({ "adapter_id": id });
            match client
                .post(format!("{base}/api/assistants/resolve"))
                .json(&body)
                .send()
                .and_then(|r| r.json::<serde_json::Value>())
            {
                Ok(rep) => {
                    let ok = rep["ok"].as_bool().unwrap_or(false);
                    if ok {
                        println!("    {} resolved", green("✓"));
                    } else {
                        println!("    {} failed: {}", red("✗"), dim(&rep["errors"].to_string()));
                    }
                }
                Err(e) => println!("    {} request failed: {}", red("✗"), dim(&e.to_string())),
            }
        }
        println!("{}", dim("Re-run `sensei doctor` to confirm."));
    } else if worst_fail == 1 {
        println!();
        println!(
            "{}",
            dim(
                "Run `sensei doctor --fix` to auto-resolve, or restart your Claude session if only `events` is stale."
            )
        );
    }

    worst_fail
}

// ── Watchdog section ────────────────────────────────────────────────────────

/// The window the doctor reads abandonments over.
const WATCHDOG_DAYS: i32 = 7;

/// `/api/tasks/abandoned` as the doctor reads it.
#[derive(serde::Deserialize)]
struct Abandonments {
    days: i32,
    by_day: Vec<AbandonedDay>,
    worst: Vec<AbandonedTask>,
}

#[derive(serde::Deserialize)]
struct AbandonedDay {
    day: String,
    task_kind: String,
    count: i64,
}

#[derive(serde::Deserialize)]
struct AbandonedTask {
    task_kind: String,
    path: String,
    count: i64,
    last_at: String,
}

/// Render the tasks the watchdog abandoned (#188). An abandonment is a warning,
/// not a failure: it makes "slow" visible before it is "missing". Returns exit
/// pressure 1 only when the daemon could not answer — an unread count is not a
/// clean one.
fn print_watchdog_section() -> i32 {
    let cfg = SenseiConfig::from_env();
    let url =
        format!("http://127.0.0.1:{}/api/tasks/abandoned?days={WATCHDOG_DAYS}", cfg.daemon_port);
    println!();
    println!("{}", bold("Watchdog"));
    let read = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .and_then(|c| c.get(&url).send())
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json::<Abandonments>());
    match read {
        Ok(a) => {
            let lines = abandonment_lines(&a);
            let glyph = if a.by_day.is_empty() { green("✓") } else { yellow("⚠") };
            println!("  {} {}", glyph, lines[0]);
            for line in &lines[1..] {
                println!("    {}", dim(line));
            }
            0
        }
        Err(e) => {
            println!("  {} {}", red("✗"), dim(&format!("cannot read abandonments: {e}")));
            1
        }
    }
}

/// The watchdog section's lines: a total, one line per day (its kinds in the
/// daemon's order, most first), then the worst offenders by kind and path.
fn abandonment_lines(a: &Abandonments) -> Vec<String> {
    let total: i64 = a.by_day.iter().map(|d| d.count).sum();
    if total == 0 {
        return vec![format!("no task abandoned in the last {} days", a.days)];
    }
    let mut lines = vec![format!("{total} tasks abandoned in the last {} days", a.days)];
    for day in a.by_day.chunk_by(|x, y| x.day == y.day) {
        let count: i64 = day.iter().map(|d| d.count).sum();
        let kinds: Vec<String> =
            day.iter().map(|d| format!("{} {}", d.task_kind, d.count)).collect();
        lines.push(format!("{}  {count}  {}", day[0].day, kinds.join(" · ")));
    }
    lines.push("worst:".to_string());
    for w in &a.worst {
        let path = if w.path.is_empty() { "—" } else { &w.path };
        let day = w.last_at.get(..10).unwrap_or(&w.last_at);
        lines.push(format!("{}× {}  {path}  (last {day})", w.count, w.task_kind));
    }
    lines
}

// ── Indexing engines section ────────────────────────────────────────────────

/// Render the Indexing engines section — the compilers the indexer places call
/// edges with. With `fix`, the engines that are not ready are installed first.
/// Returns extra exit-code pressure: 1 if any engine is still not ready.
fn print_engines_section(fix: bool) -> i32 {
    println!();
    println!("{}", bold("Indexing engines"));

    let statuses = if fix {
        let ensured = engines::ensure();
        for engine in &ensured.installed {
            println!("  {} installing {}…", yellow("…"), engine.label());
        }
        for failure in &ensured.failures {
            println!("    {} {}", red("✗"), dim(&failure.to_string()));
        }
        ensured.after
    } else {
        engines::check()
    };

    for status in &statuses {
        if status.is_ready() {
            println!("  {} {:<10} {}", green("✓"), status.engine.label(), green("ready"));
            continue;
        }
        println!("  {} {:<10} {}", red("✗"), status.engine.label(), red("missing"));
        for cause in &status.missing {
            println!("      {}", dim(&cause.to_string()));
        }
    }

    let pressure = engines_pressure(&statuses);
    if pressure != 0 && !fix {
        println!();
        println!("{}", dim("Run `sensei doctor --fix` to install them."));
    }
    pressure
}

/// Exit-code pressure from the engines: 1 if any is not ready, else 0.
fn engines_pressure(statuses: &[EngineStatus]) -> i32 {
    i32::from(statuses.iter().any(|s| !s.is_ready()))
}

/// Map a status string to a coloured glyph.
fn status_glyph(s: &str) -> (String, &'static str) {
    match s {
        "ok" => (green("✓"), "ok"),
        "warn" => (yellow("⚠"), "warn"),
        "fail" => (red("✗"), "fail"),
        _ => (dim("·"), "unknown"),
    }
}

// ── Terminal summary ────────────────────────────────────────────────────────

fn print_terminal(t: &HealthPayload) {
    let bar = "─".repeat(60);
    println!("{}", dim(&bar));
    let header = match t.status {
        HealthStatus::Ok => green("✓ All five components healthy."),
        HealthStatus::NeedsAction => red("✗ One or more components need attention."),
        _ => yellow(&format!("{:?}", t.status)),
    };
    println!("{}", bold(&header));
    println!();

    // Per-component table.
    for c in &t.components {
        let (icon, status_word) = match c.status {
            ComponentStatus::Ready => (green("✓"), green("ready")),
            ComponentStatus::Installing => (yellow("…"), yellow(verb_for_component(c))),
            ComponentStatus::Failed => (red("✗"), red("failed")),
            ComponentStatus::Pending => (dim("·"), dim("pending")),
            ComponentStatus::Checking => (cyan("·"), cyan("checking")),
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

#[cfg(test)]
mod tests {
    use super::*;
    use sensei_bootstrap::engines::{Cause, Engine, EngineStatus};

    fn status(engine: Engine, missing: Vec<Cause>) -> EngineStatus {
        EngineStatus { engine, missing }
    }

    /// #188: the doctor shows what the watchdog abandoned — a total, the
    /// count per day split by kind, and the worst offenders by kind and path.
    #[test]
    fn abandonments_render_per_day_and_name_the_worst() {
        let body = serde_json::json!({
            "days": 7,
            "by_day": [
                { "day": "2026-10-25", "task_kind": "ingest_capture", "count": 3 },
                { "day": "2026-10-25", "task_kind": "compile_projects", "count": 1 },
                { "day": "2026-10-23", "task_kind": "compute_group_metrics", "count": 2 }
            ],
            "worst": [
                { "task_kind": "ingest_capture", "path": "/t/a.jsonl", "count": 3,
                  "last_at": "2026-10-25T14:02:00Z" },
                { "task_kind": "compute_group_metrics", "path": "", "count": 2,
                  "last_at": "2026-10-23T09:00:00Z" }
            ]
        });
        let lines = abandonment_lines(&serde_json::from_value(body).unwrap());
        assert_eq!(lines[0], "6 tasks abandoned in the last 7 days");
        assert_eq!(lines[1], "2026-10-25  4  ingest_capture 3 · compile_projects 1");
        assert_eq!(lines[2], "2026-10-23  2  compute_group_metrics 2");
        assert_eq!(lines[3], "worst:");
        assert_eq!(lines[4], "3× ingest_capture  /t/a.jsonl  (last 2026-10-25)");
        assert_eq!(lines[5], "2× compute_group_metrics  —  (last 2026-10-23)");
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn no_abandonment_says_so() {
        let body = serde_json::json!({ "days": 7, "by_day": [], "worst": [] });
        assert_eq!(
            abandonment_lines(&serde_json::from_value(body).unwrap()),
            vec!["no task abandoned in the last 7 days"]
        );
    }

    #[test]
    fn every_engine_ready_adds_no_exit_pressure() {
        let ready = [status(Engine::Rust, vec![]), status(Engine::TypeScript, vec![])];
        assert_eq!(engines_pressure(&ready), 0);
    }

    #[test]
    fn one_engine_missing_fails_the_doctor() {
        let one_missing =
            [status(Engine::Rust, vec![]), status(Engine::TypeScript, vec![Cause::NodeMissing])];
        assert_eq!(engines_pressure(&one_missing), 1);
        let all_missing = [
            status(Engine::Rust, vec![Cause::RustupMissing]),
            status(Engine::TypeScript, vec![Cause::NodeMissing]),
        ];
        assert_eq!(engines_pressure(&all_missing), 1);
    }
}
