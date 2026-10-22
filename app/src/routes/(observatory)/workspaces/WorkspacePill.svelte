<script lang="ts">
  // Small mono pill — the client label and the dormant/archived status tag.
  // `tone="dormant"` quiets the text one ramp step (ink-mute) so the status
  // tag reads as secondary to the client pill beside it.
  let { text, tone }: { text: string; tone?: 'dormant' } = $props();
</script>

<span
  data-component="proj-pill"
  data-tone={tone}
  class="font-mono text-xs uppercase tracking-[0.08em] bg-paper-mute border border-paper-edge rounded-full py-0.5 px-2 whitespace-nowrap shrink-0"
  class:text-ink-mute={tone === 'dormant'}
  class:text-ink-soft={tone !== 'dormant'}
>{text}</span>
