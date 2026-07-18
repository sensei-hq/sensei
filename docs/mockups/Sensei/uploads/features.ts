// ─── Platform features + roadmap — single source of truth ───
//
// Every feature/tool across the platform (sensei + dōjō + relay), with its phase
// and status. The site reads from here: the "what's shipped" copy, the labeled
// roadmap beat, the waitlist CTAs, and any standalone /roadmap page.
//
// KEEP THIS HONEST. Update `status` as work lands — the site follows. Nothing
// unbuilt should read as shipped. Ground truth for status:
//   docs/plan/README.md  (gap analysis + phases)
//   docs/backlog.md      (open GitHub issues)
//   VERSION              (current release; wired to the footer as __APP_VERSION__)

export type FeatureStatus =
  | 'shipped' // live in the product today
  | 'beta' // usable but incomplete — dogfooding
  | 'in-progress' // actively being built
  | 'planned' // committed, not started
  | 'exploring'; // researching / design-only

export type Surface =
  | 'app' // Observatory — the desktop app
  | 'core' // daemon · capture · CLI · MCP
  | 'dojo' // team consoles + federation
  | 'relay' // real-time pairing line
  | 'collective'; // global shared brain

export interface Phase {
  id: string;
  label: string; // section heading on the site
  blurb: string; // one-line framing under the heading
}

export interface Feature {
  id: string;
  surface: Surface;
  phaseId: Phase['id'];
  name: string;
  blurb: string; // one site-ready line, mentor voice — plain, no marketing
  status: FeatureStatus;
  since?: string; // version it shipped in, when known
  waitlist?: boolean; // show a "notify me / request early access" CTA
}

// Display metadata for status badges — keep labels/tone consistent across the site.
export const statusMeta: Record<FeatureStatus, { label: string; tone: 'live' | 'soon' | 'later' }> = {
  shipped: { label: 'Available', tone: 'live' },
  beta: { label: 'Beta', tone: 'soon' },
  'in-progress': { label: 'In progress', tone: 'soon' },
  planned: { label: 'Planned', tone: 'later' },
  exploring: { label: 'Exploring', tone: 'later' }
};

// Surfaces, in the order they should read on the site.
export const surfaceMeta: Record<Surface, { label: string }> = {
  app: { label: 'The app' },
  core: { label: 'The engine' },
  dojo: { label: 'Dōjō — for teams' },
  relay: { label: 'Relay' },
  collective: { label: 'The Collective' }
};

// Phases, in display order. `now` is the honest core; the rest is the labeled roadmap.
export const phases: Phase[] = [
  {
    id: 'now',
    label: 'Available now',
    blurb: 'The local loop — sensei watches, notices, and remembers, entirely on your machine.'
  },
  {
    id: 'phase-3',
    label: 'Next — deeper insight',
    blurb: 'More of what the loop already produces, surfaced where you can act on it.'
  },
  {
    id: 'phase-4',
    label: 'Dōjō — for teams',
    blurb: 'Extend the loop across a team: contribute, review, and distribute knowledge, anonymized.'
  },
  {
    id: 'phase-5',
    label: 'Relay — supervise from anywhere',
    blurb: 'Watch long, multi-agent runs from your phone. No code leaves your machine.'
  },
  {
    id: 'horizon',
    label: 'On the horizon',
    blurb: 'The community brain and everything an organization needs to run it.'
  }
];

export const features: Feature[] = [
  // ── Available now — the app ──────────────────────────────────────────────
  {
    id: 'today',
    surface: 'app',
    phaseId: 'now',
    name: 'Today',
    blurb: 'The morning briefing — one observation worth your attention, everything else out of sight.',
    status: 'shipped',
    since: 'v0.3.5'
  },
  {
    id: 'sessions',
    surface: 'app',
    phaseId: 'now',
    name: 'Sessions',
    blurb: 'The week in review — going well, not going well, and things noticed, in three lanes.',
    status: 'shipped',
    since: 'v0.3.5'
  },
  {
    id: 'insights',
    surface: 'app',
    phaseId: 'now',
    name: 'Insights',
    blurb: 'Patterns sensei noticed, each with confidence and provenance. You decide which become memories.',
    status: 'shipped',
    since: 'v0.3.5'
  },
  {
    id: 'memories',
    surface: 'app',
    phaseId: 'now',
    name: 'Memories',
    blurb: 'Adopted teachings — named, dated, and traceable. They earn trust as they prove out.',
    status: 'shipped',
    since: 'v0.3.5'
  },
  {
    id: 'instruments',
    surface: 'app',
    phaseId: 'now',
    name: 'Instruments',
    blurb: 'Try your tools in isolation and replay what the assistant actually did.',
    status: 'shipped',
    since: 'v0.3.5'
  },
  {
    id: 'projects',
    surface: 'app',
    phaseId: 'now',
    name: 'Projects',
    blurb: 'A per-project view of every codebase sensei watches.',
    status: 'shipped',
    since: 'v0.3.5'
  },

  // ── Available now — the engine ───────────────────────────────────────────
  {
    id: 'capture',
    surface: 'core',
    phaseId: 'now',
    name: 'Local capture',
    blurb: 'Observes your sessions with Claude Code and Zed — locally, on your machine.',
    status: 'shipped'
  },
  {
    id: 'code-graph',
    surface: 'core',
    phaseId: 'now',
    name: 'Live code graph',
    blurb: 'A semantic graph of your code — symbols, calls, and clusters — re-indexed as you edit.',
    status: 'shipped'
  },
  {
    id: 'ftr-loop',
    surface: 'core',
    phaseId: 'now',
    name: 'The learning loop',
    blurb: 'Capture → analyze → learn → measure first-turn-resolution, then reinforce what worked.',
    status: 'shipped'
  },
  {
    id: 'semantic-search',
    surface: 'core',
    phaseId: 'now',
    name: 'Semantic search + context-pack',
    blurb: 'Concept-level retrieval for your assistant — hybrid semantic search with a grep floor.',
    status: 'shipped'
  },
  {
    id: 'mcp-server',
    surface: 'core',
    phaseId: 'now',
    name: 'MCP server',
    blurb: "Exposes sensei's graph, rules, and patterns to your assistant over MCP.",
    status: 'shipped'
  },
  {
    id: 'model-routing',
    surface: 'core',
    phaseId: 'now',
    name: 'Model routing',
    blurb: 'Bring your own key (Claude, GPT) or run fully local on embedded Gemma.',
    status: 'shipped'
  },
  {
    id: 'governance',
    surface: 'core',
    phaseId: 'now',
    name: 'Rules & governance',
    blurb: 'Project rules with a hierarchy, applied during work and consolidated as they grow.',
    status: 'shipped'
  },

  // ── Next — deeper insight (phase 3) ──────────────────────────────────────
  {
    id: 'preferences',
    surface: 'app',
    phaseId: 'phase-3',
    name: 'Configure & preferences',
    blurb: 'A persistent settings surface — tune what sensei watches and how it behaves.',
    status: 'planned'
  },
  {
    id: 'extend',
    surface: 'app',
    phaseId: 'phase-3',
    name: 'Extend & customize',
    blurb: 'Author your own agents, personas, and skills, in-app.',
    status: 'planned'
  },
  {
    id: 'pattern-catalog',
    surface: 'app',
    phaseId: 'phase-3',
    name: 'Pattern catalog',
    blurb: 'Browse the full catalog of patterns sensei has learned.',
    status: 'planned'
  },
  {
    id: 'export-import',
    surface: 'core',
    phaseId: 'phase-3',
    name: 'Export & import',
    blurb: 'Export every memory, pattern, and guard to JSON — and import to restore.',
    status: 'planned'
  },
  {
    id: 'dora',
    surface: 'core',
    phaseId: 'phase-3',
    name: 'DORA delivery metrics',
    blurb: 'The DORA four keys alongside FTR — how AI pairing moves delivery outcomes.',
    status: 'planned'
  },
  {
    id: 'default-governance',
    surface: 'core',
    phaseId: 'phase-3',
    name: 'Starter governance bundle',
    blurb: 'A curated starter constitution so a fresh project inherits real rules, not an empty file.',
    status: 'in-progress'
  },

  // ── Dōjō — for teams (phase 4) ───────────────────────────────────────────
  {
    id: 'dojo-teams',
    surface: 'dojo',
    phaseId: 'phase-4',
    name: 'Dōjō for teams',
    blurb: 'Contribute → triage → approve → distribute team knowledge, with anonymization.',
    status: 'in-progress',
    waitlist: true
  },
  {
    id: 'dojo-consoles',
    surface: 'dojo',
    phaseId: 'phase-4',
    name: 'Team consoles',
    blurb: 'Developer, maintainer, admin, and lead consoles for governing shared knowledge.',
    status: 'in-progress',
    waitlist: true
  },
  {
    id: 'dojo-membership',
    surface: 'dojo',
    phaseId: 'phase-4',
    name: 'Sign-in & membership',
    blurb: 'Join an org with GitHub sign-in and reach your team’s Dōjō.',
    status: 'in-progress'
  },

  // ── Relay — supervise from anywhere (phase 5) ────────────────────────────
  {
    id: 'relay-line',
    surface: 'relay',
    phaseId: 'phase-5',
    name: 'Relay live line',
    blurb: 'A private line from the daemon to your phone — filtered status only, code stays home.',
    status: 'beta',
    waitlist: true
  },
  {
    id: 'relay-run-engine',
    surface: 'relay',
    phaseId: 'phase-5',
    name: 'Autonomous run engine',
    blurb: 'Long runs that pause under limits, recover from crashes, and make progress over asking.',
    status: 'in-progress',
    waitlist: true
  },
  {
    id: 'relay-mobile',
    surface: 'relay',
    phaseId: 'phase-5',
    name: 'Mobile companion',
    blurb: 'Watch progress, approve gates, and unblock runs from anywhere.',
    status: 'planned',
    waitlist: true
  },

  // ── On the horizon ───────────────────────────────────────────────────────
  {
    id: 'collective',
    surface: 'collective',
    phaseId: 'horizon',
    name: 'The Collective',
    blurb: 'An opt-in community brain — anonymized patterns, shared and pulled across the network.',
    status: 'planned',
    waitlist: true
  },
  {
    id: 'enterprise',
    surface: 'dojo',
    phaseId: 'horizon',
    name: 'Enterprise',
    blurb: 'SSO, SCIM, and team billing for organizations running a private Dōjō.',
    status: 'planned',
    waitlist: true
  }
];
