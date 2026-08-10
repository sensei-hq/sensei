<script lang="ts">
  import { PRINCIPLES } from '$lib/hub-data.js';
</script>

<section id="approach" class="bg-paper-soft">
  <div class="approach-inner mx-auto px-7 py-24">
    <div class="approach-grid gap-8">
      <div>
        <span class="mono text-ink-mute mb-3 block text-xs" style="letter-spacing:0.12em;text-transform:uppercase">The approach</span>
        <h2 class="display text-ink m-0 text-2xl" style="font-weight:400;letter-spacing:-0.015em;line-height:1.25">
          The master observes for a long time before teaching.
        </h2>
        <p class="text-ink-soft mt-4 text-sm" style="line-height:1.7;max-width:360px">
          The kanji on each tool name a phase of practice — observe, build,
          hasten, guard. They are what we ask of the people who use our
          tools, and what we ask of ourselves while making them.
        </p>
      </div>

      <div class="flex flex-col gap-0">
        {#each PRINCIPLES as pr, i (pr.title)}
          <div class="principle-row gap-5 py-5" class:border-t={i > 0} class:border-paper-edge={i > 0}>
            <div style="width:56px">
              <span class="kanji text-accent text-3xl" style="line-height:1">{pr.kanji}</span>
            </div>
            <div>
              <div class="flex items-baseline gap-3 mb-1">
                <span class="display text-ink text-lg">{pr.title}</span>
                <span class="mono text-ink-faint text-xs">{pr.label}</span>
              </div>
              <p class="text-ink-soft m-0 text-sm" style="line-height:1.65">{pr.text}</p>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </div>
</section>

<style>
  .approach-inner {
    max-width: 1120px;
  }
  .approach-grid {
    display: grid;
    grid-template-columns: 1fr 1.5fr;
    align-items: start;
  }
  .principle-row {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: start;
  }
  @media (max-width: 768px) {
    .approach-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
