<script lang="ts">
  // The identities, status only (user, 2026-11-03): selecting one opens it; every
  // action is in the identity's header, so there is one place to act.
  import { Avatar, Card } from '@rokkit/ui';
  import type { IdentityCard } from './dojo-connection-state.svelte.js';

  let { cards, onselect }: { cards: IdentityCard[]; onselect: (label: string) => void } = $props();
</script>

<ul class="m-0 p-0 list-none flex flex-col gap-2.5" data-identities>
  {#each cards as c (c.label)}
    <li>
      <Card class={c.selected ? 'border-ink' : undefined}>
        <button
          type="button"
          data-identity={c.label}
          aria-pressed={c.selected}
          class="w-full flex items-center gap-3 text-left bg-transparent border-0 p-0 cursor-pointer text-ink"
          onclick={() => onselect(c.label)}
        >
          <Avatar name={c.name} size="sm" />
          <span class="flex-1 min-w-0 flex flex-col gap-0.5">
            <span class="flex items-center gap-1.5 flex-wrap">
              <span class="text-sm font-medium" data-identity-name>{c.name}</span>
              {#if c.isDefault}
                <span class="text-xs text-ink-mute border border-paper-edge rounded-full px-1.5" data-default-persona>default</span>
              {/if}
            </span>
            <span class="text-xs {c.toneClass}" data-standing>{c.standing}</span>
            {#if c.summary}<span class="text-xs text-ink-mute" data-summary>{c.summary}</span>{/if}
          </span>
        </button>
      </Card>
    </li>
  {/each}
</ul>
