<script lang="ts">
    import Kanji from '$lib/components/Kanji.svelte';
  import { wizardState } from '$lib/wizard-state.svelte.js';

  // Every line of the summary is the wizard state's (audit 2026-10-28, item 15).
  const summary = $derived(wizardState.doneSummary);
</script>

<div class="max-w-[720px]">
  <h1
    class="display text-4xl font-light leading-tight m-0 mb-8 tracking-tight"
  >
    The observatory<br />
    <span class="text-accent">is ready.</span>
  </h1>

  <p class="text-base text-ink-soft leading-loose max-w-[560px] m-0 mb-10">
    Sensei is watching. Work normally with your AI assistants — sensei
    learns from every session and begins teaching after it has enough data.
  </p>

  <!-- What was configured -->
  <div class="text-xs uppercase tracking-wider text-ink-mute mb-3">
    What you set up
  </div>
  <div
    data-testid="done-summary"
    class="grid grid-cols-2 gap-3 mb-10"
  >
    {#each summary as s}
      <div class="flex items-start gap-3 bg-paper-mute border border-paper-edge rounded-md p-4">
        <Kanji char={s.kanji} size="inherit" tone="inherit" class="text-2xl text-accent leading-none mt-1" />
        <div class="min-w-0">
          <div class="text-sm text-ink font-medium">{s.label}</div>
          <div class="text-xs text-ink-mute mt-0.5">{s.sub}</div>
        </div>
      </div>
    {/each}
  </div>

  <!-- What happens next -->
  <div class="text-xs uppercase tracking-wider text-ink-mute mb-3">
    What happens next
  </div>
  <div class="grid grid-cols-3 gap-4 py-5 border-t border-b border-paper-edge">
    {#each [
      { kanji: '観', label: 'Observing', sub: 'sessions tracked in real time' },
      { kanji: '師', label: 'Teaching', sub: 'after ~3 sessions per workspace' },
      { kanji: '静', label: 'Local', sub: 'nothing leaves your machine' },
    ] as s}
      <div>
        <Kanji char={s.kanji} size="inherit" tone="inherit" class="block text-2xl text-accent mb-2" />
        <div class="display text-base">{s.label}</div>
        <div class="text-xs text-ink-mute mt-1">{s.sub}</div>
      </div>
    {/each}
  </div>
</div>
