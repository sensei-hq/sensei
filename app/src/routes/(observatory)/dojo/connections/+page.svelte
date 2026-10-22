<script lang="ts">
    import { railIcon } from "../../observatory-nav";
  import { invalidateAll } from '$app/navigation';
  import { PageHeader, Screen, EmptyState, Kanji, ScreenState } from '$lib/components';
  import type { DojoMembership } from '$lib/types.js';
  import {
    boundWorkspacesSummary,
    connectionsHeadline,
    kindPill,
    syncChip,
  } from './dojo-connections-state.svelte.js';

  // Signing in is joining (#236 step 4e): every dōjō the signed-in persona
  // belongs to arrives here with the next sync — there is nothing to pair.
  let { data } = $props();
</script>

{#snippet chip(label: string, bg: string, text: string)}
  <span class="font-mono text-xs px-2 py-0.5 rounded-full {bg} {text}">{label}</span>
{/snippet}

{#snippet connection(c: DojoMembership)}
  {@const pill = kindPill(c.kind)}
  {@const sync = syncChip(c.sync_status)}
  <article
    data-connection={c.id}
    class="mb-3 border border-paper-edge rounded-lg bg-paper-soft p-4"
  >
    <div class="flex items-center gap-3">
      <Kanji char={pill.kanji} size="lg" tone="accent" />
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <span class="text-sm text-ink truncate">{c.tenant_key}</span>
          {@render chip(pill.label, pill.bg, pill.text)}
        </div>
        <div class="font-mono text-xs text-ink-mute mt-0.5 truncate">{c.dojo_url}</div>
      </div>
      {@render chip(sync.label, sync.bg, sync.text)}
    </div>

    <div
      class="flex flex-wrap items-center gap-x-4 gap-y-1 mt-3 pt-3 border-t border-paper-edge text-xs text-ink-mute"
    >
      <span>role · {c.role}</span>
      <span>attribution · {c.attribution_default}</span>
      <span data-bound-count>{boundWorkspacesSummary(c.bound_workspaces)}</span>
      {#if c.org_slugs.length > 0}
        <span class="flex flex-wrap items-center gap-1.5" data-testid="membership-orgs">
          orgs ·
          {#each c.org_slugs as slug (slug)}
            {@render chip(slug, 'bg-paper-mute', 'text-ink-mute')}
          {/each}
        </span>
      {/if}
    </div>

    {#if c.bound_workspaces.length > 0}
      <div class="flex flex-wrap gap-1.5 mt-2">
        {#each c.bound_workspaces as p (p.id)}
          {@render chip(p.name, 'bg-paper-mute', 'text-ink-mute')}
        {/each}
      </div>
    {/if}
  </article>
{/snippet}

<Screen>
    {#snippet header()}
    <PageHeader
      eyebrow="Observatory · Dōjō connections"
      kanji="連" icon={railIcon("/dojo/connections")}
      title="Where your sensei meets the team's Dōjō."
      description="Every company, client, or community Dōjō you belong to joins here when you sign in — share what you learn and receive what the team already knows. You can belong to several at once."
    >
      {#snippet right()}
        <a
          href="/settings/dojo"
          data-sign-in
          class="px-3 py-2 rounded-md border border-paper-edge bg-transparent text-sm text-ink-soft no-underline"
        >Sign in to a Dōjō</a>
      {/snippet}
    </PageHeader>
    {/snippet}

    <div class="max-w-[1060px] mx-auto px-7 pb-10" data-connections-total={data.connections.length}>
      {#if data.error}
        <ScreenState status="error" error={data.error} onretry={invalidateAll} />
      {:else if data.connections.length === 0}
        <EmptyState
          kanji="連"
          title="no dōjō yet"
          description="Sign in under Settings → Dōjō: each Dōjō you belong to joins here on the next sync. Employer, client, and community Dōjōs each keep their own scopes."
        />
      {:else}
        <p class="text-sm text-ink-soft mb-6" data-connections-summary>
          {connectionsHeadline(data.connections)}
        </p>
        {#each data.connections as c (c.id)}
          {@render connection(c)}
        {/each}
      {/if}
    </div>
</Screen>
