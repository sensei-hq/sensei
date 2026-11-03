<script lang="ts">
  // Every lens as a list, for a narrow screen (M17, under 640px): the rows
  // `listView` gives, each at least 44px tall, with its role bar and badges.
  // A module opens on a tap; a fold or a group pair is read, not opened.
  import LensFailed from './LensFailed.svelte';
  import type { ListView } from './list';
  import { ROLE_BG } from './view';
  import type { Role } from './types';

  let {
    view,
    onfocus,
    failed = null,
    pending = false,
    onretry = () => {},
  }: {
    view: ListView;
    onfocus: (id: string) => void;
    /** The lens's own read failed (review r3 N4): why, with Retry — never an empty list. */
    failed?: string | null;
    /** The lens's own data is still being read. */
    pending?: boolean;
    onretry?: () => void;
  } = $props();

  const BADGE = { danger: 'text-danger', warning: 'text-warning', faint: 'text-ink-faint' };
</script>

<div class="flex flex-col gap-3" data-testid="codemap-list">
  {#if failed}
    <LensFailed reason={failed} {onretry} />
  {:else if pending}
    <p class="m-0 text-sm text-ink-faint">Reading this lens…</p>
  {:else}
    {#each view.groups as g (g.label)}
      <section>
        <h4 class="m-0 mb-1 text-xs uppercase tracking-wide text-ink-mute">{g.label}</h4>
        {#each g.rows as r (r.id)}
          {@const total = r.roles ? Object.values(r.roles).reduce((a, v) => a + v, 0) : 0}
          <svelte:element this={r.open ? 'button' : 'div'}
            class="w-full min-h-11 flex flex-col justify-center gap-1 px-3 py-1 bg-paper-soft border border-paper-edge rounded mb-1 text-left"
            class:cursor-pointer={r.open}
            role={r.open ? undefined : 'listitem'}
            onclick={r.open ? () => onfocus(r.id) : undefined}
            data-testid={`list-row-${r.id}`}>
            <span class="flex items-center justify-between gap-2">
              <span class="font-mono text-xs text-ink truncate">{r.label}</span>
              <span class="text-xs text-ink-faint shrink-0">{r.sub}</span>
            </span>
            {#if total || r.badges.length}
              <span class="flex items-center gap-2">
                {#if total}
                  <span class="flex h-1 w-24 rounded overflow-hidden" data-testid="role-bar">
                    {#each Object.entries(r.roles ?? {}).filter(([, v]) => v > 0) as [role, v] (role)}
                      <span class={ROLE_BG[role as Role]} style:width={`${(v / total) * 100}%`} title={`${v} ${role}`}></span>
                    {/each}
                  </span>
                {/if}
                {#each r.badges as b (b.text)}<span class={`text-xs ${BADGE[b.tone]}`}>{b.text}</span>{/each}
              </span>
            {/if}
          </svelte:element>
        {/each}
      </section>
    {/each}
  {/if}
</div>
