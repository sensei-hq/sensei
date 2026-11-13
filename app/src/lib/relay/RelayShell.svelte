<script lang="ts">
  // The Relay shell (COMPONENTS §2): ShellScreen with what `toRelayShell` made
  // of the page's state — mark, title, context, the five toolbar buttons, the
  // info band, the stats band, the footer — and the body the screen hands it.
  // It draws what it is given and nothing else; the state and the rules live in
  // `relay-shell.ts`.
  import type { Snippet } from 'svelte';
  import type { FilterApi, FilterState } from '@rokkit/app';
  import { ShellScreen } from '@sensei/ui';
  import { KANJI_ICONS } from '$lib/glyph-vocabulary';
  import type { RelayFooterTone, RelayShell } from './relay-shell.js';
  // The header's icon buttons: a scoped override of the theme's toolbar look
  // (user ruling 2026-11-09), one partial, to be upstreamed.
  import './relay-toolbar.css';

  let {
    shell,
    viewId,
    glyphs,
    compose,
    filters,
    onfilter,
    children,
    ...rest
  }: {
    shell: RelayShell;
    viewId: string;
    glyphs?: 'kanji' | 'icon';
    /** The compose band (Message the crew), drawn first in the body while open. */
    compose?: Snippet;
    /** The main window's repository/workspace filter band. */
    filters?: Snippet<[FilterApi]>;
    /** A stat pressed or cleared: the shell's filter state, for the page to keep. */
    onfilter?: (next: FilterState) => void;
    children: Snippet;
    [key: `data-${string}`]: string;
  } = $props();

  // Literal, so UnoCSS finds each colour utility.
  const DOT: Record<RelayFooterTone, string> = {
    accent: 'bg-accent',
    success: 'bg-success',
    warning: 'bg-warning',
    danger: 'bg-danger',
    muted: 'bg-ink-faint',
  };
</script>

<ShellScreen
  {viewId}
  {glyphs}
  mark={{ kanji: '継', icon: KANJI_ICONS['継'] }}
  title="Relay"
  context={shell.context}
  search={false}
  toolbar={shell.toolbar}
  info={shell.info}
  pinned={shell.pinned}
  stats={shell.stats}
  filter={shell.filter}
  {filters}
  {onfilter}
  data-screen="relay"
  data-relay-view={shell.view}
  {...rest}
>
  {#snippet footer()}
    <ul class="m-0 flex flex-wrap items-center gap-x-4 gap-y-1.5 p-0 list-none text-xs text-ink-soft" data-relay-footer>
      {#each shell.footer as item (item.text)}
        <li class="inline-flex items-center gap-1.5">
          <span class="inline-block h-2 w-2 shrink-0 rounded-full {DOT[item.tone]}" aria-hidden="true"></span>
          {item.text}
        </li>
      {/each}
    </ul>
  {/snippet}

  {#if shell.compose && compose}
    {@render compose()}
  {/if}
  {@render children()}
</ShellScreen>
