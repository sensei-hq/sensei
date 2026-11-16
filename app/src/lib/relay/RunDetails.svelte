<script lang="ts">
  // Run details (COMPONENTS §3d RunDetails; CONTRACT §1e.3): folded to its summary
  // line; open, every row — a value nobody reported reads so, never 0 (D83) — and how
  // many of the task's turns reported. Radius 12 (#388).
  import type { RelayRunDetailsView } from '@sensei/state';

  let { run }: { run: RelayRunDetailsView } = $props();
</script>

<details data-run-details class="rounded-xl border border-solid border-paper-edge bg-paper max-w-[880px]">
  <summary class="cursor-pointer py-3 px-4 text-sm text-ink">
    Run details <span class="text-ink-soft">· {run.line}</span>
  </summary>
  <dl class="m-0 grid grid-cols-2 md:grid-cols-4 gap-x-6 gap-y-2 px-4 pb-3">
    {#each run.rows as row (row.k)}
      <div data-run-detail={row.k} class="flex flex-col">
        <dt class="text-xs text-ink-mute">{row.k}</dt>
        <dd class="m-0 text-sm text-ink">
          {#if row.v === null}<span data-not-reported class="italic text-ink-mute">not reported</span>{:else}{row.v}{/if}
        </dd>
      </div>
    {/each}
  </dl>
  <p data-run-note class="m-0 px-4 pb-3 text-xs text-ink-mute">{run.note}</p>
</details>
