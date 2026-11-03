<script lang="ts">
  // Settings · Dōjō connection (user, 2026-11-03): identities first; a connected
  // identity lists its dōjōs and what each has filed. This file only composes:
  // every decision is in DojoConnectionScreen (dojo-connection-state.svelte.ts),
  // the identity facts and actions in personaList.
  import { railIcon } from '../settings-nav';
  import { Toggle } from '@rokkit/ui';
  import { Notice } from '@sensei/ui';
  import { PageHeader, Screen, SignInPending } from '$lib/components';
  import { personaList } from '$lib/personas.svelte.js';
  import { TRAVELS, type RepoFilter } from './dojo-connection.js';
  import { DojoConnectionScreen } from './dojo-connection-state.svelte.js';
  import TravelsPanel from './TravelsPanel.svelte';
  import IdentityCards from './IdentityCards.svelte';
  import IdentityHeader from './IdentityHeader.svelte';
  import DojoRepos from './DojoRepos.svelte';

  let { data } = $props();

  // The identity clock moves while this screen is open, so "30 minutes left"
  // reaches "credential expired" on its own.
  $effect(() => {
    void personaList.load();
    personaList.start();
    return () => personaList.stop();
  });

  const screen = new DojoConnectionScreen(
    () => ({
      identities: personaList.identities,
      phase: personaList.phase,
      identitiesError: personaList.error,
      memberships: data.memberships ?? null,
      membershipsError: data.membershipsError ?? null,
      sync: data.sync?.entities ?? null,
      syncError: data.error ?? null,
    }),
    {
      act: (id) => personaList.act(id),
      makeDefault: (id) => personaList.makeDefault(id),
      signOut: (id) => personaList.signOut(id),
    },
  );

  const filterOptions = $derived(screen.filters.map((f) => ({ value: f.key, label: f.label, count: f.count })));
</script>

<Screen>
  {#snippet header()}
    <PageHeader
      eyebrow="Settings"
      kanji="結"
      icon={railIcon('/settings/dojo')}
      title="Dōjō connection"
      description="Connect each of your GitHub identities to its dōjōs. Once connected, you'll see which repositories sync and why."
    />
  {/snippet}

  <div class="max-w-[1100px] mx-auto px-6 md:px-12 pt-8 pb-16 flex flex-col gap-6" data-screen="settings-dojo">
    <TravelsPanel travels={TRAVELS} />

    {#if screen.identitiesNotice}
      <Notice size="panel" {...screen.identitiesNotice} data-identities-notice />
    {/if}

    {#if personaList.phase === 'loading'}
      <p class="m-0 text-sm text-ink-mute" data-personas-loading>Reading identities…</p>
    {:else if personaList.phase === 'empty'}
      <Notice size="panel" tone="normal" title="No identities yet"
              detail="sensei infers them from commit authorship as you work." data-personas-empty />
    {:else if personaList.phase === 'list' && screen.header}
      <div class="flex flex-wrap items-start gap-6">
        <section aria-labelledby="ids-h" class="flex-[2_1_300px] min-w-0 flex flex-col gap-2.5">
          <h2 id="ids-h" class="m-0 font-display text-lg font-normal text-ink">Identities</h2>
          <IdentityCards cards={screen.cards} onselect={(label) => screen.select(label)} />
          <p class="m-0 text-xs text-ink-mute">Identities are inferred from commit authorship on this machine. Sign-in happens in your system browser.</p>
        </section>

        <section aria-label="Selected identity" class="flex-[3_1_480px] min-w-0 flex flex-col gap-4">
          <IdentityHeader
            header={screen.header}
            onprimary={() => screen.runPrimary()}
            onmakedefault={() => screen.runMakeDefault()}
            onsignout={() => screen.runSignOut()}
          />
          <div data-sign-in-pending><SignInPending pending={personaList.pending} /></div>

          {#if screen.body === 'connect'}
            <ol data-connect-steps class="m-0 pl-5 flex flex-col gap-1.5 text-sm text-ink-mute">
              {#each screen.connectSteps as step (step)}<li>{step}</li>{/each}
            </ol>
          {:else if screen.body === 'attention' && screen.attention}
            <Notice size="panel" {...screen.attention} />
          {:else if screen.body === 'dojos'}
            {#if screen.loadNotices.length > 0}
              <div data-load-notices class="flex flex-col gap-2">
                {#each screen.loadNotices as n (n.code)}<Notice size="panel" {...n} />{/each}
              </div>
            {/if}
            {#if screen.identityIssues.length > 0}
              <div data-identity-issues class="flex flex-col gap-2">
                {#each screen.identityIssues as n (n.key)}<Notice size="inline" {...n} expanded />{/each}
              </div>
            {/if}
            {#if screen.noDojos}
              <Notice size="panel" {...screen.noDojos} data-no-dojos />
            {:else if screen.dojos.length > 0 || screen.filter !== 'all'}
              <div class="flex flex-wrap items-center justify-between gap-3">
                <p class="m-0 text-sm text-ink-mute" data-summary>{screen.summary}</p>
                <Toggle
                  options={filterOptions}
                  value={screen.filter}
                  onchange={(v: unknown) => screen.setFilter(v as RepoFilter)}
                  label="Filter repositories"
                  size="sm"
                >
                  {#snippet itemContent(proxy)}
                    <span data-filter={proxy.get('value')}>{proxy.label}</span>
                    <span class="font-mono text-xs">{proxy.get('count')}</span>
                  {/snippet}
                </Toggle>
              </div>
              <DojoRepos dojos={screen.dojos} />
            {/if}
            {#if screen.activity.length > 0}
              <details class="text-sm text-ink-mute" data-activity>
                <summary class="cursor-pointer">Sync activity</summary>
                <ul class="m-0 mt-2 pl-4 flex flex-col gap-1 font-mono text-xs">
                  {#each screen.activity as a (a.key)}
                    <li data-entity={a.key}>{a.label} · <span class={a.toneClass}>{a.line}</span></li>
                  {/each}
                </ul>
              </details>
            {/if}
          {/if}
        </section>
      </div>
    {/if}
  </div>
</Screen>
