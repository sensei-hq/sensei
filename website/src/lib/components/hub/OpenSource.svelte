<script lang="ts">
  import { REPOS } from '$lib/hub-data.js';
  import { Button } from '@rokkit/ui';
</script>

<section id="open" class="opensource-wrap mx-auto px-7 py-24">
  <div class="opensource-grid gap-8">
    <div>
      <div class="flex items-baseline gap-3 mb-4">
        <span class="kanji text-accent text-3xl" style="line-height:1">公</span>
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">Kō · in the open</span>
      </div>
      <h2 class="display text-ink m-0 text-2xl" style="font-weight:400;letter-spacing:-0.015em;line-height:1.3">
        All four libraries are built in the open.
      </h2>
      <p class="text-ink-soft mt-4 text-sm" style="line-height:1.7;max-width:420px">
        Gateway, DBD, Rokkit and Kavach are MIT-licensed and developed in public.
        Read the source, file an issue, or send a pull request — the workshop door is open.
      </p>
      <Button href="https://github.com/sensei-hq" target="_blank" rel="noopener"
              style="outline" size="md" class="mt-5">
        <span class="kanji text-sm" style="line-height:1">叉</span>
        Browse the repositories
      </Button>
    </div>

    <div class="border border-paper-edge rounded-lg bg-paper" style="overflow:hidden">
      <div class="divide-y divide-paper-edge">
        {#each REPOS as r (r.name)}
          <a href={r.href} target="_blank" rel="noopener"
             class="repo-row gap-4 px-5 py-4 no-underline">
            <span class="repo-dot {r.accentClass}"></span>
            <span>
              <span class="mono text-ink block text-sm">{r.name}</span>
              <span class="text-ink-mute text-xs">{r.note}</span>
            </span>
            <span class="mono text-ink-mute text-xs">{r.lang}</span>
          </a>
        {/each}
      </div>
    </div>
  </div>
</section>

<style>
  .opensource-wrap {
    max-width: 1120px;
  }
  .opensource-grid {
    display: grid;
    grid-template-columns: 1fr 1.2fr;
    align-items: center;
  }
  .repo-row {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    transition: background 0.15s;
  }
  .repo-row:hover {
    background: var(--paper-soft);
  }
  .repo-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  @media (max-width: 768px) {
    .opensource-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
