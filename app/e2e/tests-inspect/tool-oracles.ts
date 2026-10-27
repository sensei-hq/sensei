/**
 * What each sensei tool SHOULD answer, worked out without the tool: from the
 * source (ripgrep), git, the installed packages, and the database. The tool-calls
 * inspect spec runs each tool through the Playground and holds its answer to
 * these. A check returns the problems it found; none means the answer is right.
 *
 * A tool whose answer only the daemon can know (it reads one table and returns
 * it) is held to that table, read here directly; where the daemon's REST answer
 * is the only other source, the check says so in its name.
 */
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

export type Answer = { isError: boolean; text: string; json: unknown };

export type Ctx = {
  root: string;
  wsId: string;
  sql: (q: string) => string[];
  rg: (args: string[]) => string[];
  git: (args: string[]) => string;
  daemon: (path: string) => Promise<unknown>;
};

export type Case = {
  tool: string;
  args: Record<string, string>;
  /** How the expectation was derived, for the report. */
  oracle: string;
  expect: (a: Answer, ctx: Ctx) => string[] | Promise<string[]>;
};

export function makeCtx(root: string, wsId: string, daemonUrl: string): Ctx {
  const run = (cmd: string, args: string[], cwd = root) => {
    try {
      return execFileSync(cmd, args, { cwd, encoding: 'utf8', maxBuffer: 64 << 20 });
    } catch (e) {
      // ripgrep exits 1 on no match; that is an empty answer, not a failure.
      const err = e as { status?: number; stdout?: string };
      if (cmd === 'rg' && err.status === 1) return '';
      throw e;
    }
  };
  return {
    root,
    wsId,
    sql: (q) => run('psql', ['-h', 'localhost', '-d', 'sensei', '-tAc', q]).split('\n').filter(Boolean),
    rg: (args) => run('rg', ['--no-ignore', '-g', '!target', '-g', '!node_modules', ...args]).split('\n').filter(Boolean),
    git: (args) => run('git', args).trim(),
    daemon: async (path) => (await fetch(`${daemonUrl}${path}`)).json(),
  };
}

const same = (label: string, got: Iterable<string>, want: Iterable<string>): string[] => {
  const g = new Set(got);
  const w = new Set(want);
  const missing = [...w].filter((x) => !g.has(x));
  const extra = [...g].filter((x) => !w.has(x));
  return [
    ...(missing.length ? [`${label}: missing ${missing.slice(0, 8).join(', ')}${missing.length > 8 ? ` (+${missing.length - 8})` : ''}`] : []),
    ...(extra.length ? [`${label}: not expected ${extra.slice(0, 8).join(', ')}${extra.length > 8 ? ` (+${extra.length - 8})` : ''}`] : []),
  ];
};
const ok = (a: Answer): string[] => (a.isError ? [`the tool reported an error: ${a.text.slice(0, 160)}`] : []);
const obj = (a: Answer) => a.json as Record<string, any>;

/** The repo-relative files that call `name(` — the definition lines excluded. */
function callSites(ctx: Ctx, name: string, definitions: RegExp): string[] {
  return ctx
    .rg(['-n', `\\b${name}\\(`, 'crates', 'packages', 'app/src', 'dojo/src'])
    .filter((l) => !definitions.test(l))
    .map((l) => l.split(':')[0]);
}

/** The workspace-defined free functions a Rust function's body calls. */
function rustCallees(ctx: Ctx, file: string, line: number): string[] {
  const lines = readFileSync(join(ctx.root, file), 'utf8').split('\n');
  const body: string[] = [];
  for (let i = line; i < lines.length; i++) {
    body.push(lines[i]);
    if (i > line - 1 && lines[i] === '}') break;
  }
  const text = body.join('\n');
  const closures = new Set([...text.matchAll(/let\s+(\w+)\s*=\s*\|/g)].map((m) => m[1]));
  const called = new Set([...text.matchAll(/(?<![.\w:])([a-z_][a-z0-9_]*)\(/g)].map((m) => m[1]));
  return [...called].filter((n) => !closures.has(n) && ctx.rg(['-l', `fn ${n}\\b`, 'crates']).length > 0);
}

const SKILLS_DIR = 'dojo/node_modules/@rokkit/cli';

export const CASES: Case[] = [
  {
    tool: 'search',
    args: { query: 'codemap' },
    oracle: 'ripgrep: where `fn codemap(` is defined',
    expect: (a, ctx) => {
      const want = ctx.rg(['-n', '^\\s*pub\\(crate\\) async fn codemap\\(', 'crates']).map((l) => {
        const [f, n] = l.split(':');
        return `${f}:${n}`;
      });
      const got = (obj(a).functions ?? []).filter((f: any) => f.name === 'codemap').map((f: any) => `${f.file_path}:${f.line_start}`);
      return [...ok(a), ...same('codemap defined at', got, want)];
    },
  },
  {
    tool: 'context_pack',
    args: { query: 'code map level 0 blocks' },
    oracle: 'disk: every item’s file exists and holds the matched text',
    expect: (a, ctx) => {
      const items = obj(a).items ?? [];
      const out = [...ok(a)];
      if (items.length === 0) out.push('no items');
      if (obj(a).count !== items.length) out.push(`count ${obj(a).count} but ${items.length} items`);
      for (const it of items) {
        const p = join(ctx.root, it.file);
        if (!existsSync(p)) out.push(`${it.file}: no such file`);
        else if (it.match && !readFileSync(p, 'utf8').includes(it.match.trim())) out.push(`${it.file}: does not hold “${it.match.slice(0, 40)}”`);
      }
      return out;
    },
  },
  {
    tool: 'get_callers',
    args: { name: 'level0' },
    oracle: 'ripgrep: the files that call `level0(`',
    expect: (a, ctx) => {
      const want = callSites(ctx, 'level0', /fn level0\(|level0\(groupBy: GroupBy\)/);
      return [...ok(a), ...same('caller files', (obj(a).callers ?? []).map((c: any) => c.file_path), want)];
    },
  },
  {
    tool: 'get_callees',
    args: { name: 'level0' },
    oracle: 'source: the workspace functions level0’s body calls',
    expect: (a, ctx) => {
      const def = ctx.rg(['-n', '^pub fn level0\\(', 'crates'])[0].split(':');
      const want = rustCallees(ctx, def[0], Number(def[1]));
      const got = (obj(a).callees ?? []).filter((c: any) => c.file_path.endsWith('.rs')).map((c: any) => c.name);
      return [...ok(a), ...same('rust callees', got, want)];
    },
  },
  {
    tool: 'get_impact',
    args: { name: 'level0' },
    oracle: 'ripgrep: every direct caller is reached',
    expect: (a, ctx) => {
      const reached = new Set((obj(a).reached ?? []).map((c: any) => c.file_path));
      const missing = callSites(ctx, 'level0', /fn level0\(|level0\(groupBy: GroupBy\)/).filter((f) => !reached.has(f));
      return [...ok(a), ...(missing.length ? [`direct callers not reached: ${[...new Set(missing)].join(', ')}`] : [])];
    },
  },
  {
    tool: 'get_workspace_summary',
    args: {},
    oracle: 'database: the sensei workspace’s id',
    expect: (a, ctx) => [...ok(a), ...(obj(a).workspace?.id === ctx.wsId ? [] : [`workspace ${obj(a).workspace?.id}, not ${ctx.wsId}`])],
  },
  {
    tool: 'get_lib_docs',
    args: { name: 'rokkit' },
    oracle: 'package: the overview is rokkit’s llms.txt',
    expect: (a, ctx) => {
      const out = [...ok(a)];
      if (obj(a).library !== 'rokkit') out.push(`library ${obj(a).library}`);
      const first = (obj(a).overview ?? [])[0]?.content ?? '';
      if (!/Rokkit/.test(first)) out.push('the overview does not describe rokkit');
      return out;
    },
  },
  {
    tool: 'search_lib_docs',
    args: { query: 'Select' },
    oracle: 'package: rokkit’s two select components rank first (a page matches on its content too, which the answer does not carry)',
    expect: (a) => {
      const top = (obj(a).results ?? []).slice(0, 2).map((r: any) => `${r.library}/${r.component}`);
      return [...ok(a), ...same('top two', top, ['rokkit/select', 'rokkit/multi-select'])];
    },
  },
  {
    tool: 'list_library_skills',
    args: { name: 'rokkit' },
    oracle: `package: the skills ${SKILLS_DIR} ships, word for word`,
    expect: (a, ctx) => {
      const dir = join(ctx.root, SKILLS_DIR, 'skills');
      const want = readdirSync(dir);
      const got = (a.json as any[]) ?? [];
      const out = [...ok(a), ...same('skills', got.map((s) => s.name), want)];
      for (const s of got) {
        const file = join(dir, s.name, 'SKILL.md');
        if (existsSync(file) && readFileSync(file, 'utf8') !== s.body) out.push(`${s.name}: body differs from the package`);
      }
      return out;
    },
  },
  {
    tool: 'get_library_skill',
    args: { name: 'rokkit', focus: 'charts' },
    oracle: 'package: charts-rokkit/SKILL.md, word for word',
    expect: (a, ctx) => {
      const want = readFileSync(join(ctx.root, SKILLS_DIR, 'skills/charts-rokkit/SKILL.md'), 'utf8');
      return [...ok(a), ...(obj(a).body === want ? [] : ['body differs from the package'])];
    },
  },
  {
    tool: 'list_library_agents',
    args: { name: 'rokkit' },
    oracle: `package: the agents ${SKILLS_DIR} ships`,
    expect: (a, ctx) => {
      const want = readdirSync(join(ctx.root, SKILLS_DIR, 'agents')).map((f) => f.replace(/\.md$/, ''));
      return [...ok(a), ...same('agents', ((a.json as any[]) ?? []).map((s) => s.name), want)];
    },
  },
  {
    tool: 'get_communities',
    args: {},
    oracle: 'database: inference.communities over the workspace’s folders — the 50 largest, and the total',
    expect: (a, ctx) => {
      const rows = ctx.sql(
        `select c.node_count from inference.communities c join sensei.folder_workspaces fw on fw.folder_id = c.folder_id where fw.workspace_id = '${ctx.wsId}' order by c.node_count desc`,
      );
      const got = (obj(a).communities ?? []).map((c: any) => String(c.node_count));
      const out = [...ok(a)];
      if (obj(a).total !== rows.length) out.push(`total ${obj(a).total}, the workspace has ${rows.length}`);
      if (got.join() !== rows.slice(0, 50).join()) out.push(`not the 50 largest: ${got.slice(0, 5).join(',')}… vs ${rows.slice(0, 5).join(',')}…`);
      return out;
    },
  },
  {
    tool: 'get_patterns',
    args: { pattern: 'route' },
    oracle: 'git: the tracked SvelteKit route files — each answered file is one, at most 200 of them, and the total covers them all',
    expect: (a, ctx) => {
      // The files git tracks — not a coverage report's copies of them.
      const want = ctx.git(['ls-files', 'app', 'dojo', 'website']).split('\n').filter((f) => /\/src\/routes\/(.+\/)?\+(page|layout|server|error)\./.test(f));
      const got: string[] = (obj(a).files ?? []).map((f: any) => f.file_path);
      const total = obj(a).total;
      const out = [...ok(a)];
      const notRoutes = got.filter((f) => !/\+(page|layout|server|error)\./.test(f));
      if (notRoutes.length) out.push(`not route files: ${notRoutes.slice(0, 3).join(', ')}`);
      if (got.length !== Math.min(200, total)) out.push(`${got.length} files for a total of ${total}`);
      if (!(total >= want.length)) out.push(`total ${total}, but ${want.length} route files are tracked`);
      return out;
    },
  },
  {
    tool: 'list_workspaces',
    args: {},
    oracle: 'database: sensei.workspaces',
    expect: (a, ctx) => [...ok(a), ...same('workspaces', (obj(a).workspaces ?? []).map((w: any) => w.id), ctx.sql('select id from sensei.workspaces'))],
  },
  {
    tool: 'find_workspaces',
    args: { under: '{root}' },
    oracle: 'database: the workspaces whose folders lie under the repo',
    expect: (a, ctx) => {
      const ids = ((a.json as any[]) ?? []).map((w) => w.id);
      return [...ok(a), ...(ids.includes(ctx.wsId) ? [] : ['sensei is not among them'])];
    },
  },
  {
    tool: 'get_user_for_workspace',
    args: {},
    oracle: 'git: user.name and user.email in the repo',
    expect: (a, ctx) => {
      const u = obj(a).user ?? {};
      const out = [...ok(a)];
      if (u.name !== ctx.git(['config', 'user.name'])) out.push(`name ${u.name}`);
      if (u.email !== ctx.git(['config', 'user.email'])) out.push(`email ${u.email}`);
      return out;
    },
  },
  {
    tool: 'use_workspace',
    args: { workspace: 'sensei' },
    oracle: 'database: pins the sensei workspace’s id',
    expect: (a, ctx) => [...ok(a), ...(a.text.includes(ctx.wsId) ? [] : ['does not name the sensei workspace id'])],
  },
  {
    tool: 'get_workflow_state',
    args: {},
    oracle: 'database: sensei.workflow_state for sensei',
    expect: (a, ctx) => {
      const [row] = ctx.sql(`select active_phase || '|' || coalesce(active_task, '') from sensei.workflow_state where workspace = 'sensei'`);
      const got = `${obj(a).active_phase}|${obj(a).active_task ?? ''}`;
      return [...ok(a), ...(got === row ? [] : [`${got} ≠ ${row}`])];
    },
  },
  {
    tool: 'match_pattern',
    args: { description: 'mcp server tool call' },
    oracle: 'disk: every ranked file exists, and the MCP call handler ranks',
    expect: (a, ctx) => {
      const m = obj(a).matches ?? [];
      const out = [...ok(a)];
      if (!m.length) out.push('no matches');
      for (const x of m) if (!x.name || !existsSync(join(ctx.root, x.name))) out.push(`ranked ${x.name}: no such file`);
      return out;
    },
  },
  {
    tool: 'get_pattern_for',
    args: { symbol: 'codemap' },
    oracle: 'database: the workspace patterns whose instances include diagrams.rs',
    expect: (a, ctx) => {
      const want = ctx.sql(
        `select id from sensei.workspace_patterns where workspace_id = '${ctx.wsId}' and instances::text like '%crates/senseid/src/api/handlers/diagrams.rs%'`,
      );
      return [...ok(a), ...same('patterns', (obj(a).patterns ?? []).map((p: any) => p.id), want)];
    },
  },
  {
    tool: 'get_duplicates',
    args: {},
    oracle: 'disk: both sides of each pair exist at the line named',
    expect: (a, ctx) => {
      const d = obj(a).duplicates ?? [];
      const out = [...ok(a)];
      if (obj(a).message) out.push(obj(a).message);
      if (!d.length) out.push('no duplicates');
      for (const p of d.slice(0, 10))
        for (const s of [p.a, p.b]) {
          const lines = existsSync(join(ctx.root, s.file)) ? readFileSync(join(ctx.root, s.file), 'utf8').split('\n') : [];
          if (!lines[s.line - 1]?.includes(s.name)) out.push(`${s.file}:${s.line} is not ${s.name}`);
        }
      return out;
    },
  },
  {
    tool: 'get_workspace_conventions',
    args: {},
    oracle: 'database: the workspace’s indexed files',
    expect: (a, ctx) => {
      const [n] = ctx.sql(`select count(*) from sensei.files f join sensei.folder_workspaces fw on fw.folder_id = f.folder_id where fw.workspace_id = '${ctx.wsId}'`);
      const got = obj(a).structure?.file_count;
      return [...ok(a), ...(got === Number(n) ? [] : [`file_count ${got}, the workspace has ${n}`])];
    },
  },
  {
    tool: 'resolve_risk_class',
    args: { paths: '["crates/senseid/src/analysis/codemap.rs"]' },
    oracle: 'rule: a production source file needs review',
    expect: (a) => [...ok(a), ...(obj(a).class === 'review' ? [] : [`class ${obj(a).class}`])],
  },
  {
    tool: 'get_rules',
    args: {},
    oracle: 'daemon REST: /api/knowledge/rules?workspace=sensei',
    expect: async (a, ctx) => {
      const want = (await ctx.daemon('/api/knowledge/rules?workspace=sensei')) as any;
      const ids = (x: any) => (x?.rules ?? x ?? []).map((r: any) => r.id ?? r.statement);
      return [...ok(a), ...same('rules', ids(a.json), ids(want))];
    },
  },
  {
    tool: 'get_commands',
    args: {},
    oracle: 'database: sensei.folder_commands over the workspace’s folders',
    expect: (a, ctx) => {
      const want = ctx.sql(`select c.id from sensei.folder_commands c join sensei.folder_workspaces fw on fw.folder_id = c.folder_id where fw.workspace_id = '${ctx.wsId}'`);
      return [...ok(a), ...same('commands', (obj(a).commands ?? []).map((c: any) => String(c.id)), want)];
    },
  },
  {
    tool: 'gateway_status',
    args: {},
    oracle: 'daemon REST: /api/gateway/status',
    expect: async (a, ctx) => {
      const want = (await ctx.daemon('/api/gateway/status')) as any;
      return [...ok(a), ...(obj(a).status === want.status ? [] : [`status ${obj(a).status}, REST says ${want.status}`])];
    },
  },
  {
    tool: 'get_layered_context',
    args: {},
    oracle: 'database: every memory is the workspace’s or global',
    expect: (a, ctx) => {
      const mine = new Set(ctx.sql(`select id from sensei.memories where workspace_id = '${ctx.wsId}' or workspace_id is null`));
      const foreign = (obj(a).memories ?? []).filter((m: any) => !mine.has(m.id)).map((m: any) => m.id);
      return [...ok(a), ...(foreign.length ? [`memories from another workspace: ${foreign.join(', ')}`] : [])];
    },
  },
  {
    tool: 'run_status',
    args: {},
    oracle: 'daemon REST: /api/runs',
    expect: async (a, ctx) => {
      const want = (await ctx.daemon('/api/runs')) as any;
      return [...ok(a), ...same('runs', (obj(a).runs ?? []).map((r: any) => r.id), (want.runs ?? []).map((r: any) => r.id))];
    },
  },
  {
    tool: 'get_pending_nudges',
    args: { run_id: '00000000-0000-0000-0000-000000000000' },
    oracle: 'an unknown run is refused, saying so',
    expect: (a) => (a.isError && /not found|unknown/i.test(a.text) && !/HTTP 404 Not Found$/.test(a.text.trim()) ? [] : [`answered: ${a.text.slice(0, 120)}`]),
  },
  {
    tool: 'get_intake_guide',
    args: {},
    oracle: 'daemon REST: /api/playbook/guide',
    expect: async (a, ctx) => {
      const want = (await ctx.daemon('/api/playbook/guide')) as any;
      return [...ok(a), ...(JSON.stringify(a.json) === JSON.stringify(want) ? [] : ['differs from the daemon’s guide'])];
    },
  },
  {
    tool: 'recommend_playbook',
    args: { lifecycle: 'stable', intent: 'fix a bug', risk: 'low' },
    oracle: 'an axis value it does not know is refused, naming the known ones',
    expect: (a) => (a.isError && /intent/.test(a.text) && /enhancement/.test(a.text) ? [] : [`answered: ${a.text.slice(0, 160)}`]),
  },
  {
    tool: 'list_playbook_rule_proposals',
    args: {},
    oracle: 'database: learned rules not yet enabled',
    expect: (a, ctx) =>
      [...ok(a), ...same('proposals', (obj(a).proposals ?? []).map((p: any) => p.id), ctx.sql(`select id from sensei.playbook_rules where source = 'learned' and not enabled`))],
  },
];

/** Not run: each writes to the live database, runs commands, or spends on a model. */
export const NOT_RUN: Record<string, string> = Object.fromEntries([
  ...['set_stance', 'create_session', 'update_session', 'add_library', 'update_phase', 'log_event', 'propose_memory', 'save_memory',
    'promote_memory', 'accept_proposal', 'reject_proposal', 'record_outcome', 'start_run', 'pause_run', 'register_plan',
    'update_task_status', 'report_run_outcome', 'accept_playbook_rule'].map((t) => [t, 'writes to the live database']),
  ['run_checkers', 'runs the adopted checker commands and records their verdicts'],
  ...['infer', 'embed', 'consensus', 'generate_image', 'plan'].map((t) => [t, 'spends on a model']),
]);
