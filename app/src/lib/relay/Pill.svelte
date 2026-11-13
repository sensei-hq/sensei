<script lang="ts">
  // A tone-tinted chip (COMPONENTS.md §3a): 11px, 1/8 padding, full radius,
  // nowrap. It draws only its label and tone; the tone vocabulary lives in
  // primitives.ts so the class contract is pinned on data. Its hook is its own:
  // rokkit's theme styles `[data-pill]` above these utilities.
  import { pillClass, type PillTone } from './primitives.js';

  let { label, tone = 'neutral' }: { label: string; tone?: PillTone } = $props();
</script>

<span data-relay-pill data-tone={tone} class={pillClass(tone)}>{label}</span>
