<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { PageHeader, EmptyState, Kanji } from '$lib/components';
  import { Button } from '@rokkit/ui';
  import type { DojoMembership } from '$lib/types.js';
  import {
    ConnectForm,
    KIND_OPTIONS,
    boundProjectsSummary,
    connectionsHeadline,
    kindPill,
    syncChip,
  } from './dojo-connections-state.svelte.js';

  let { data } = $props();

  // Connect-form field/validation/submit state lives in the state class; the
  // component only drives it and re-loads the list on success.
  const form = new ConnectForm();

  async function connect() {
    const ok = await form.submit(senseiApi(appState.port));
    if (ok) await invalidateAll();
  }

  // A static utility-class constant (a rendering rule, not a data derivation).
  const inputClass =
    'w-full px-3 py-2 border border-paper-edge rounded-md bg-paper-mute text-ink text-sm outline-none focus:border-ink-soft';
</script>

{#snippet chip(label: string, bg: string, text: string)}
  <span class="font-mono text-xs px-2 py-0.5 rounded-full {bg} {text}">{label}</span>
{/snippet}

{#snippet field(label: string)}
  <span class="text-xs text-ink-mute">{label}</span>
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
      <span data-bound-count>{boundProjectsSummary(c.bound_projects)}</span>
      {#if c.org_slugs.length > 0}
        <span class="flex flex-wrap items-center gap-1.5" data-testid="membership-orgs">
          orgs ·
          {#each c.org_slugs as slug (slug)}
            {@render chip(slug, 'bg-paper-mute', 'text-ink-mute')}
          {/each}
        </span>
      {/if}
    </div>

    {#if c.bound_projects.length > 0}
      <div class="flex flex-wrap gap-1.5 mt-2">
        {#each c.bound_projects as p (p.id)}
          {@render chip(p.name, 'bg-paper-mute', 'text-ink-mute')}
        {/each}
      </div>
    {/if}
  </article>
{/snippet}

<PageHeader
  variant="h2"
  eyebrow="Observatory · Dōjō connections"
  kanji="連"
  title="Where your sensei meets the team's Dōjō."
  description="Pair with a company, client, or community Dōjō to share what you learn and receive what the team already knows. You can belong to several at once."
>
  {#snippet right()}
    <button
      type="button"
      data-connect-toggle
      onclick={() => (form.open = !form.open)}
      class="px-3 py-2 rounded-md border border-paper-edge bg-transparent text-sm text-ink-soft cursor-pointer"
    >Connect a Dōjō</button>
  {/snippet}
</PageHeader>

<div class="max-w-[1060px] mx-auto px-7 pb-10" data-connections-total={data.connections.length}>
  {#if form.open}
    <section data-connect-form class="mb-8 border border-paper-edge rounded-lg bg-paper-soft p-5">
      <p class="text-sm font-medium text-ink m-0 mb-1">Connect a Dōjō</p>
      <p class="text-xs text-ink-mute m-0 mb-4 max-w-[640px] leading-normal">
        Pairing needs the service membership id and a device token from your Dōjō
        console. The token is stored in your OS keychain and never read back.
      </p>

      <div class="grid grid-cols-2 gap-3">
        <label class="flex flex-col gap-1">
          {@render field('Membership id')}
          <input
            class={inputClass}
            type="text"
            placeholder="service membership uuid"
            bind:value={form.membershipId}
          />
        </label>
        <label class="flex flex-col gap-1">
          {@render field('Tenant key')}
          <input
            class={inputClass}
            type="text"
            placeholder="github/acme"
            bind:value={form.tenantKey}
          />
        </label>
        <label class="flex flex-col gap-1">
          {@render field('Registry url (optional)')}
          <input
            class={inputClass}
            type="text"
            placeholder="https://dojo.acme.internal"
            bind:value={form.registryUrl}
          />
        </label>
        <label class="flex flex-col gap-1">
          {@render field('Kind')}
          <select class={inputClass} bind:value={form.kind}>
            {#each KIND_OPTIONS as opt (opt.value)}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </label>
        <label class="flex flex-col gap-1">
          {@render field('Org slugs (optional)')}
          <input
            class={inputClass}
            type="text"
            data-testid="connect-org-slugs"
            placeholder="sensei-hq, acme"
            bind:value={form.orgSlugs}
          />
          <span class="text-xs text-ink-mute leading-normal">
            git remote owners this dōjō covers — used to suggest bindings.
          </span>
        </label>
        <label class="flex flex-col gap-1">
          {@render field('Device token')}
          <input
            class={inputClass}
            type="password"
            placeholder="paste device token"
            bind:value={form.deviceToken}
          />
        </label>
        <label class="flex flex-col gap-1">
          {@render field('Project to bind (optional)')}
          <input
            class={inputClass}
            type="text"
            placeholder="project uuid"
            bind:value={form.projectId}
          />
        </label>
      </div>

      <div class="flex items-center gap-3 mt-4">
        <Button
          variant="primary"
          size="sm"
          data-connect-submit
          disabled={!form.canSubmit}
          onclick={connect}
        >{form.submitting ? 'Connecting…' : 'Connect'}</Button>
        <Button
          variant="secondary"
          style="outline"
          size="sm"
          onclick={() => (form.open = false)}
        >Cancel</Button>
        {#if form.error}
          <span class="text-xs text-danger" data-connect-error>{form.error}</span>
        {/if}
      </div>
    </section>
  {/if}

  {#if data.connections.length === 0}
    <EmptyState
      kanji="連"
      title="no dōjō connected"
      description="Connect one to share and receive team learnings. Employer, client, and community Dōjōs each keep their own scopes."
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
