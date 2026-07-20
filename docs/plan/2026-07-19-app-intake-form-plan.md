# Sensei-app Intake Form Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A per-user intake screen in the desktop app — describe a work chunk in freeform, the daemon classifies it + recommends a playbook, and on confirm the run is recorded (`playbook_run`).

**Architecture:** Reuses the shipped front-door backend (`/api/playbook/guide`, `/api/playbook/recommend`, `classify_chunk`, `is_trusted`). One small backward-compatible daemon change adds a `preview` flag (recommend without persisting) and returns the classified axes. A new SvelteKit route in the observatory group renders the flow via a runes state module; a nav entry links to it.

**Tech Stack:** Rust (`crates/senseid`, axum handler + pure helpers); SvelteKit 5 (runes), UnoCSS + `@rokkit/unocss` (canonical tokens `docs/architecture/frontend-svelte-guidelines.md`); Vitest; the Svelte MCP autofixer (mandatory for `.svelte`).

**Design:** `docs/plan/2026-07-19-app-intake-form-design.md`.

**Conventions:**
- **GIT HYGIENE:** the pre-commit hook stages broadly — always `git status` then explicit `git add <paths>`; leave a clean tree; per-task commit to `develop` (approach A).
- **Package manager:** `app/` is bun. Run `cd app && bun run check` (svelte-check) and `bun run test` (vitest) — read `app/package.json` for exact script names before running.
- **Svelte:** every `.svelte` / `.svelte.ts` edit MUST go through the Svelte MCP autofixer (`mcp__plugin_svelte_svelte__svelte-autofixer`) until clean, then re-run to confirm.
- **Visual verification is deferred to Jerry** (Tauri app — the UI does not render in bare Vite; `svelte-check` + unit tests gate it structurally).

---

### Task 1: Backend — `preview` flag (skip persist) + classified axes in response

**Files:**
- Modify: `crates/senseid/src/api/handlers/playbook.rs` (persist block ~L72-93; response `json!` ~L110-119; helper `parse_confirm` L245-249; test mod `classify_tests` L283-end)

The recommend leg of the app form must classify + recommend **without** writing a row; the confirm leg writes exactly one. The response must also carry the classified axes so the form can display them and drive the confirm call. `parse_confirm` is a generic truthy-flag parser (bool or `"true"` string) — rename it `parse_bool_flag` and reuse it for the `preview` flag (DRY; no second parser).

- [ ] **Step 1: Write the failing test.** In `crates/senseid/src/api/handlers/playbook.rs`, in `mod classify_tests`, rename the existing `confirm_accepts_bool_and_string` test body to call `parse_bool_flag`, and add a new `should_persist` test:

```rust
    #[test]
    fn bool_flag_accepts_bool_and_string() {
        use serde_json::json;
        // MCP tool sends the string form; direct callers may send a real bool.
        assert!(parse_bool_flag(&json!(true)));
        assert!(parse_bool_flag(&json!("true")));
        assert!(parse_bool_flag(&json!("TRUE")));
        assert!(!parse_bool_flag(&json!("false")));
        assert!(!parse_bool_flag(&json!(false)));
        assert!(!parse_bool_flag(&serde_json::Value::Null));
    }

    #[test]
    fn preview_flag_skips_persist() {
        use serde_json::json;
        assert!(should_persist(&json!({})));                   // no preview → persist
        assert!(should_persist(&json!({ "preview": false }))); // explicit false → persist
        assert!(!should_persist(&json!({ "preview": true })));  // preview → skip
        assert!(!should_persist(&json!({ "preview": "true" }))); // string form → skip
    }
```

Delete the old `confirm_accepts_bool_and_string` test (its assertions moved into `bool_flag_accepts_bool_and_string`).

- [ ] **Step 2: Run + confirm fail.** Run: `cargo test -p senseid --bin senseid preview_flag_skips_persist 2>&1 | tail -6`
  Expected: FAIL — `cannot find function 'should_persist'` / `parse_bool_flag` not found.

- [ ] **Step 3: Rename the helper + add `should_persist`.** Replace the `parse_confirm` definition (L245-249):

```rust
/// Parse a truthy flag: a real JSON bool, or the string `"true"` (case-insensitive).
/// The MCP tool layer forwards flags as strings; direct HTTP callers send bools.
/// Shared by the `confirm` and `preview` flags on `recommend_playbook`.
fn parse_bool_flag(v: &serde_json::Value) -> bool {
    v.as_bool()
        .or_else(|| v.as_str().map(|s| s.eq_ignore_ascii_case("true")))
        .unwrap_or(false)
}

/// Whether a `recommend_playbook` call should record a `playbook_run`. Preview
/// calls (the app intake form's recommend leg) classify + recommend without
/// writing a row; the confirm leg persists exactly one.
fn should_persist(body: &serde_json::Value) -> bool {
    !parse_bool_flag(&body["preview"])
}
```

- [ ] **Step 4: Update the persist block + call site.** In `recommend_playbook`, replace the persist block (currently L72-93):

```rust
    // Persist the run unless this is a preview call. Recommend-and-confirm
    // defaults confirmed=false until the caller confirms; the app intake form's
    // recommend leg passes preview=true (classify + recommend, no row written).
    let confirmed = parse_bool_flag(&body["confirm"]);
    let session_id = body["session_id"].as_str().and_then(|s| s.parse().ok());
    if should_persist(&body) {
        if let Err(e) = state
            .pg
            .insert_playbook_run(
                session_id,
                body["feature"].as_str(),
                axes.lifecycle.as_str(),
                axes.intent.as_str(),
                axes.risk.as_str(),
                rec.rule_id,
                &rec.playbook,
                &rec.rationale,
                confirmed,
                Some(classified_by.as_str()),
                model_fallback,
            )
            .await
        {
            tracing::error!("recommend_playbook: insert_playbook_run failed: {e}");
        }
    }
```

- [ ] **Step 5: Add the classified axes to the response.** In the same handler's final `Json(serde_json::json!({ ... }))` (currently L110-119), add three fields (place them right after `"rationale"`):

```rust
    Json(serde_json::json!({
        "playbook": rec.playbook,
        "rationale": rec.rationale,
        "lifecycle": axes.lifecycle.as_str(),
        "intent": axes.intent.as_str(),
        "risk": axes.risk.as_str(),
        "rule": rec.rule_name,
        "defaulted": rec.defaulted,
        "opening_tone": opening_tone,
        "when_to_use": when_to_use,
        "auto_select": auto_select,
        "trust": { "n": trust_n, "ftr": trust_ftr },
    }))
```

- [ ] **Step 6: Run tests + clippy.**
  Run: `cargo test -p senseid --bin senseid 'classify_tests' 2>&1 | tail -8` → PASS (both tests).
  Run: `cargo test -p senseid --bin senseid playbook 2>&1 | tail -6` → PASS (no regressions).
  Run: `cargo clippy -p senseid --all-targets 2>&1 | tail -4` → clean.

- [ ] **Step 7: Commit.**

```bash
git add crates/senseid/src/api/handlers/playbook.rs
git commit -m "feat(senseid): recommend_playbook preview flag (no persist) + classified axes in response"
```

---

### Task 2: App types + api-client methods

**Files:**
- Modify: `app/src/lib/types.ts` (add intake types near the other API DTOs)
- Modify: `app/src/lib/api.ts` (add two named methods to the `senseiApi` returned object, next to `getInsights`)

The app talks to the daemon through the typed `senseiApi(port)` client (named methods, e.g. `getInsights`). Add the two intake calls + their DTOs. `getIntakeGuide` uses the fallback-returning `get<T>` (empty guide on a daemon hiccup — quiet state, never a broken screen). `recommendPlaybook` uses `tryPost<T>` so the form can surface classify/record errors.

- [ ] **Step 1: Add the DTOs to `app/src/lib/types.ts`** (append near the other interfaces, e.g. after `InsightsBoard`):

```ts
/** One playbook in the front-door catalog (from GET /api/playbook/guide). */
export interface IntakePlaybook {
  name: string;
  title: string;
  when_to_use: string;
  opening_tone: string;
  method_ref: string | null;
}

/** One per-axis prompt in the intake guide (kind === "axis"). */
export interface IntakeAxisGuide {
  kind: string;
  axis: string | null;
  prompt: string;
  help: string | null;
}

/** GET /api/playbook/guide — the front-door frame + axis prompts + catalog. */
export interface IntakeGuide {
  frame: string;
  axes: IntakeAxisGuide[];
  playbooks: IntakePlaybook[];
}

/** Proven FTR history for the recommended combo (drives the auto-select badge). */
export interface PlaybookTrust {
  n: number;
  ftr: number;
}

/** POST /api/playbook/recommend — the classified axes + the chosen playbook. */
export interface PlaybookRecommendation {
  playbook: string;
  rationale: string;
  lifecycle: string;
  intent: string;
  risk: string;
  rule: string;
  defaulted: boolean;
  opening_tone: string;
  when_to_use: string;
  auto_select: boolean;
  trust: PlaybookTrust;
}
```

- [ ] **Step 2: Import the DTOs in `app/src/lib/api.ts`.** Add to the existing `import type { ... } from './types.js';` block:

```ts
  IntakeGuide, PlaybookRecommendation,
```

- [ ] **Step 3: Add the two named methods** to the object returned from `senseiApi` (place them right after the `getInsights: (...) => ...,` entry):

```ts
    // ── Front door · Intake ─────────────────────────────────────────────
    // The guide (frame + axis prompts + catalog) for the intake screen.
    // Fallback is the empty guide so a daemon hiccup renders the quiet state.
    getIntakeGuide: () =>
      get<IntakeGuide>('/api/playbook/guide', { frame: '', axes: [], playbooks: [] }),

    // Classify + recommend a playbook. `{ chunk, preview: true }` previews
    // (no row written); `{ lifecycle, intent, risk, confirm: true }` records
    // the confirmed run. tryPost so the form can surface errors.
    recommendPlaybook: (body: Record<string, unknown>) =>
      tryPost<PlaybookRecommendation>('/api/playbook/recommend', body),
```

- [ ] **Step 4: Type-check.** Run: `cd app && bun run check 2>&1 | tail -15`
  Expected: no new errors referencing `types.ts` / `api.ts` (baseline must be clean per zero-errors-policy — if the baseline already has errors, fix them first).

- [ ] **Step 5: Commit.**

```bash
git add app/src/lib/types.ts app/src/lib/api.ts
git commit -m "feat(app): intake DTOs + senseiApi getIntakeGuide/recommendPlaybook"
```

---

### Task 3: `intake.svelte.ts` state module + unit tests

**Files:**
- Create: `app/src/routes/(observatory)/intake/intake.svelte.ts`
- Create: `app/src/routes/(observatory)/intake/intake.spec.svelte.ts`

A runes state class owns the flow: seed the guide, run recommend-preview, honor auto-select (auto-confirm), and confirm. It depends only on a small `IntakeApi` interface (the two methods from Task 2) so tests inject a fake — no network, no `appState`. Mirrors the `insights-board.svelte.js` state-owns-refetch pattern.

- [ ] **Step 1: Write the failing tests.** Create `app/src/routes/(observatory)/intake/intake.spec.svelte.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { IntakeState, type IntakeApi } from './intake.svelte.js';
import type { IntakeGuide, PlaybookRecommendation } from '$lib/types.js';

const GUIDE: IntakeGuide = {
  frame: 'Describe the chunk of work.',
  axes: [{ kind: 'axis', axis: 'intent', prompt: 'What are you doing?', help: null }],
  playbooks: [
    { name: 'debug_flow', title: 'Debug Flow', when_to_use: '', opening_tone: '', method_ref: null },
    { name: 'gsd', title: 'Get Stuff Done', when_to_use: '', opening_tone: '', method_ref: null },
  ],
};

const rec = (over: Partial<PlaybookRecommendation> = {}): PlaybookRecommendation => ({
  playbook: 'debug_flow', rationale: 'a fix', lifecycle: 'stable', intent: 'bug', risk: 'low',
  rule: 'r', defaulted: false, opening_tone: 'careful', when_to_use: '',
  auto_select: false, trust: { n: 0, ftr: 0 }, ...over,
});

// Records every recommendPlaybook body so tests can assert the confirm payload.
function fakeApi(result: PlaybookRecommendation, opts: { ok?: boolean } = {}): IntakeApi & { calls: Record<string, unknown>[] } {
  const calls: Record<string, unknown>[] = [];
  return {
    calls,
    getIntakeGuide: async () => GUIDE,
    recommendPlaybook: async (body) => {
      calls.push(body);
      return opts.ok === false
        ? { ok: false, error: { status: 500, message: 'boom' } }
        : { ok: true, data: result };
    },
  };
}

describe('IntakeState', () => {
  it('resolves the playbook title from the guide catalog', () => {
    const s = new IntakeState(GUIDE);
    s.rec = rec({ playbook: 'gsd' });
    expect(s.playbookTitle).toBe('Get Stuff Done');
  });

  it('falls back to the raw name when the catalog lacks the playbook', () => {
    const s = new IntakeState(GUIDE);
    s.rec = rec({ playbook: 'unknown_pb' });
    expect(s.playbookTitle).toBe('unknown_pb');
  });

  it('preview previews without confirming, then confirm records', async () => {
    const s = new IntakeState(GUIDE);
    s.chunk = 'fix the crash';
    const api = fakeApi(rec());
    await s.recommend(api);
    expect(s.phase).toBe('recommended');
    expect(s.rec?.playbook).toBe('debug_flow');
    // First call is the preview leg.
    expect(api.calls[0]).toEqual({ chunk: 'fix the crash', preview: true });

    await s.confirm(api);
    expect(s.phase).toBe('recorded');
    // Confirm reuses the classified axes (no re-classify) with confirm:true.
    expect(api.calls[1]).toEqual({ lifecycle: 'stable', intent: 'bug', risk: 'low', confirm: true });
  });

  it('auto-selects (auto-confirms) when the daemon says trusted', async () => {
    const s = new IntakeState(GUIDE);
    s.chunk = 'tweak a low-risk thing';
    const api = fakeApi(rec({ auto_select: true, trust: { n: 12, ftr: 0.9 } }));
    await s.recommend(api);
    // Preview then an automatic confirm — no manual confirm() call.
    expect(api.calls.length).toBe(2);
    expect(api.calls[1]).toEqual({ lifecycle: 'stable', intent: 'bug', risk: 'low', confirm: true });
    expect(s.phase).toBe('recorded');
  });

  it('ignores an empty chunk', async () => {
    const s = new IntakeState(GUIDE);
    s.chunk = '   ';
    const api = fakeApi(rec());
    await s.recommend(api);
    expect(api.calls.length).toBe(0);
    expect(s.phase).toBe('describe');
  });

  it('surfaces a recommend error', async () => {
    const s = new IntakeState(GUIDE);
    s.chunk = 'fix the crash';
    const api = fakeApi(rec(), { ok: false });
    await s.recommend(api);
    expect(s.phase).toBe('error');
    expect(s.error).toBe('boom');
  });

  it('reset returns to the describe phase', async () => {
    const s = new IntakeState(GUIDE);
    s.chunk = 'x'; s.rec = rec(); s.phase = 'recorded';
    s.reset();
    expect(s.phase).toBe('describe');
    expect(s.chunk).toBe('');
    expect(s.rec).toBeNull();
  });
});
```

- [ ] **Step 2: Run + confirm fail.** Run: `cd app && bun run test intake.spec 2>&1 | tail -15`
  Expected: FAIL — cannot resolve `./intake.svelte.js`.

- [ ] **Step 3: Implement the state module.** Create `app/src/routes/(observatory)/intake/intake.svelte.ts`:

```ts
/**
 * Intake screen state — owns the freeform → classify → recommend → confirm flow.
 *
 * Depends only on `IntakeApi` (the two `senseiApi` methods) so it unit-tests
 * with an injected fake — no network, no appState. Mirrors the state-owns-the-
 * flow pattern of insights-board.svelte.js.
 */
import type { ApiResult } from '$lib/api.js';
import type { IntakeGuide, PlaybookRecommendation } from '$lib/types.js';

/** The slice of the daemon client this state needs (lets tests inject a fake). */
export interface IntakeApi {
  getIntakeGuide(): Promise<IntakeGuide>;
  recommendPlaybook(body: Record<string, unknown>): Promise<ApiResult<PlaybookRecommendation>>;
}

export type IntakePhase = 'describe' | 'loading' | 'recommended' | 'recorded' | 'error';

export class IntakeState {
  guide = $state<IntakeGuide>({ frame: '', axes: [], playbooks: [] });
  chunk = $state('');
  phase = $state<IntakePhase>('describe');
  rec = $state<PlaybookRecommendation | null>(null);
  error = $state('');

  constructor(guide: IntakeGuide) {
    this.guide = guide;
  }

  /** Human title for the recommended playbook, from the guide catalog. */
  get playbookTitle(): string {
    const name = this.rec?.playbook ?? '';
    return this.guide.playbooks.find((p) => p.name === name)?.title ?? name;
  }

  /** Recommend leg: classify + recommend without writing a row (preview). */
  async recommend(api: IntakeApi): Promise<void> {
    const chunk = this.chunk.trim();
    if (!chunk) return;
    this.phase = 'loading';
    this.error = '';
    const res = await api.recommendPlaybook({ chunk, preview: true });
    if (!res.ok) {
      this.phase = 'error';
      this.error = res.error.message || 'Could not classify the chunk.';
      return;
    }
    this.rec = res.data;
    this.phase = 'recommended';
    // Auto-select-on-trust: skip the manual confirm when the daemon trusts it.
    if (res.data.auto_select) await this.confirm(api);
  }

  /** Confirm leg: record one confirmed run, reusing the classified axes. */
  async confirm(api: IntakeApi): Promise<void> {
    const r = this.rec;
    if (!r) return;
    const res = await api.recommendPlaybook({
      lifecycle: r.lifecycle,
      intent: r.intent,
      risk: r.risk,
      confirm: true,
    });
    if (!res.ok) {
      this.phase = 'error';
      this.error = res.error.message || 'Could not record the playbook.';
      return;
    }
    this.phase = 'recorded';
  }

  reset(): void {
    this.chunk = '';
    this.rec = null;
    this.error = '';
    this.phase = 'describe';
  }
}
```

- [ ] **Step 4: Svelte MCP autofixer.** Run the autofixer over `app/src/routes/(observatory)/intake/intake.svelte.ts` (`mcp__plugin_svelte_svelte__svelte-autofixer`); apply fixes; re-run until clean.

- [ ] **Step 5: Run tests.** Run: `cd app && bun run test intake.spec 2>&1 | tail -15` → PASS (all 7).

- [ ] **Step 6: Commit.**

```bash
git add "app/src/routes/(observatory)/intake/intake.svelte.ts" "app/src/routes/(observatory)/intake/intake.spec.svelte.ts"
git commit -m "feat(app): IntakeState — freeform→classify→recommend→confirm flow + tests"
```

---

### Task 4: Intake route — loader + page + recommendation card

**Files:**
- Create: `app/src/routes/(observatory)/intake/+page.ts`
- Create: `app/src/routes/(observatory)/intake/+page.svelte`

The loader fetches the guide (like `insights/+page.ts`). The page seeds `IntakeState`, wires the textarea + Recommend button, and renders the recommendation card. Styling uses canonical tokens only (no literal px; Zen-Sumi spacing per `docs/architecture/frontend-svelte-guidelines.md`); `PageHeader` matches sibling screens.

- [ ] **Step 1: Create the loader** `app/src/routes/(observatory)/intake/+page.ts`:

```ts
import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

/** Load the front-door guide (frame + axis prompts + catalog). The state module
 *  owns recommend/confirm thereafter. */
export const load: PageLoad = async () => {
  const guide = await senseiApi(appState.port).getIntakeGuide();
  return { guide };
};
```

- [ ] **Step 2: Create the page** `app/src/routes/(observatory)/intake/+page.svelte`:

```svelte
<script lang="ts">
  import { PageHeader } from '$lib/components';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import { IntakeState } from './intake.svelte.js';

  let { data } = $props();

  const api = senseiApi(appState.port);
  const intake = new IntakeState(data.guide);

  function recommend(): void { void intake.recommend(api); }
  function confirm(): void { void intake.confirm(api); }
</script>

<div class="flex flex-col gap-3 p-4 max-w-2xl">
  <PageHeader kanji="門" eyebrow="Sensei" title="Intake" description="Start a chunk of work" variant="h1" />

  {#if intake.phase !== 'recorded'}
    <section class="flex flex-col gap-2">
      {#if intake.guide.frame}
        <p class="text-sm text-ink-soft m-0">{intake.guide.frame}</p>
      {/if}
      <textarea
        class="w-full min-h-32 rounded bg-paper-soft border border-paper-edge py-2 px-3 text-sm text-ink"
        placeholder="Describe the work chunk…"
        bind:value={intake.chunk}
        disabled={intake.phase === 'loading'}
      ></textarea>
      <div class="flex justify-end">
        <button
          class="text-sm bg-ink text-paper rounded-sm py-1 px-3 border-none cursor-pointer disabled:opacity-50"
          onclick={recommend}
          disabled={intake.phase === 'loading' || !intake.chunk.trim()}
        >
          {intake.phase === 'loading' ? 'Reading…' : 'Recommend a playbook'}
        </button>
      </div>
    </section>
  {/if}

  {#if intake.phase === 'error'}
    <p class="text-sm bg-danger-soft text-danger border border-danger rounded py-2 px-3 m-0">{intake.error}</p>
  {/if}

  {#if intake.rec && (intake.phase === 'recommended' || intake.phase === 'recorded')}
    {@const r = intake.rec}
    <section class="flex flex-col gap-2 rounded bg-paper-soft border border-paper-edge py-2 px-3">
      <div class="flex items-center justify-between gap-2">
        <h2 class="text-sm font-medium text-ink m-0">{intake.playbookTitle}</h2>
        {#if r.auto_select}
          <span class="text-xs bg-success-soft text-success rounded-sm py-1 px-2">
            trusted · FTR {r.trust.ftr.toFixed(2)} over {r.trust.n}
          </span>
        {/if}
      </div>
      <p class="text-sm text-ink-soft m-0 leading-snug">{r.rationale}</p>
      {#if r.opening_tone}
        <p class="text-xs italic text-ink-faint m-0">{r.opening_tone}</p>
      {/if}
      <div class="flex flex-wrap gap-2 text-xs text-ink-soft">
        <span class="border border-paper-edge rounded-sm py-1 px-2">{r.lifecycle}</span>
        <span class="border border-paper-edge rounded-sm py-1 px-2">{r.intent}</span>
        <span class="border border-paper-edge rounded-sm py-1 px-2">{r.risk}</span>
      </div>

      {#if intake.phase === 'recorded'}
        <div class="flex items-center justify-between gap-2">
          <p class="text-sm text-success m-0">
            {r.auto_select ? 'Auto-selected and recorded.' : 'Recorded.'}
          </p>
          <button class="text-xs text-accent bg-transparent border-none cursor-pointer" onclick={() => intake.reset()}>
            New intake
          </button>
        </div>
      {:else}
        <div class="flex justify-end">
          <button class="text-sm bg-ink text-paper rounded-sm py-1 px-3 border-none cursor-pointer" onclick={confirm}>
            Use this playbook
          </button>
        </div>
      {/if}
    </section>
  {/if}
</div>
```

Tokens above are copied from `app/src/routes/(observatory)/insights/RecCard.svelte` (card = `rounded bg-paper-soft border border-paper-edge`; primary button = `bg-ink text-paper rounded-sm`; text = `text-ink`/`text-ink-soft`/`text-ink-faint`/`text-accent`; soft states = `bg-success-soft`/`bg-danger-soft`) — do NOT substitute `bg-surface`/`border-line`/`text-on-accent` (those don't exist in this design system). `PageHeader` props are `{ kanji, eyebrow, title, description, variant }` (verified against `PageHeader.harness.svelte`) — there is no `subtitle`.

- [ ] **Step 3: Svelte MCP autofixer.** Run the autofixer over BOTH `+page.svelte` and `+page.ts` (`mcp__plugin_svelte_svelte__svelte-autofixer`); apply every fix; re-run until clean. If the autofixer or `bun run check` flags any class/token as unknown, replace it with the nearest canonical token used in sibling observatory pages (grep them) — do not invent tokens.

- [ ] **Step 4: Type-check.** Run: `cd app && bun run check 2>&1 | tail -15` → no new errors. Confirm `./$types.js` resolves (SvelteKit generates it once the route dir exists; if `check` regenerates types it may need a second run).

- [ ] **Step 5: Commit.**

```bash
git add "app/src/routes/(observatory)/intake/+page.ts" "app/src/routes/(observatory)/intake/+page.svelte"
git commit -m "feat(app): intake route — guide loader + freeform form + recommendation card"
```

---

### Task 5: Observatory nav — "Intake" entry

**Files:**
- Modify: `app/src/routes/(observatory)/observatory-nav.ts` (`buildNavItems`, the anchors block)
- Modify: `app/src/routes/(observatory)/observatory-nav.spec.ts`

Intake is the front door — the first anchor, above Today. The rail is data-driven (`buildNavItems`) and unit-tested (`observatory-nav.spec.ts`); `resolveActiveHref` picks it up automatically via `allHrefs()`.

- [ ] **Step 1: Write the failing test.** In `app/src/routes/(observatory)/observatory-nav.spec.ts`, inside the first `it("shows anchors ...")` test, add after the existing `topHrefs` assertions:

```ts
    // Intake is the front door — the leading anchor, above Today.
    expect(topHrefs[0]).toBe("/intake");
    expect(byHref(entries, "/intake")?.text).toBe("Intake");
```

- [ ] **Step 2: Run + confirm fail.** Run: `cd app && bun run test observatory-nav 2>&1 | tail -12`
  Expected: FAIL — `topHrefs[0]` is `"/"` (Today), `/intake` undefined.

- [ ] **Step 3: Add the nav entry.** In `app/src/routes/(observatory)/observatory-nav.ts`, in `buildNavItems`, prepend the Intake anchor as the first entry in the `entries` array (before `link("家", "Today", "/")`):

```ts
    // Front door — where a chunk of work starts; the leading anchor.
    link("門", "Intake", "/intake"),
    link("家", "Today", "/"),
```

- [ ] **Step 4: Run tests.**
  Run: `cd app && bun run test observatory-nav 2>&1 | tail -12` → PASS.
  Run the autofixer is not needed (`.ts`, no Svelte markup), but run: `cd app && bun run check 2>&1 | tail -8` → clean.

- [ ] **Step 5: Commit.**

```bash
git add "app/src/routes/(observatory)/observatory-nav.ts" "app/src/routes/(observatory)/observatory-nav.spec.ts"
git commit -m "feat(app): observatory rail — Intake front-door anchor"
```

---

## Final verification (whole plan)

- [ ] **Rust:** `cargo test -p senseid --bin senseid 2>&1 | tail -6` — green; `cargo clippy -p senseid --all-targets 2>&1 | tail -4` — clean.
- [ ] **App:** `cd app && bun run test 2>&1 | tail -12` — green (incl. `intake.spec`, `observatory-nav`); `cd app && bun run check 2>&1 | tail -8` — zero errors (zero-errors-policy).
- [ ] **Backend end-to-end (daemon on :7744 if running):**
  - Preview does NOT write a row and returns the axes:
    ```bash
    before=$(psql -d sensei -tAc "select count(*) from sensei.playbook_run")
    curl -s localhost:7744/api/playbook/recommend -H 'content-type: application/json' \
      -d '{"chunk":"fix a null deref in the parser","preview":true}' | tee /dev/stderr | grep -q '"lifecycle"'
    after=$(psql -d sensei -tAc "select count(*) from sensei.playbook_run")
    test "$before" = "$after" && echo "OK: preview wrote no row"
    ```
  - Confirm writes exactly one confirmed row:
    ```bash
    curl -s localhost:7744/api/playbook/recommend -H 'content-type: application/json' \
      -d '{"lifecycle":"stable","intent":"bug","risk":"low","confirm":true}' >/dev/null
    psql -d sensei -tAc "select confirmed from sensei.playbook_run order by created_at desc limit 1"  # → t
    ```
- [ ] **Visual (Jerry):** flag for smoke via `make app-dev` → open the Observatory → **Intake** in the rail → type a chunk → recommend → card renders with axes + confirm → "Recorded." (Tauri app; cannot be auto-verified.)
- [ ] **Whole-plan review** (subagent): coherence across the backend flag, the client, the state module, the route, and the nav; token fidelity; no session-id leak (form sends none — recorded session-less by design).

## Self-review notes (author)

- **Spec coverage:** `preview` flag + axes-in-response ✓ T1; typed client ✓ T2; freeform→classify→recommend→confirm-persist + auto-select ✓ T3; route/frame/card ✓ T4; "Intake" nav entry, listed first ✓ T5. Session-less-by-default ✓ (T3 confirm body sends no `session_id`; design notes app has no session field — verified in `appstate.svelte.ts`).
- **Type consistency:** `IntakeApi.recommendPlaybook(body) -> Promise<ApiResult<PlaybookRecommendation>>` matches the `tryPost` return (T2) and the fake (T3). `PlaybookRecommendation.{lifecycle,intent,risk}` (T2) are the fields T1 adds to the response and T3's confirm leg reuses. `playbookTitle` reads `guide.playbooks[].{name,title}` — exactly `list_playbooks`' shape.
- **DRY:** `parse_confirm` generalized to `parse_bool_flag`, reused for `confirm` + `preview` (no second parser). The app reuses `senseiApi`'s `get`/`tryPost`; no bespoke fetch.
- **No new schema, no new MCP tool, no DDL.** The daemon change is additive + backward-compatible (the CLI's two-call flow and existing callers are unaffected: absent `preview` → persists as before).
