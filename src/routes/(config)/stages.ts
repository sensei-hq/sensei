/**
 * Setup wizard stage definitions — metadata for the rail and navigation.
 * Each stage maps to a route under (config)/setup/.
 *
 * Icons are *meaning* kanji, not counters — the glyph reinforces what the
 * step is about, and survives reordering without going stale. The numeric
 * "01 / 11" lives in the bottom bar for a sense of progression.
 */

export interface WizardStage {
  id: string;
  path: string;
  icon: string;
  title: string;
  sub: string;
  watermark: boolean;
}

export const STAGES: WizardStage[] = [
  {
    id: "welcome",
    path: "/setup/welcome",
    icon: "礼",
    title: "Welcome",
    sub: "先生 · a quiet observer of your work",
    watermark: true,
  },
  {
    id: "preferences",
    path: "/setup/preferences",
    icon: "名",
    title: "Preferences",
    sub: "A few small choices before you step in. Anything here can be changed later by re-opening this wizard.",
    watermark: true,
  },
  {
    id: "assistants",
    path: "/setup/assistants",
    icon: "連",
    title: "Assistants",
    sub: "Registers plugins, skills, commands, agents, logging and metrics.",
    watermark: true,
  },
  {
    id: "roots",
    path: "/setup/roots",
    icon: "庵",
    title: "Roots",
    sub: "Where your work lives. Sensei recurses and finds repos.",
    watermark: true,
  },
  {
    id: "scan",
    path: "/setup/scan",
    icon: "観",
    title: "Scan",
    sub: "Ready for scan",
    watermark: false,
  },
  {
    id: "projects",
    path: "/setup/projects",
    icon: "組",
    title: "Projects",
    sub: "A project has one or more repos. Edit, split, or confirm.",
    watermark: true,
  },
  {
    id: "libraries",
    path: "/setup/libraries",
    icon: "書",
    title: "Libraries",
    sub: "Libraries without their own MCP — sensei indexes docs & code and wraps them with its own tools.",
    watermark: true,
  },
  {
    id: "instruments",
    path: "/setup/instruments",
    icon: "器",
    title: "Instruments",
    sub: "Tools sensei can reach for — recommended based on what's in your stack. Each MCP brings its own capabilities.",
    watermark: true,
  },
  {
    id: "inference",
    path: "/setup/inference",
    icon: "想",
    title: "Inference",
    sub: "Providers give sensei models for reasoning — inferring insights, consolidating memory, and making recommendations. Add providers, select models",
    watermark: true,
  },
  {
    id: "assignments",
    path: "/setup/assignments",
    icon: "任",
    title: "Assignments",
    sub: "Decide which models handle each reasoning role. The first model is primary; the rest are fallbacks used if the primary is down or rate-limited.",
    watermark: true,
  },
  {
    id: "done",
    path: "/setup/done",
    icon: "入",
    title: "Enter",
    sub: "",
    watermark: false,
  },
];

/** Find the index of the current stage from a URL pathname. */
export function stageIndex(pathname: string): number {
  const segment = pathname.split("/").pop() ?? "";
  const idx = STAGES.findIndex((s) => s.id === segment);
  return idx >= 0 ? idx : 0;
}

/** Get next stage path, or null if at the end. */
export function nextStagePath(pathname: string): string | null {
  const idx = stageIndex(pathname);
  return idx < STAGES.length - 1 ? STAGES[idx + 1].path : null;
}

/** Get previous stage path, or null if at the start. */
export function prevStagePath(pathname: string): string | null {
  const idx = stageIndex(pathname);
  return idx > 0 ? STAGES[idx - 1].path : null;
}
