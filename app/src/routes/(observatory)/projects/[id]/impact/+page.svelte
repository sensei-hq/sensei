<script lang="ts">
  import { page } from '$app/state';
</script>

<div class="max-w-[820px] mx-auto px-12 py-10 pb-16">
  <h1 class="display text-xl font-normal m-0 mb-2">Impact</h1>
  <p class="text-sm text-ink-mute m-0 mb-6">
    Outcome verdicts for major changes shipped from this project.
  </p>
  <div class="flex flex-col items-center gap-2 px-5 py-10 bg-paper-mute border border-paper-mute rounded-lg text-center">
    <span class="kanji text-3xl text-accent opacity-40">果</span>
    <p class="text-sm text-ink-soft leading-normal max-w-[420px] m-0">
      Impact verdicts are greenfield — the outcome-logging pass lands with
      T3 Slice 3. Until then, this pane stays empty by design.
    </p>
    <p class="text-xs text-ink-soft m-0 mt-2 font-mono">project: {page.params.id}</p>
  </div>
</div>
