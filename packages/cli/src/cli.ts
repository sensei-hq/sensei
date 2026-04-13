#!/usr/bin/env node
import { parseArgs } from "node:util";

const { positionals, values } = parseArgs({
  args: process.argv.slice(2),
  allowPositionals: true,
  options: {
    help: { type: "boolean", short: "h", default: false },
    force: { type: "boolean", default: false },
    "fail-on-drift": { type: "boolean", default: false },
    "dry-run": { type: "boolean", default: false },
    mcp: { type: "boolean", default: false },
    template: { type: "string" },
    examples: { type: "string" },
    sample: { type: "string" },   // number parsed manually
    port: { type: "string" },
    db: { type: "string" },
    // benchmark named params
    source: { type: "string" },
    dest: { type: "string" },
    model: { type: "string" },
    output: { type: "string" },
    acp: { type: "string" },
    tasks: { type: "string" },
    skills: { type: "string" },
    verbose: { type: "boolean", default: false },
    resume: { type: "string" },
    workdir: { type: "string" },
    corpus: { type: "string" },
    repo: { type: "string" },
    // stats command
    tool: { type: "string" },
    session: { type: "string" },
    since: { type: "string" },
    all: { type: "boolean", default: false },
    local: { type: "boolean", default: false },
    json: { type: "boolean", default: false },
    gaps: { type: "boolean", default: false },
    hooks: { type: "boolean", default: false },
    drift: { type: "boolean", default: false },
    agent: { type: "string" },
    lib: { type: "string" },
    global: { type: "boolean", default: false },
    // init options
    "use-recommended": { type: "boolean", default: false },
    // login options
    "platform-url": { type: "string" },
  },
});

const [cmd, ...rest] = positionals;

// Always operate from repo root — never scatter .index folders across the repo
import { findRepoRoot } from "./git.js";
import { existsSync } from "node:fs";
import { join as pathJoin } from "node:path";
const _cwd = process.cwd();
const repoRoot = findRepoRoot(_cwd);

// Global commands don't require a repo root — skip the guard for them.
const GLOBAL_CMDS = new Set(["serve", "server", "stats", "mcp", "benchmark", "remove", "clean", "register", "plugin"]);
if (!GLOBAL_CMDS.has(cmd) && repoRoot === _cwd && !existsSync(pathJoin(_cwd, ".git")) && !existsSync(pathJoin(_cwd, "package.json"))) {
  console.error("sensei: could not detect repo root. Run sensei from inside a git repo or a directory with package.json.");
  process.exit(1);
}

const HELP = `sensei — AI skills toolchain

Usage:
  sensei <command> [options]

Commands:
  init                     Set up a new repo (index + CLAUDE.md + hooks + skills)
  add                      Add sensei to an existing repo (non-destructive)
  setup --mcp              Register sensei MCP server in ~/.claude/mcp.json
  setup --hooks            Install Claude hook scripts and register daemon autostart
  status                   Show index age, drift status, active profiles
  index                    Re-index the current repo
  drift                    Check for doc drift
  doctor <path>            Reformat docs to match canonical templates
  benchmark doctor         Run 3-strategy doc conversion benchmark
  benchmark coverage       Score llmspec.yaml docs[].covers[] with Ollama
  benchmark populate       A/B benchmark: Claude without vs with populate-llmspec skill
  benchmark inspect        Switch to a benchmark branch for inspection
  benchmark promote        Merge chosen strategy branch, submit telemetry
  serve                    Start local telemetry report receiver
  server status            Check if server is running and show model setup status
  stats                    Show tool usage analytics (last 7 days)
  update-registry          Index custom_libs from .sensei/config.yaml locally
  update-registry --lib <name>   Re-index a single named library
  register [acp]           Register sensei MCP server, hooks, and settings in detected ACPs
  install-skills           Install bundled sensei skills for Claude Code (prompts for repo/global)
  install-skills --global  Install all skills globally (~/.claude/skills/)
  plugin install           Install and register the sensei plugin in Claude Code (~/.claude/plugins/)

Options:
  -h, --help               Show this help message

init:
  --global                 Install skills and hooks globally (~/.claude/) instead of repo-local
  --use-recommended        Install recommended skills without prompting

setup:
  --mcp                    Register MCP server (writes ~/.claude/mcp.json)
  --agent <name>           Generate and install project-specific skills (supported: claude)
  --hooks                  Install Claude hook scripts and register daemon autostart

index:
  --force                  Force full re-scan (ignore cached fingerprints)

drift:
  --fail-on-drift          Exit 1 if any drift is detected (for CI)

doctor:
  --dry-run                Show what would change without writing files
  --template <path>        Path to template file (default: docs/templates/feature.md)

benchmark doctor:
  --source <dir>           Input directory (e.g. docs/requirements)
  --dest <dir>             Output directory (e.g. docs/features)
  --template <path>        Template file (default: docs/templates/feature.md)
  --examples <dir>         Examples directory — can be absolute path (e.g. /path/to/other-repo/docs/features)
  --sample <n>             Process only the first N input files
  --verbose                Show git steps, prompt sizes, and Claude API call markers

benchmark coverage:
  Uses local Ollama model to populate llmspec.yaml docs[].covers[]
  and score against .sensei/llmspec-expected.yaml gold standard

benchmark indexer:
  Compare cocoindex-code vs sensei's symbol indexer.
  Measures file coverage, query relevance, and prints a spot-check for manual review.
  Requires: cocoindex-code installed (pipx install cocoindex-code) and indexed.

benchmark populate:
  Compares Claude-without-skill vs Claude-with-populate-llmspec-skill.
  Scores each strategy with score-coverage.ts and reports tokens, time, score.
  Requires: .sensei/llmspec.yaml and .sensei/llmspec-expected.yaml

benchmark inspect:
  sensei benchmark inspect <run>-<a|b|c>

benchmark promote:
  sensei benchmark promote <run-name>

serve:
  --port <n>               Port to listen on (default: 7744)

watch:
  --repo <path>            Repo to watch (default: auto-detected repo root)

stats:
  --all                    Show all-time data instead of last 7 days
  --tool <name>            Show stats for a specific tool
  --session <id>           Show all events for a session
  --since <YYYY-MM-DD>     Show events on or after this date
  --json                   Output as JSON
  --gaps                   Show missed-opportunity report (bash vs sensei tools)
`;

async function main() {
  if (values.help && !cmd) {
    console.log(HELP);
    return;
  }

  switch (cmd) {
    case "init": {
      const { init } = await import("./commands/init.js");
      await init(repoRoot, {
        global: values.global,
        useRecommended: values["use-recommended"],
      });
      break;
    }
    case "add": {
      const { add } = await import("./commands/add.js");
      await add(repoRoot);
      break;
    }
    case "setup": {
      if (values.agent) {
        const { setupAgent } = await import("./commands/setup.js");
        await setupAgent(repoRoot, values.agent);
        break;
      }
      if (values.hooks) {
        const { setupHooks } = await import("./commands/setup.js");
        await setupHooks();
        break;
      }
      if (!values.mcp) {
        console.error("Usage: sensei setup --mcp");
        process.exit(1);
      }
      const { setupMcp } = await import("./commands/setup.js");
      await setupMcp(repoRoot);
      break;
    }
    case "status": {
      const { status } = await import("./commands/status.js");
      await status(repoRoot);
      break;
    }
    case "index": {
      const { indexRepo } = await import("@sensei/graph-indexer");
      const { loadSenseiConfig } = await import("@sensei/shared");
      const { spinner } = await import("@clack/prompts");
      const config = await loadSenseiConfig(repoRoot);
      if (!config?.repo_id) { console.error("sensei: not initialised. Run sensei init first."); process.exit(1); }
      const s = spinner();
      s.start("Indexing repo...");
      const result = await indexRepo({ repoId: config.repo_id, repoPath: repoRoot, project: config.repo_id });
      s.stop(`Indexed: ${result.filesIndexed} files, ${result.functionsIndexed} functions, ${result.edgesCreated} edges (${result.durationMs}ms)`);
      break;
    }
    case "drift": {
      const { checkDrift } = await import("@sensei/tools");
      const failOnDrift = values["fail-on-drift"];
      const result = await checkDrift(repoRoot);
      console.log(result.summary);
      if (failOnDrift && result.drifted.length > 0) process.exit(1);
      break;
    }
    case "doctor": {
      const { doctor } = await import("./commands/doctor.js");
      const target = rest[0] ?? ".";
      await doctor(target, repoRoot, {
        dryRun: values["dry-run"],
        template: values.template,
      });
      break;
    }
    case "migrate": {
      const { migrate } = await import("./commands/migrate.js");
      await migrate(repoRoot);
      break;
    }
    case "mcp": {
      // Start the MCP stdio server — used by Claude Code, Cursor, etc.
      // Falls back to process.cwd() if SENSEI_REPO_PATH is not set.
      await import("@sensei/server/mcp-entry");
      break;
    }
    case "serve": {
      const subCmd = rest[0];
      if (subCmd === "stop" || subCmd === "restart") {
        const port = values.port ? parseInt(values.port, 10) : 7744;
        try {
          const res = await fetch(`http://127.0.0.1:${port}/stop`, { method: "POST" });
          if (res.ok) {
            console.log("sensei serve: stopped.");
          } else {
            console.error(`sensei serve: server returned ${res.status}`);
            process.exit(1);
          }
        } catch {
          console.error("sensei serve: server is not running.");
          process.exit(1);
        }
        if (subCmd === "restart") {
          // Give the old process a moment to exit, then start fresh.
          await new Promise(r => setTimeout(r, 500));
          const { serve } = await import("./commands/serve.js");
          await serve(repoRoot, { port: values.port ? parseInt(values.port, 10) : undefined });
        }
      } else {
        const { serve } = await import("./commands/serve.js");
        await serve(repoRoot, {
          port: values.port ? parseInt(values.port, 10) : undefined,
        });
      }
      break;
    }
    case "benchmark": {
      const subCmd = rest[0];
      if (!subCmd || values.help) {
        console.log(HELP);
        break;
      }
      if (subCmd === "doctor") {
        const { benchmarkDoctor } = await import("./commands/benchmark-doctor.js");
        // Support named params (--source / --dest) or legacy positional args
        const source = values.source ?? rest[1];
        const dest = values.dest ?? rest[2];
        if (!source || !dest) {
          console.error("Usage: sensei benchmark doctor --source <input-dir> --dest <output-dir> [--template <path>] [--examples <dir>] [--sample N] [--verbose]");
          process.exit(1);
        }
        await benchmarkDoctor(source, dest, repoRoot, {
          template: values.template,
          examples: values.examples,
          sample: values.sample ? parseInt(values.sample, 10) : undefined,
          verbose: values.verbose,
        });
      } else if (subCmd === "inspect") {
        const { benchmarkInspect } = await import("./commands/benchmark-inspect.js");
        const runBranch = rest[1];
        if (!runBranch) {
          console.error("Usage: sensei benchmark inspect <run>-<letter>");
          process.exit(1);
        }
        await benchmarkInspect(runBranch, repoRoot);
      } else if (subCmd === "promote") {
        const { benchmarkPromote } = await import("./commands/benchmark-promote.js");
        const runName = rest[1];
        if (!runName) {
          console.error("Usage: sensei benchmark promote <run-name>");
          process.exit(1);
        }
        await benchmarkPromote(runName, repoRoot);
      } else if (subCmd === "coverage") {
        const { benchmarkCoverage } = await import("./commands/benchmark-coverage.js");
        await benchmarkCoverage(repoRoot);
      } else if (subCmd === "populate") {
        const { benchmarkPopulate } = await import("./commands/benchmark-populate.js");
        await benchmarkPopulate(repoRoot);
      } else if (subCmd === "indexer") {
        const { benchmarkIndexer } = await import("./commands/benchmark-indexer.js");
        await benchmarkIndexer(repoRoot, { corpus: values.corpus, all: values.all });
      } else if (subCmd === "run") {
        const { benchmarkRun } = await import("./commands/benchmark-run.js");
        await benchmarkRun(repoRoot, {
          acp: values.acp,
          output: values.output,
          repo: values.repo,
          tasks: values.tasks,
          skills: values.skills,
          verbose: values.verbose,
          resume: values.resume,
          workdir: values.workdir,
        });
      } else {
        console.error(`Unknown benchmark subcommand: ${subCmd}\n`);
        console.log(HELP);
        process.exit(1);
      }
      break;
    }
    case "server": {
      const subCmd = rest[0];
      if (subCmd === "status" || !subCmd) {
        interface HealthResponse {
          backend?: string;
          ollamaRunning?: boolean;
        }
        interface SetupResponse {
          diskFreeGB?: number;
          ramAvailableGB?: number;
          ollamaModel?: boolean;
          ollamaModelName?: string;
        }
        const url = process.env.SENSEI_SERVER_URL ?? "http://localhost:7744";
        try {
          const res = await fetch(`${url}/health`, { signal: AbortSignal.timeout(2000) });
          if (!res.ok) throw new Error(`HTTP ${res.status}`);
          const data = await res.json() as HealthResponse;
          console.log(`sensei server: running at ${url}`);
          console.log(`  backend: ${data.backend ?? "none"}`);
          console.log(`  ollama:  ${data.ollamaRunning ? "running" : "not running"}`);
          const setupRes = await fetch(`${url}/setup/status`, { signal: AbortSignal.timeout(2000) });
          if (!setupRes.ok) throw new Error(`HTTP ${setupRes.status}`);
          const setup = await setupRes.json() as SetupResponse;
          console.log(`  disk:    ${setup.diskFreeGB} GB free`);
          console.log(`  ram:     ${setup.ramAvailableGB} GB available`);
          console.log(`  model:   ${setup.ollamaModel ? `✓ ${setup.ollamaModelName}` : `✗ ${setup.ollamaModelName} not pulled`}`);
        } catch {
          console.log(`sensei server: not running at ${url}`);
          console.log(`  Start with: sensei serve`);
        }
        break;
      }
      console.error(`Unknown server subcommand: ${subCmd}`);
      process.exit(1);
    }
    case "remove":
    case "clean": {
      const { remove } = await import("./commands/remove.js");
      await remove(repoRoot, {
        local: values.local,
        all: values.all,
        dryRun: values["dry-run"],
      });
      break;
    }
    case "watch": {
      const { watch } = await import("./commands/watch.js");
      const repo = values.repo ?? repoRoot;
      await watch(repo, { drift: values.drift });
      break;
    }
    case "stats": {
      const { stats } = await import("./commands/stats.js");
      await stats({
        all: values.all,
        tool: values.tool,
        session: values.session,
        since: values.since,
        json: values.json,
        gaps: values.gaps,
      });
      break;
    }
    case "update-registry": {
      const { updateRegistry } = await import("./commands/update-registry.js");
      await updateRegistry(repoRoot, values.lib);
      break;
    }
    case "install-skills": {
      const { installSkills, promptAndInstallSkills } = await import("./commands/install-skills.js");
      if (values.global) {
        await installSkills(repoRoot, "global");
      } else if (positionals.length > 1) {
        // sensei install-skills <repo-path>
        await installSkills(positionals[1], "repo");
      } else {
        await promptAndInstallSkills(repoRoot);
      }
      break;
    }
    case "register": {
      const { register } = await import("./commands/register.js");
      await register(rest[0]);
      break;
    }
    case "plugin": {
      const subCmd = rest[0];
      if (subCmd === "install" || !subCmd) {
        const { pluginInstall } = await import("./commands/plugin.js");
        await pluginInstall();
      } else {
        console.error(`Unknown plugin subcommand: ${subCmd}\nUsage: sensei plugin install`);
        process.exit(1);
      }
      break;
    }
    default:
      if (cmd) console.error(`Unknown command: ${cmd}\n`);
      console.log(HELP);
  }
}

main().catch(err => {
  console.error(err.message);
  process.exit(1);
});
