<script lang="ts">
  import { untrack } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { PageHeader, Kanji, ScreenState } from '$lib/components';
  import {
    CollectiveSharing,
    ATTRIBUTION_OPTIONS,
    CADENCE_OPTIONS,
    CATEGORY_META,
    DESTINATION_TOGGLES,
  } from './collective-sharing-state.svelte.js';

  let { data } = $props();

  // All screen state (the full wire object + read-modify-write saves) lives in
  // the controller; the api client is injected so this stays a pure template.
  // Seed once from the loaded preferences — the controller owns every save +
  // server-object adoption thereafter. untrack makes the one-time read explicit.
  // On a LOAD failure `data.preferences` is null (F8) — the controller isn't
  // built and the screen shows error-with-Retry instead of fabricated defaults.
  const prefs = untrack(() => data.preferences);
  const state = prefs ? new CollectiveSharing(prefs, senseiApi(appState.port)) : null;
</script>

<!-- A small switch rendered as an accessible button — geometry-only inline, all
     colour via named tokens. -->
{#snippet switchBtn(on: boolean, onToggle: () => void, label: string, testid: string)}
  <button
    type="button"
    role="switch"
    aria-checked={on}
    aria-label={label}
    data-toggle={testid}
    disabled={state?.saving}
    onclick={onToggle}
    class="relative inline-flex shrink-0 items-center rounded-full cursor-pointer transition-colors disabled:opacity-50 disabled:cursor-not-allowed {on
      ? 'bg-primary'
      : 'bg-paper-mute'}"
    style="width: 40px; height: 22px;"
  >
    <span
      class="absolute rounded-full bg-paper transition-transform"
      style="width: 16px; height: 16px; top: 3px; left: 3px; transform: translateX({on ? 18 : 0}px);"
    ></span>
  </button>
{/snippet}

{#snippet sectionHead(title: string, sub: string)}
  <div class="mb-3">
    <h2 class="display text-base font-medium m-0 text-ink">{title}</h2>
    {#if sub}
      <p class="text-xs text-ink-soft mt-1 mb-0 max-w-[640px] leading-normal">{sub}</p>
    {/if}
  </div>
{/snippet}

<PageHeader
  eyebrow="Observatory · Collective"
  kanji="群"
  title="What your sensei shares, and toward whom."
  description="Choose where anonymized insight goes, how often batches are prepared, and which categories may leave. Only the generalized rewrite of a learning is shared — never the original text of a memory. File paths, project, client and person names are stripped from everything that goes, and anything that still looks identifying is held back instead of sent. Review each batch before it leaves."
/>

{#if data.error || !state}
  <div class="max-w-[1060px] mx-auto px-7 pb-10">
    <ScreenState status="error" error={data.error ?? 'Failed to load sharing preferences'} onretry={invalidateAll} />
  </div>
{:else}
<div class="max-w-[1060px] mx-auto px-7 pb-10" data-collective-destination={state.destination}>
  <!-- meta: last-changed + live save / error feedback -->
  <div class="flex items-center gap-3 mt-5 mb-7 text-xs">
    <span class="text-ink-mute" data-updated>{state.updatedLabel}</span>
    {#if state.saving}
      <span class="text-ink-soft" data-saving>saving…</span>
    {/if}
    {#if state.error}
      <span class="text-danger" data-error>{state.error}</span>
    {/if}
  </div>

  <!-- Destination — two coexisting toggles mapped to the destination enum -->
  <section class="mb-8" data-sharing={state.sharing}>
    {@render sectionHead(
      'Destination',
      'Where anonymized insight goes. Both can run at once, or neither — personal sensei works fine with both off.',
    )}
    <div class="border border-paper-edge rounded-lg bg-paper-soft divide-y divide-paper-edge">
      {#each DESTINATION_TOGGLES as d (d.key)}
        {@const on = d.key === 'global' ? state.globalOn : state.dojoOn}
        <div class="flex items-center gap-3 py-3 px-4" data-destination-row={d.key}>
          <Kanji char={d.kanji} size="lg" tone={on ? 'accent' : 'muted'} />
          <div class="flex-1 min-w-0">
            <div class="text-sm text-ink">{d.label}</div>
            <div class="text-xs text-ink-soft mt-0.5 leading-normal">{d.blurb}</div>
          </div>
          {@render switchBtn(
            on,
            () => (d.key === 'global' ? state.setGlobal(!on) : state.setDojo(!on)),
            `${d.label} sharing`,
            `dest-${d.key}`,
          )}
        </div>
      {/each}
    </div>
    {#if !state.sharing}
      <p class="text-xs text-ink-mute mt-2 mb-0" data-nothing-leaves>
        Nothing leaves the machine while both destinations are off.
      </p>
    {/if}
  </section>

  <!-- Cadence — single-select chip strip -->
  <section class="mb-8">
    {@render sectionHead('Cadence', 'How often batches are prepared. Manual pauses batching until you ask for one.')}
    <div class="flex items-center gap-2" data-cadence={state.cadence}>
      {#each CADENCE_OPTIONS as c (c.value)}
        {@const active = state.cadence === c.value}
        <button
          type="button"
          data-cadence-option={c.value}
          aria-pressed={active}
          disabled={state.saving}
          onclick={() => state.setCadence(c.value)}
          class="px-3 py-1.5 rounded-full text-xs cursor-pointer border transition-colors disabled:opacity-50 disabled:cursor-not-allowed {active
            ? 'bg-accent-soft text-accent border-accent-soft'
            : 'bg-paper-mute text-ink-soft border-paper-edge'}"
        >{c.label}</button>
      {/each}
    </div>
  </section>

  <!-- Categories — one boolean toggle per category, single column -->
  <section class="mb-8">
    {@render sectionHead(
      'What gets shared',
      'Each category is one kind of insight. Turn off any you would rather keep to yourself.',
    )}
    <div class="flex flex-col gap-1" data-categories>
      {#each CATEGORY_META as cat (cat.key)}
        {@const on = state.isCategoryOn(cat.key)}
        <div
          class="flex items-center gap-3 py-3 px-4 rounded-md border border-paper-edge bg-paper-soft"
          data-category={cat.key}
          data-on={on}
        >
          <Kanji char={cat.kanji} size="base" tone={on ? 'accent' : 'muted'} />
          <div class="flex-1 min-w-0">
            <div class="text-sm text-ink">{cat.label}</div>
            <div class="text-xs text-ink-soft mt-0.5 leading-normal">{cat.blurb}</div>
          </div>
          {@render switchBtn(on, () => state.toggleCategory(cat.key), `share ${cat.label}`, `cat-${cat.key}`)}
        </div>
      {/each}
    </div>
  </section>

  <!-- Attribution default — radio-like selector -->
  <section class="mb-4">
    {@render sectionHead('Default attribution', 'How shared insight is credited by default.')}
    <div class="grid gap-2" data-attribution={state.attribution} role="radiogroup" aria-label="Default attribution">
      {#each ATTRIBUTION_OPTIONS as a (a.value)}
        {@const active = state.attribution === a.value}
        <button
          type="button"
          role="radio"
          aria-checked={active}
          data-attribution-option={a.value}
          disabled={state.saving}
          onclick={() => state.setAttribution(a.value)}
          class="text-left py-3 px-4 rounded-md border cursor-pointer transition-colors disabled:opacity-50 disabled:cursor-not-allowed {active
            ? 'bg-paper border-accent'
            : 'bg-paper-soft border-paper-edge'}"
        >
          <div class="flex items-center gap-2">
            <span
              class="rounded-full {active ? 'bg-accent' : 'border border-ink-faint'}"
              style="width: 12px; height: 12px;"
            ></span>
            <span class="text-sm {active ? 'text-ink' : 'text-ink-soft'}">{a.label}</span>
          </div>
          <p class="text-xs text-ink-soft mt-1 mb-0 leading-normal">{a.blurb}</p>
        </button>
      {/each}
    </div>
  </section>
</div>
{/if}
