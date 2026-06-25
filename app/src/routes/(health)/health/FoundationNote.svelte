<script lang="ts">
  // The "what this is" framing card — shown in the left column while the gate
  // ledger is still checking, so the otherwise-empty space explains what sensei
  // is verifying and reassures that anything missing is installed locally.
  // Purely informational; no props.
  import { Eyebrow, Kanji } from '$lib/components';
</script>

<div
  data-component="foundation-note"
  class="mt-4 max-w-[300px] rounded-lg border border-paper-edge bg-paper-soft p-4"
>
  <div class="mb-1.5"><Eyebrow>What this is</Eyebrow></div>
  <p class="text-xs text-ink-soft leading-relaxed">
    sensei runs on a small local foundation — a database, a model runtime, and a
    background watcher. This check confirms each piece is present before opening.
  </p>
  <div class="mt-3 pt-3 border-t border-paper-edge flex gap-2 items-start">
    <Kanji char="修" size="base" tone="accent" />
    <p class="text-xs text-ink-mute leading-relaxed">
      Anything missing is installed for you — usually automatically, occasionally
      with one script to paste. Nothing leaves your machine.
    </p>
  </div>
</div>
