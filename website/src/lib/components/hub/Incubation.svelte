<script lang="ts">
  import { INCUBATING } from '$lib/hub-data.js';
</script>

<section id="incubation" class="incubation-wrap mx-auto px-7 py-24">
  <div class="flex items-end justify-between flex-wrap gap-4 mb-7">
    <div>
      <div class="flex items-baseline gap-3 mb-3">
        <span class="kanji text-accent text-3xl" style="line-height:1">育</span>
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">Iku · to nurture — in incubation</span>
      </div>
      <h2 class="display text-ink m-0 text-3xl" style="font-weight:300;letter-spacing:-0.02em">
        Still taking shape.
      </h2>
    </div>
    <p class="text-ink-soft text-sm" style="line-height:1.6;max-width:380px">
      Experiments we are nurturing in the workshop. Rougher edges, same
      temperament — these may grow into the next tools we ship.
    </p>
  </div>

  <div class="incubation-grid gap-5">
    {#each INCUBATING as p (p.id)}
      <div class="incubation-card border border-paper-edge border-dashed rounded-lg text-{p.id}">
        <div class="p-6 flex flex-col" style="flex:1">
          <div class="flex items-center justify-between">
            <span class="kanji text-{p.id} text-3xl" style="line-height:1">{p.kanji}</span>
            <span class="flex items-center gap-2 border border-paper-edge rounded-sm px-2 py-1 text-ink-mute mono text-xs" style="white-space:nowrap">
              <span class="incubation-dot" style="background:var(--{p.id})"></span>
              Incubating
            </span>
          </div>
          <div class="flex items-baseline gap-2 mt-5 mb-1">
            <span class="display text-ink text-xl" style="font-weight:400;letter-spacing:-0.02em">{p.name}</span>
            <span class="mono text-ink-faint text-xs" style="white-space:nowrap">{p.label}</span>
          </div>
          <span class="mono text-ink-mute mb-3 text-xs" style="letter-spacing:0.1em;text-transform:uppercase">{p.category}</span>
          <p class="display text-ink m-0 text-base" style="font-weight:400;line-height:1.3">{p.tagline}</p>
          <p class="text-ink-soft mt-2 text-sm" style="line-height:1.6">{p.blurb}</p>
        </div>
      </div>
    {/each}
  </div>
</section>

<style>
  .incubation-wrap {
    max-width: 1120px;
  }
  .incubation-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
  }
  .incubation-card {
    display: flex;
    flex-direction: column;
    flex: 1;
    background: transparent;
    transition: border-color 0.18s, background 0.18s;
  }
  .incubation-card:hover {
    background: var(--paper-soft);
  }
  .incubation-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  @media (max-width: 900px) {
    .incubation-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
