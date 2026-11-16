<script lang="ts">
  // Past runs under the Crew view (CONTRACT §1i; #424, the UI half of #393): one row per
  // ended run — id and name, span, tasks done, members, and when it is deleted (three days
  // or fewer in the warning tone). A display-only rokkit List card; nothing when none ended.
  import { List, Pill } from '@rokkit/ui';
  import type { ProxyItem } from '@rokkit/states';
  import type { RelayPastRunView } from '@sensei/state';
  import SectionHeading from './SectionHeading.svelte';
  import { NOW_ROW } from './primitives.js';

  let { rows }: { rows: RelayPastRunView[] } = $props();
</script>

{#if rows.length}
  <section aria-label="Past runs" data-relay-section="past-runs" class="flex flex-col gap-1.5">
    <SectionHeading label="Past runs" count={rows.length} />
    <List items={rows} fields={{ value: 'id' }} selectable={false} variant="card" label="Past runs">
      {#snippet itemContent(proxy: ProxyItem)}
        {@const run = proxy.original as RelayPastRunView}
        <span data-past-run={run.id} class={NOW_ROW}>
          <span class="flex flex-col min-w-0 flex-[1_1_220px]">
            <span class="flex items-center gap-2">
              <strong data-part="title" class="text-sm font-semibold">{run.title}</strong>
              <Pill value={run.state.label} tone={run.state.tone} size="sm" />
            </span>
            <span data-part="name" class="text-xs text-ink-mute truncate">{run.name}</span>
          </span>
          <span data-part="span" class="text-xs text-ink-soft tabular-nums">{run.span}</span>
          <span data-part="done" class="text-xs text-ink-soft">{run.done}</span>
          <span data-part="members" class="text-xs text-ink-soft">{run.members}</span>
          <span
            data-part="deletes"
            data-warning={run.deletes.warning}
            class={['ml-auto text-xs', run.deletes.warning ? 'text-warning font-semibold' : 'text-ink-mute']}
          >{run.deletes.text}</span>
        </span>
      {/snippet}
    </List>
  </section>
{/if}
