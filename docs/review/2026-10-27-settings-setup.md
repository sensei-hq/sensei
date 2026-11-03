# Review · Settings window, Setup wizard, Health screens — 2026-10-27

Adversarial, read-only functional review for a screen-by-screen walk-through.
Nothing was modified; the daemon (v0.21.0, `127.0.0.1:7744`) was called with GET only.

**Scope.** The Settings window (`app/src/routes/(preferences)/`, opened by Preferences… ⌘,), the
setup wizard (`(config)/setup/`), `(health)/` (health, logs, upgrade), and the identity surfaces
(`lib/personas.svelte.ts`, `SignInOverlay`, `SignInPending`).

**How to read it.** Each screen has: purpose → endpoints (with live `curl` evidence) →
interactions → mockup/spec → a checklist. Severities: **bug** (wrong or lying behaviour),
**gap** (missing behaviour the screen implies), **polish**, **question** (needs your decision).
Paths are relative to `app/src/` unless stated.

**The cross-cutting fact behind several findings: the Settings window is a separate webview
with its own JS heap.** Every singleton (`appState`, `wizardState`, `healthState`, `personaList`,
`forgeAuth`, `scanState`) is a fresh copy there. `healthState` is never initialised in that
window (only `/health` calls `healthState.init()`), so it stays `'checking'`. `wizardState.load()`
only runs when `healthState.isOk` (`lib/wizard-state.svelte.ts:360-368`), so **in the Settings
window `wizardState` is never hydrated**. Any settings section that reads `wizardState` slices
other than the ones it re-reads itself sees initial empty state.

---

## 0 · Window shell, rail, and what the root layout does in it

**Files.** `(preferences)/+layout.svelte`, `+layout.ts`, `settings/+layout.svelte`,
`settings/settings-nav.ts`, `settings/SettingsSidebar.svelte`, `src-tauri/src/lib.rs:300-325`,
`routes/+layout.svelte`, `lib/boot-gate.ts`.

**What it does.** Tauri opens a 1000×720 window labelled `settings` at `/settings/general`
(`lib.rs:302-318`). The boot gate exempts `/settings/*` (`boot-gate.ts:22-24`). A link out of
settings is cancelled and re-emitted to the main window as `dev-navigate`
(`(preferences)/+layout.svelte:21-27`). The rail is a rokkit `List` with five groups (You,
Sources, Reasoning, Measurement, Dōjō) plus Extensions.

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 0.1 | bug | **The root layout runs in the Settings window too**, so that window starts its own `forgeAuth` poller and its own `SignInOverlay`. `forgeAuth`'s "one automatic renewal per window" guard (`#armed`) is per instance, so with Settings open a renewal-due credential can open **two** browser tabs (one per window; more with workspace windows open). | `routes/+layout.svelte:80-90`, `lib/forge-auth.svelte.ts:131-139` |
| 0.2 | bug | Menu events go to every window (`app.emit`). **View → Identities… opens the overlay in both windows at once.** A navigation item pressed while Settings has focus also goes to both: Settings does `goto` → cancels → re-emits to main, so main navigates twice. | `lib.rs:262-270`, `routes/+layout.svelte:99-108` |
| 0.3 | bug | **A preference changed in Settings does not reach the main window.** The main window reads `appState.config` once (observatory `+layout.ts:7-10`) and has no listener for config changes. Glyphs ("Kanji or icons … across the app") and display name stay stale in main until it reloads. | `(observatory)/+layout.ts`, `routes/+layout.svelte:21` |
| 0.4 | bug | If `appState.load()` fails, `config` becomes `{}` and `loaded=true`, a fabricated default. The rail then draws kanji even when the user chose icons, and nothing says the read failed. | `lib/appstate.svelte.ts:160-171` |
| 0.5 | gap | The rail differs from the spec: no search, no review cues, no restore-default, no Profile or Assignments panes. `GET /api/preferences` → **404**, `/api/preferences/cues` → **404**. | `docs/spec/screen/preferences.md` |
| 0.6 | polish | No "services" footer and no "Save & close", both of which the mockup has. The mockup's model is a scan-first preferences window with a step counter. | `docs/mockups/Sensei/assets/screens/o5-preferences.png`, `hq/_prefs.png` |

**Checklist**
- [ ] Open Settings (⌘,), then View → Identities… — expect one overlay in the focused window. Code suggests **one in each window** (bug 0.2).
- [ ] In Settings · General switch Glyphs to Icons, then look at the main window rail — expect icons. Code suggests **kanji until main reloads** (bug 0.3).
- [ ] With Settings open, wait for a credential to enter its renewal window (or watch the daemon log for two `POST /api/auth/signin` in one poll cycle) — expect one browser tab. Code suggests **one tab per window** (bug 0.1).
- [ ] Click "Dōjō · Connections" on the Dōjō page — expect the main window to come forward on `/dojo/connections` and Settings to stay put.

---

## 1 · Settings · General

**Files.** `settings/general/+page.{ts,svelte}`, `general-state.svelte.ts`, `settings-reads.ts`,
`preferences-form.ts`.

**Purpose.** Display name, morning digest, correction tone, glyphs, and switches for nudge-on-regression,
anonymized telemetry and the welcome greeting. Changes save automatically.

**Endpoints.** `GET /api/config` (200; contains `setup.preferences` JSON, `user_name=Jerry`,
`ui.glyphs=icon`). Writes go through `appState.setConfigs` → `trySetConfig`, so `setup.preferences`,
`user_name` and `ui.glyphs` are all saved together.

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 1.1 | bug | **Five of the seven controls are inert.** `digestCadence`, `correctionAggressiveness`, `nudgeOnRegression`, `anonymizedTelemetry` and `showWelcome` are saved, but nothing in the app or daemon reads them (rg finds only defaults, contracts and mocks; no `crates/` reader). The screen says "saved" for settings that change nothing. | `rg` over `app/src` minus `(preferences)`, and `crates/` |
| 1.2 | bug | **Every keystroke in Display name sends a full config write** (`oninput` → `persist`) with no debounce. Writes can land out of order, and `setConfigs`' rollback restores the `prev` snapshot taken at call time. A late failure can therefore roll the cache back over a later successful write. | `general/+page.svelte:91-94`, `lib/appstate.svelte.ts:88-97` |
| 1.3 | polish | Corrupt `setup.preferences` JSON silently becomes the defaults. The next save then writes those defaults back over the real values, with no error shown. | `preferences-form.ts:12-15,36-38` |
| 1.4 | polish | Saving an empty display name is allowed, and the greeting then falls back to the system user name. | `general-state.svelte.ts:39-55` |

**Checklist**
- [ ] Type a 10-character name quickly — expect one save and "saved". Code suggests about 10 PUTs (watch the daemon log); the final state should still be right but is not guaranteed (1.2).
- [ ] Turn Morning digest to Off, then look at Today — expect a quieter dashboard. Code suggests **no change**: nothing reads it (1.1). The same applies to Correction tone, Nudge, Telemetry and Welcome greeting.
- [ ] Stop the daemon and change a toggle — expect a warning-coloured error where "saved" normally appears, and the toggle to snap back. Code: the error shows, **but the toggle does not snap back** (prefs keep the clicked value; only `appState` rolls back).

---

## 2 · Settings · Assistants (+ Conversation history consent)

**Files.** `settings/assistants/+page.{ts,svelte}`, `settings-reads.ts:29-40`,
`lib/components/settings/TranscriptConsentCard.svelte`, `transcript-consent.svelte.ts`.

**Endpoints.** `GET /api/assistants/detect` (200, 8 rows). Each row has
`{id,name,family,installed,configured,config_path}`. There is **no `version` field**, so the
version column never renders. `GET /api/transcripts/consent` (200, 6 sources, all `consented:true`).
Writes: `PUT /api/transcripts/consent/{source}`.

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 2.1 | bug | **Assistants that are not installed are listed as "detected".** `readAssistants` drops `installed`, and the row label is `configured ? "configured" : "detected"`. Live: Cursor, Windsurf and Kiro are `installed:false` but would show "detected" under "AI coding tools detected on this machine". | `settings-reads.ts:33-38`, `assistants/+page.svelte:51-53`; curl `/api/assistants/detect` |
| 2.2 | gap | The page is read-only. Its only remedy is "Run the setup wizard", and that is a main-window route. Settings has no configure or unconfigure control. | `assistants/+page.svelte:30-33` |
| 2.3 | bug | **A refused consent toggle can stay flipped on screen.** The switch is bound one-way with `onchange`, but the `Switch` wrapper keeps its own `$bindable` value. When the PUT fails, `sources` does not change, so nothing re-renders the switch at the stored value. Metrics solved the same problem with `{#key revision}`. | `TranscriptConsentCard.svelte:44-49`, `lib/components/Switch.svelte:5,30-36`, cf. `metrics/+page.svelte:155-162` |
| 2.4 | question | Consent defaults to **on** (daemon test `everything_is_consented_until_someone_says_no`; the card text says "On unless you turn it off"). The state file's header and the wizard comment say it "starts off". Decide which is intended; the comment is wrong today. | `transcript-consent.svelte.ts:5-7` vs `:42`; `setup/assistants/+page.svelte:119-120` |
| 2.5 | polish | The consent list (6 sources, including Copilot CLI) and the assistant list (8, no Copilot) disagree, and consent shows "reading history" for Cursor, which is not installed. | curl both endpoints |

**Checklist**
- [ ] Open Settings · Assistants — expect only installed tools. Code suggests **Cursor / Windsurf / Kiro shown as "detected"** (2.1).
- [ ] Stop the daemon, toggle a consent switch — expect the switch to return to its prior position with an error. Code suggests the **error shows but the switch stays flipped** (2.3).

---

## 3 · Settings · Roots

**Files.** `settings/roots/+page.svelte`, `lib/components/settings/RootsSection.svelte`,
`root-removal.svelte.ts`, `root-exclusions.svelte.ts`. The wizard Roots stage uses the same section.

**Endpoints.** `GET /api/scan/roots` (200, 4 roots, all `watching`, 1 repository each).
`GET /api/scan/roots/{id}/repositories` (200, e.g. `[{name:"dbd",kind:"git"}]`). Writes:
`POST /api/scan/roots`, `DELETE /api/scan/roots/{id}?repositories=keep|remove`,
`POST /api/scan/prune`, and the exclusion endpoints.

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 3.1 | bug | **Removing a root confirms nothing.** On `outcome==="removed"` the row is filtered out of `roots` (`:37-38`). The removal panel and its result message ("Removed N folders, …") live *inside* that row's `{#each}` block (`:163`), so they unmount the moment the message is set. The user sees the row vanish with no summary. | `RootsSection.svelte:34-44,163-224`, `root-removal.svelte.ts:111-117` |
| 3.2 | gap | **No progress while removing** (confirms the known item). One DELETE call, no SSE, no "Removing…" label. The buttons only go disabled (`canDecide=false` while `busy`). A large prune looks frozen. | `root-removal.svelte.ts:102-106`, `RootsSection.svelte:199-213` |
| 3.3 | bug | "remove this one" on a repository **prunes immediately with no confirm**. It deletes the indexed data and adds an exclusion. | `RootsSection.svelte:189-194`, `root-removal.svelte.ts:120-132` |
| 3.4 | gap | A root "kept" (paused) has **no resume control**. Its chip reads "paused · not syncing" with only × left. | `RootsSection.svelte:125-130` |
| 3.5 | gap | Add: the daemon answers with whether it is `scanning`/`watching` and declines to scan a missing path (`crates/senseid/src/api/handlers/workspace.rs:455-500`). The UI ignores this and mints a row with `status:'scanning'`, which renders as "recursive". A typo'd path looks accepted. There is no scan progress in Settings either. The api.ts comment "Does not start scanning" (`lib/api.ts:723`) is stale. | `lib/wizard-state.svelte.ts:671-688` |
| 3.6 | polish | When the roots read fails, the page shows a warning **and** an empty list with the Add box, which reads as "no roots". | `roots/+page.svelte:12-26` |
| 3.7 | gap (mockup) | The mockup has Browse… (folder picker), drag-a-folder, and a "remove" text button. The app has a text field only. | `o5-preferences.png`, `o3-first-run-scan.png` |

**Checklist**
- [ ] Add a non-existent path (`~/nope`) — expect "path does not exist / not scanning". Code suggests it is **accepted silently** (3.5).
- [ ] Remove a test root with "Remove everything indexed here" — expect a busy indicator, then "Removed N …". Code suggests **buttons grey out and then the row just disappears** (3.1, 3.2).
- [ ] "Stop syncing, keep the data" — expect "paused · not syncing" and a way to resume. Code: **no resume** (3.4).

---

## 4 · Settings · Workspaces

**Files.** `settings/workspaces/+page.svelte`, `lib/components/settings/WorkspacesSection.svelte`,
`workspaces-section-state.svelte.ts`.

**Endpoints.** `GET /api/workspaces` (200, **133 workspaces; 129 have no git/subtree repository**;
folder kinds: folder 855, module 49, git 4). Writes: `PATCH`-style `tryUpdateWorkspace`
(rename) and `tryUpdateFolder` (role).

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 4.1 | polish | No "saved" feedback for a role or a rename. Only failures are shown. | `workspaces-section-state.svelte.ts:111-138` |
| 4.2 | polish | Blanking the name input leaves it blank on screen: the rename is skipped, but the input is not restored. | `:113-114`, `WorkspacesSection.svelte:37-41` |
| 4.3 | question | 129 of 133 workspaces fall under "no repository on this machine". Is that expected for this estate (the narrowed watch roots), or should those be hidden or pruned? | curl |

**Checklist**
- [ ] Change a repository's role, reload Settings — expect it to stick (code: it does, via the daemon).
- [ ] Clear a workspace name and tab away — expect the old name back. Code: **stays blank** until reload.

---

## 5 · Settings · Libraries

**Files.** `settings/libraries/+page.svelte`, `lib/components/settings/LibrariesSection.svelte`,
`lib/wizard-state.svelte.ts:811-838`.

**Endpoints.** `GET /api/libs` (200, **209 libraries; 206 have `pageCount: 0`**, e.g. `serde_json`,
`serde`, `tokio`, `svelte`). No write endpoint exists for the wrap choice.

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 5.1 | bug | **In the Settings window this page likely shows "No libraries detected yet. Run the scan stage first."** `refreshLibraries` keeps only libs whose `repos` intersect `wizardState.workspaces` folder names, and that slice is never hydrated in the Settings window (see intro). The known "N · sensei will wrap" symptom matches the old in-main-window path, where `wizardState` *was* hydrated. Verify which you see. | `wizard-state.svelte.ts:821-826`, `:360-368` |
| 5.2 | bug | **Where it does render, it misinforms** (confirms the known item). The header says "Detected · sensei will wrap" and the badge "N will be wrapped" for every library. `enabled` defaults to `true` (`:837`), and `pageCount` is read but never shown. Live: 206 of 209 have **no docs**. The wizard's Done page then claims "docs & code indexed by sensei". | `LibrariesSection.svelte:52-59`, `setup/done/+page.svelte:33-35` |
| 5.3 | bug | **The Wrap switch persists nothing and affects nothing.** It mutates in-memory `wizardState` only. The only writer (`COMMIT_HANDLERS.libraries`, writing `setup.libraries`) belongs to a wizard stage that no longer exists (`(config)/stages.ts`: welcome/assistants/roots/scan/done). **The daemon never reads `setup.libraries`** (no hit in `crates/`). Live config has `setup.libraries = "done"`: the stage marker at `wizard-state.svelte.ts:931` overwrote the payload under the same key, so any old selection is gone and `mapLibraries` parses "done" as empty. | `LibrariesSection.svelte:94-97`, `wizard-state.svelte.ts:219-231,922-935` |
| 5.4 | gap (mockup) | The mockup shows a per-library status ("docs indexed / partial / schema only"), call counts, a detail pane, "+ add library", and filters. None are present. | `12-libraries.png` |

**Checklist**
- [ ] Open Settings · Libraries in the Settings window — note whether it is **empty** (5.1) or a list.
- [ ] If a list: find `serde_json` — expect "no docs held". Code shows it as "will be wrapped" (5.2).
- [ ] Flip a Wrap switch, close and reopen Settings — expect it to stick. Code: **reverts to on** (5.3).

---

## 6 · Settings · Instruments

**Files.** `settings/instruments/+page.svelte`, `lib/components/settings/InstrumentsSection.svelte`.

**Endpoints.** `GET /api/instruments` (200: `total 6`, `stack: []`, `workspace_count`; only
`playwright-mcp` is installed and selected; none recommended).

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 6.1 | correction | **The switch is rokkit's.** `InstrumentsSection` uses `$lib/components/Switch.svelte`, the same wrapper over `@rokkit/ui` Switch that General, Metrics and Consent use. The non-rokkit control in Settings is the **native checkbox on Extensions** (`extensions/+page.svelte:73-80`). | `InstrumentsSection.svelte:5`, `Switch.svelte:2` |
| 6.2 | bug | **"Install X" switches and "N MCPs to install" install nothing.** The selection lives in in-memory `wizardState`. Its writer (`setup.instruments`) belongs to a removed wizard stage, the live key is `"done"`, and no install call exists. | `InstrumentsSection.svelte:76-78,110,143`, `wizard-state.svelte.ts:232-238` |
| 6.3 | bug | "Detected in your stack" chips come from `wizardState.workspaces`, which is empty in the Settings window, so the strip never renders. The daemon also returns `stack: []`. | `InstrumentsSection.svelte:28-41` |
| 6.4 | polish | The empty-state copy says "the daemon has no endpoint exposing recommendations yet". `/api/instruments` exists. Stale. | `:66-72` |
| 6.5 | gap (mockup) | The mockup is a tool playground (tree of MCPs → tools → try-it form). Different surface; no mockup for a settings pane. | `13-instruments.png` |

**Checklist**
- [ ] Flip "Install GitHub MCP" on, reopen Settings — expect it installed or at least remembered. Code: **neither** (6.2).

---

## 7 · Settings · Providers

**Files.** `settings/providers/+page.svelte`, `lib/components/settings/ProvidersSection.svelte`,
`wizard-state.svelte.ts:874-907`.

**Endpoints.** `GET /api/gateway/routers` (200, 8 routers; only Ollama `configured` and it needs no key).
Writes: `POST /api/gateway/routers/{id}/key`, `DELETE …/key`.

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 7.1 | bug | **Clear deletes a Keychain key with no confirm, and its failure is ignored.** `clearInferenceRouterKey` drops the `ApiResult` and re-reads. A refused delete gives no message, and a failed re-read throws an unhandled rejection from the click handler. | `ProvidersSection.svelte:126-131`, `wizard-state.svelte.ts:903-907` |
| 7.2 | polish | A save error shows `statusText` (e.g. "Bad Request"), not the daemon's reason, because `tryPost` keeps only the status text. | `ProvidersSection.svelte:39-41`, `lib/api.ts:149-163` |
| 7.3 | polish | "1 of 8 configured" counts Ollama, which needs no key. | `:12,66` |

**Checklist**
- [ ] Paste an invalid key, Save — expect a specific reason. Code: **generic status text** (7.2).
- [ ] With a key configured, click Clear — expect a confirm. Code: **immediate delete** (7.1).

---

## 8 · Settings · Inference (assignments + local models)

**Files.** `settings/inference/+page.svelte`, `InferenceAssignmentsPanel.svelte`,
`LocalModelsPanel.svelte`, `local-models.svelte.ts`.

**Endpoints.** `GET /api/gateway/chains` (200, 9 chains; roles: inference→`reasoning`,
consolidation→`summarize`, embedding→`embed`, voice unassigned).
`GET /api/gateway/models/provision/status` (200: `all-minilm` ready, `gemma2:2b` ready,
**`gemma4:latest` absent**, `llama-cpp-chat-default` absent). Writes: chain role PUT, member move
and delete, `provisionModel`.

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 8.1 | bug | **Removing a chain member cannot be undone from the UI.** The × fires with no confirm. The comment says adding happens "in the wizard's fuller Available-models picker", but that stage no longer exists, and `addGatewayChainModel` (`lib/api.ts:971`) has **no caller**. | `InferenceAssignmentsPanel.svelte:159-162,194-202` |
| 8.2 | bug | **A role change is partly applied on failure.** It clears the old chain's role and then sets the new one. If the second write fails, the role is left **unassigned** on the daemon and the error mentions only the set. | `:66-83` |
| 8.3 | gap | Chain members show `modelName` only. Live, `reasoning` lists **gemma4 twice** and `embed` lists **all-minilm twice** (different routers, and `routerName` holds a UUID, not a name). The primary `gemma4` is **not pulled** (provision status `absent`), and `claude`/`gpt` members have no configured key. None of this is shown. | curl `/api/gateway/chains`, `/api/gateway/routers` |
| 8.4 | polish | After a 501 on Pull, the notice replaces the **whole model list**. | `LocalModelsPanel.svelte:36-37` |
| 8.5 | gap (spec) | The spec calls for a hardware tier chip, budget strip, circuit state, add/delete model, and per-chain timeout. None are present. | `docs/spec/screen/settings-inference.md` |

**Checklist**
- [ ] Remove the last member of a test chain — expect a confirm and a way to add it back. Code: **neither** (8.1).
- [ ] Read the reasoning chain — can you tell the two `gemma4` rows apart, or see that gemma4 is not pulled? Code: **no** (8.3).

---

## 9 · Settings · Metrics

**Files.** `settings/metrics/+page.{ts,svelte}`, `metric-status-state.svelte.ts`.

**Endpoints.** `GET /api/metrics/status/summary` (200: **157 repositories**). The worst-reason
headline, recomputed with the page's own ranking, is **"Calendar days are unsettled further back than
yesterday" (fault, red) for 152** and "This metric has been retired" for 5. Only 4 roots are watched
(dbd, gateway, sensei, torii). `GET /api/metrics/status?repo=` per selection. Write:
`PATCH /api/dojo/metric-activation`.

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 9.1 | bug | **About 150 repositories nobody watches show a red fault** (confirms the known item; 152 today, not 154). `behind` is the only `fault` and leads the ranking. The summary carries no watched/unwatched fact, so the client cannot say "not watched". The fix needs a daemon field or reason code. | curl summary; `metric-status-state.svelte.ts:74-112` |
| 9.2 | bug | **Healthy watched repos headline "This metric has been retired".** Among `normal` reasons the lowest precedence wins (`retired`=11). Live: dbd/gateway/sensei/torii are 26× `sealed` + 2× `retired`, and the rail line says "retired". | `:95-112`; curl (dbd: `sealed:26, retired:2`) |
| 9.3 | bug | The disabled-switch reason says "Sign in under **Settings · Assistants** first". Sign-in lives under Settings · Dōjō and View → Identities. | `metric-status-state.svelte.ts:353` |
| 9.4 | bug | A failed persona read is reported as "No persona is signed in". `personaList.load()` failure leaves `personas=[]` and `setPersona(null)`, which is a fabricated state. | `metrics/+page.svelte:29-33`, `lib/personas.svelte.ts:132-141` |
| 9.5 | gap | The write uses the **first** persona with a slot (live: `jerrythomas`). The comment says "the screen names which one it used", but it does not. It also ignores `isDefault`. | `metric-status-state.svelte.ts:222-230` |
| 9.6 | polish | The header says switching a metric off "is recorded by the dōjō that owns it — **not here**", directly above working switches. | `metrics/+page.svelte:48` |
| 9.7 | polish | Clicking two repositories quickly can show the first one's rows under the second (no request token in `select`). | `:375-393` |
| 9.8 | info | `repo_key` is non-null for all 157, so "local only" never appears today. | curl |

**Checklist**
- [ ] Scroll the rail — expect unwatched repos to read "not watched" in neutral ink. Code: **red "Calendar days are unsettled…" × ~152** (9.1).
- [ ] Select `sensei` — expect a clean status line. Code: **"This metric has been retired"** (9.2).
- [ ] Toggle a metric while signed out — read the hint; it points to Assistants (9.3).

---

## 10 · Settings · Dōjō (Connection)

**Files.** `settings/dojo/+page.{ts,svelte}`, `dojo-status-state.svelte.ts`, `lib/personas.svelte.ts`,
`SignInPending.svelte`.

**Endpoints.** `GET /api/dojo/sync-state` (200: 26 tracked, **2 error**, 5 skipped, 19 synced; the
errors are `dojo_sync_plan` and `forge_mirror` for slot `default`: "could not reach dōjō …
127.0.0.1:5173"). `GET /api/auth/personas` (200: jerrythomas default/active, sensei-hq-org (slot
`default`) active, joelthomas8847 never connected).

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 10.1 | bug | **A sign-in failure is wiped before it can be read.** The button runs `act(p)` and then `load()`, and `load()` sets `error=null` on success. "could not start the sign-in for X" flashes and vanishes. The overlay path does the same (`routes/+layout.svelte:38-44`). | `dojo/+page.svelte:142`, `lib/personas.svelte.ts:132-147` |
| 10.2 | bug | **Any failed action hides the whole identity list.** `personaList.error` renders *instead of* the list. Make default does not reload on error, so the list stays hidden until you navigate away. | `dojo/+page.svelte:89-160`, `personas.svelte.ts:229-243` |
| 10.3 | polish | `startSignIn` drops the daemon's reason (`{ok:false,error}` → `null` → generic text). | `personas.svelte.ts:318-326` |
| 10.4 | gap | Sync rows in error ("could not reach dōjō") have no retry or sync-now action, and no pointer to what to fix. | `dojo/+page.svelte:171-205` |
| 10.5 | info | The dōjō is unreachable today (local `127.0.0.1:5173`). This is environmental, but it is what the screen will show tomorrow. | curl sync-state |

**Checklist**
- [ ] Click Connect on `joelthomas8847` with the dōjō down — expect a lasting error. Code: **error flashes then clears** (10.1).
- [ ] Confirm the two `error` rows appear under **sensei-hq-org** (slot `default` mapped to its label), not under a heading "default".

---

## 11 · Settings · Extensions

**Files.** `settings/extensions/+page.{ts,svelte}`, `extensions-state.svelte.ts`,
`crates/senseid/src/installer/install.rs:130-170`.

**Endpoints.** `GET /api/install/installed` (200, **2 items**: commands `review-slice`, `preflight`
from `~/.claude/commands`). Write: `PUT /api/install/installed/{name}/enabled`, which moves the `.md`
into `disabled/`.

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 11.1 | bug | **The copy promises "Skills, commands, agents, and hooks installed in sensei". The daemon lists only loose `*.md` under `~/.claude/skills` and `~/.claude/commands`.** Skills are directories (46 on this machine), so **no skill appears**; agents and hooks are never listed. The two rows are the user's own commands, not anything sensei installed. | `install.rs:134-170`; `ls ~/.claude/skills` = 46 dirs |
| 11.2 | question | Toggling moves a **user-authored** file into `disabled/`. Is Settings meant to manage files sensei did not install? | `config.rs:312-317` |
| 11.3 | bug (rule) | `readExtensions` fabricates `kind:'unknown'`, `enabled:true`, `name:''` for missing fields. | `settings-reads.ts:46` |
| 11.4 | polish | A native `<input type=checkbox>` where every other screen uses the rokkit `Switch`, and the error shows `statusText` instead of the daemon's reason. | `extensions/+page.svelte:73-80`, `lib/api.ts:103-115` |

**Checklist**
- [ ] Open Extensions — expect your skills. Code: **only 2 commands** (11.1).

---

## 12 · Setup wizard (welcome → assistants → roots → scan → done)

**Files.** `(config)/+layout.{ts,svelte}`, `stages.ts`, `StageNav.svelte`, `setup/*/+page.svelte`,
`lib/wizard-state.svelte.ts`, `lib/scan-state.svelte.ts`, `lib/setup/loaders.ts`.

**Endpoints.** `loadWizardData` reads config, assistant families, roots, workspaces, libs,
instruments and routers in parallel. Libs, instruments and routers no longer back any wizard stage,
yet a failure of the libs or instruments read fails the whole load (`dataOrThrow`). Scan:
`POST /api/scan/folder` per unscanned root, SSE `/api/scan/events`, poll `/api/index/status`.

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 12.1 | bug | **A failed wizard load is swallowed.** `wizardState.load()` returns `false` and the layout ignores it, setting a `loaded` flag that nothing in the template reads. The wizard renders empty (no assistants, no roots) with no error. | `(config)/+layout.svelte:35-40`, `wizard-state.svelte.ts:625-637` |
| 12.2 | bug | **Scan: every hydrated folder is shown as "indexed".** `/api/workspaces` folders carry no `status`, and the code defaults to `'indexed'`; workspace status is a hard-coded `'active'`. "PROCESSED" therefore counts all 908 folders, including 129 workspaces with no repository on disk. | `scan-state.svelte.ts:366-385`; curl folder keys |
| 12.3 | bug | **"Re-scan" re-scans nothing.** `triggerScans` only scans roots with `scanned=false`, and all are `true`. The queue is idle, so it reports "complete" about 3 s later. | `setup/scan/+page.svelte:193-199`, `scan-state.svelte.ts:449-457` |
| 12.4 | polish | Revisiting the summary shows "Files indexed 0": file counts come only from live SSE. | `scan/+page.svelte:186-187` |
| 12.5 | bug | **The Done page overstates.** Libraries come from the unfiltered `mapLibraries` with everything enabled ("209 libraries · docs & code indexed by sensei", though 206 have no docs). Instruments say "recommended for your stack" while the stack is empty. | `setup/done/+page.svelte:13-43` |
| 12.6 | question | "Nothing leaves your machine" (Welcome, Done, Health) is false once a dōjō is connected: the repository graph, sync plan and metrics are pushed. Keep it, qualify it, or drop it? | `welcome/+page.svelte:28-30`, `done/+page.svelte:87`, `health/FoundationNote.svelte:21-22` |
| 12.7 | question | Hydrate preselects every installed-but-unconfigured family (`selected = any installed`). On a forced re-run, "Configure & Continue" configures Claude Desktop even if it was deliberately left off. | `wizard-state.svelte.ts:731-737` |
| 12.8 | gap (spec) | The spec wants Continue available before the scan finishes ("enter with 5 projects, more come in incrementally"). The app gates Continue on `scanState.completed`. The mockup is a single welcome step with Browse/drag; the app has 5 stages with Assistants before Roots. | `docs/spec/screen/first-run-scan.md` Done gate; `o3-first-run-scan.png`; `wizard-state.svelte.ts:430-433` |
| 12.9 | polish | Welcome says "install the local components", but the wizard installs none (Health does). Scan says "identify folders" where it means repositories. | `welcome/+page.svelte:28`, `scan/+page.svelte:35` |
| 12.10 | latent | `commitStage` writes `setup.<stageId> = "done"` under the **same key** a commit handler uses for its payload (`setup.libraries`, `setup.instruments`, `setup.preferences`). Re-adding any of those stages would erase the saved choice right after saving it. This is what left `setup.libraries="done"` live. | `wizard-state.svelte.ts:931` |

**Checklist**
- [ ] View → Setup (`?force=1`) with the daemon stopped — expect an error. Code: **empty wizard, no message** (12.1).
- [ ] Scan stage → wait → Re-scan — expect activity. Code: **instant "complete"** (12.3).
- [ ] Done page — compare "N libraries · docs & code indexed" with reality (12.5).

---

## 13 · Health

**Files.** `(health)/health/*`, `lib/health-state.svelte.ts`, `lib/health-transport.ts`.
It runs over Tauri IPC (`health_check_and_resolve`), not HTTP. The daemon's `GET /health` reports
Homebrew, Postgres 17.11, Ollama and components all `ready`.

**Findings.**

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 13.1 | bug | **"Needs your hand" always blames Homebrew.** The sub-copy is fixed to "Homebrew isn't here yet…", but the bootstrap emits remedies for other formulas too (`overwrite_link_remedy`, `tap_missing_remedy`, `generic_brew_remedy`). `remedy.message` is **never rendered** (`Remedy.svelte` shows only the script). | `health-state.svelte.ts:133`, `crates/bootstrap/src/health/resolvers/brew_helpers.rs:94-125`, `health/Remedy.svelte:51-77` |
| 13.2 | bug | **A failed IPC call hangs "Checking the foundation." forever.** If `invoke('health_check_and_resolve')` rejects, `resolve` rejects, `init()`'s promise rejects unhandled, and the status stays `checking`. No retry button exists in that state. An unknown component id in an event also throws inside the listener. | `health-transport.ts` resolve; `health-state.svelte.ts:178-193,249-255`; `health/+page.svelte:18` |
| 13.3 | polish | The `ok` copy ends "Opening the observatory." even with `?auto=false`, where it does not open. | `health-state.svelte.ts:142` |
| 13.4 | question | "Learn more" uses `target=_blank` inside the Tauri webview. Confirm it opens the system browser. | `Remedy.svelte:57-59` |
| 13.5 | polish (mockup) | The mockup footer is "sensei 0.1.0 · macOS 14.4 · arm64 · 16gb · 412gb free"; the app shows version and platform only. | `o1-bootstrap-probing.png`, `Footer.svelte` |

**Checklist**
- [ ] View → Health — expect six gates ready and a Continue button (auto=false).
- [ ] (If you can) break a non-Homebrew formula link — expect the remedy to name it. Code: **"Homebrew isn't here yet"** (13.1).

---

## 14 · Logs

**Files.** `(health)/logs/*` — `invoke('get_log_sessions')`.

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 14.1 | bug | **"Submit to GitHub" opens `github.com/sensei-hq/app/issues/new`.** The repo is `sensei-hq/sensei`, as in `GITHUB_ORG`/`GITHUB_REPO` used by Help → Report issue. | `logs/+page.svelte:33`, `crates/bootstrap/src/config.rs:27-30` |
| 14.2 | gap | No way back. The `(health)` layout has no rail or back button, so leaving `/logs` needs the menu. | `(health)/+layout.svelte` |
| 14.3 | question | `anonymize` strips only `/Users/<name>/`. Stderr can still carry hostnames, emails, or repo and client names into a public issue. | `logs/helpers.ts:55` |
| 14.4 | polish | `copyMarkdown` has no catch, so a clipboard failure is silent. | `logs/+page.svelte:37-45` |

**Checklist**
- [ ] Logs → Report → Submit to GitHub — expect `sensei-hq/sensei`. Code: **`sensei-hq/app`** (14.1).

---

## 15 · Upgrade (post-restart)

**Files.** `(health)/upgrade/+page.svelte`. No mockup: `14-upgrades.png` and
`observatory-upgrades.md` are the dōjō *downstream* screen, a different surface.

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 15.1 | bug | **A failed upgrade reports success.** If `invoke('run_upgrade_steps')` throws, the catch sets `done=true` and leaves `anyFailed=false`, so the page reads "Up to date. Resuming. … All upgrade steps completed." with both steps still "pending". The error is discarded. | `upgrade/+page.svelte:94-98,131-148` |
| 15.2 | polish | The `listen` unlisten handle is returned from an async function and never called (leak). | `:71-101` |
| 15.3 | info | The browser path walks fake steps (`simulateBrowserUpgrade`). It runs only without Tauri. | `:103-114` |

---

## 16 · Sign-in overlay and identities

**Files.** `lib/components/SignInOverlay.svelte`, `SignInPending.svelte`, `lib/personas.svelte.ts`,
`lib/forge-auth.svelte.ts`, `routes/+layout.svelte`.

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| 16.1 | bug | **The overlay hides every identity whenever `forgeAuth.lastError` is set.** It receives `error={personaList.error ?? forgeAuth.lastError}` and renders the error *instead of* the list. After one failed auto-renewal ("could not start the authorize flow — is the dōjō reachable?"), View → Identities shows no Sign-in buttons until a later renewal succeeds. | `routes/+layout.svelte:120`, `SignInOverlay.svelte:67-68`, `forge-auth.svelte.ts:153-158` |
| 16.2 | bug | Sign-in failure is wiped by the following `load()` (same as 10.1). | `routes/+layout.svelte:38-44` |
| 16.3 | bug | The footer says "Opens a separate window per identity." Since #300 every sign-in opens the **system browser**. | `SignInOverlay.svelte:120` |
| 16.4 | polish | An `approve` row (which *does* summon the overlay) is not warning-toned: `needsUser` checks only `signIn`/`connect`. | `SignInOverlay.svelte:54` |
| 16.5 | polish | The overlay shows `githubLogin ?? label`; Settings · Dōjō shows `label`. The same identity can read differently in the two places. | `SignInOverlay.svelte:86`, `dojo/+page.svelte:114` |
| 16.6 | polish | "Copied" never resets for a later pending sign-in. | `SignInPending.svelte:16-27` |

**Checklist**
- [ ] With the dōjō down, open View → Identities — expect three identities. Code: **possibly only an error line** if a renewal already failed (16.1).
- [ ] Read the overlay footer (16.3).

---

## Top issues (ranked)

1. **Settings · Libraries and Instruments do nothing, and Libraries misinforms** (5.1-5.3, 6.2). Switches persist nowhere, the daemon never reads `setup.libraries`/`setup.instruments` (both live as `"done"`), "sensei will wrap" appears beside 206 libraries with 0 pages, and in the Settings window Libraries is likely empty because `wizardState` is never hydrated there.
2. **A failed upgrade reads as "Up to date"** (15.1): success fabricated over a thrown IPC error.
3. **General: five of seven preferences are inert** (1.1): digest, tone, nudge, telemetry and welcome are saved and never read.
4. **The Settings window doubles the app's global side effects** (0.1, 0.2): a second `forgeAuth` can open a second renewal tab, and menu events (Identities, navigation) fire in both windows.
5. **Identity failures disappear or hide the list** (10.1, 10.2, 16.1, 16.2): errors are wiped by the next `load()`, and `forgeAuth.lastError` replaces the overlay's sign-in rows.
6. **Metrics headlines are wrong both ways** (9.1, 9.2): about 152 unwatched repos in red "unsettled", healthy watched repos "retired". The sign-in hint points to the wrong screen (9.3).
7. **Inference: a chain member's removal is irreversible** (8.1; `addGatewayChainModel` has no caller), and a role move can leave a role unassigned (8.2).
8. **Assistants lists uninstalled tools as "detected"** (2.1), and a refused consent switch can stay flipped (2.3).
9. **Roots removal: no progress, and the result message unmounts with the row** (3.1, 3.2). A typo'd root path is accepted silently (3.5).
10. **Health "needs your hand" always blames Homebrew and drops `remedy.message`; an IPC failure hangs on "Checking"** (13.1, 13.2).
11. **Setup: a failed wizard load is swallowed; scan shows every folder "indexed"; Re-scan is a no-op; Done overstates libraries** (12.1-12.5).
12. **Extensions lists 2 user commands under "skills, commands, agents, hooks installed in sensei"** (11.1). Skill directories are never listed.
13. **A preference change in Settings (glyphs, name) does not reach the main window** (0.3).
14. **Logs files issues against `sensei-hq/app`** (14.1).

### Corrections to the known items
- *"Libraries shows N · sensei will wrap for libraries whose docs sensei does not hold"*: confirmed as misinformation (206/209 have `pageCount 0`), but in the **new Settings window** the list is likely empty (5.1). The toggle beneath it is also inert (5.3).
- *"Instruments has a switch that may not be rokkit's"*: **incorrect.** It is the rokkit `Switch` via `$lib/components/Switch.svelte`. The non-rokkit control is the native checkbox on Extensions (11.4). The real Instruments problem is that the switch installs nothing (6.2).
- *"Roots removal shows no progress"*: confirmed, and worse: even the final "Removed N …" message is never visible (3.1).
- *"Metrics shows 'Calendar days are unsettled' for 154 unwatched repositories"*: confirmed at **152** today (157 repos, 4 watched). The summary has no watched fact, so the fix needs the daemon. Also new: watched repos headline "retired" (9.2).
