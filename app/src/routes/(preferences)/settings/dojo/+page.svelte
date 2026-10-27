<script lang="ts">
    import { railIcon } from "../settings-nav";
  import { invalidateAll } from '$app/navigation';
  import { PageHeader, Screen, ScreenState, SignInPending } from "$lib/components";
  import { personaList } from '$lib/personas.svelte.js';
  import {
    agreementLine,
    detailLine,
    entityLabel,
    groupSync,
    summarise,
    syncTone,
    type SyncItem,
  } from './dojo-status-state.svelte.js';

  let { data } = $props();

  // The credential registry is the same singleton the sign-in overlay uses, so
  // the standing shown here and the standing that opens the overlay can never
  // disagree. `load()` resolves without throwing and records its own error.
  $effect(() => {
    void personaList.load();
  });

  // Recomputed on a timer, because the whole point of this screen is watching a
  // credential approach expiry — a `now` captured once would freeze "30 minutes
  // left" and never reach zero. 30s is well inside the one-hour warning band.
  let now = $state(Math.floor(Date.now() / 1000));
  $effect(() => {
    const id = setInterval(() => (now = Math.floor(Date.now() / 1000)), 30_000);
    return () => clearInterval(id);
  });

  // Each identity's sync, by day and repository, under the identity's NAME —
  // `default` is a Keychain slot, and read as "the default identity".
  const grouped = $derived(
    data.sync
      ? groupSync(
          data.sync.entities,
          personaList.personas.map((p) => ({ slot: p.sessionSlot, label: p.label })),
        )
      : [],
  );
  const syncOf = (label: string | null) => grouped.find((g) => g.label === label);
  const unattributed = $derived(syncOf(null));

  const toneClass = (tone: string) =>
    tone === 'dead' || tone === 'danger'
      ? 'text-danger'
      : tone === 'warn'
        ? 'text-warning'
        : tone === 'ok' || tone === 'success'
          ? 'text-success'
          : tone === 'info'
            ? 'text-info'
            : 'text-ink-mute';
</script>

<Screen>
    {#snippet header()}
    <PageHeader
      eyebrow="Settings"
      kanji="結" icon={railIcon("/settings/dojo")}
      title="Dōjō"
      description="The credential that reaches your dōjō, and what has been agreed with it. A forge token lasts about eight hours and renews itself silently; this is where you can see one running out before it does."
    />
    {/snippet}

    <div class="max-w-[820px] mx-auto px-12 pt-8 pb-16" data-screen="settings-dojo">
      <section>
        <h2 class="display text-base font-medium m-0 mb-1 text-ink">Identities</h2>
        <p class="text-xs text-ink-soft mt-0 mb-3 max-w-[620px] leading-normal">
          Each identity's credential, and what it has agreed with its dōjō — by day, by repository.
          Renewal is automatic; it only needs you when the session behind it has gone. A skipped
          item is a decision, not a fault.
        </p>

        {#if data.sync && data.sync.count > 0}
          <p class="text-xs text-ink-mute mt-0 mb-3" data-summary>{summarise(data.sync.counts)}</p>
        {/if}
        {#if data.error || !data.sync}
          <ScreenState
            status="error"
            error={data.error ?? 'Failed to load sync state'}
            onretry={invalidateAll}
          />
        {/if}

        {#if personaList.error}
          <!-- A failed registry read is NOT "no identities": that phrasing invites
               connecting one, which is the wrong action against a registry that is
               merely unreachable. -->
          <p class="text-xs text-danger m-0" data-persona-error>{personaList.error}</p>
        {:else if personaList.personas.length === 0}
          <p class="text-sm text-ink-mute m-0" data-personas-empty>
            No identities yet. sensei infers them from commit authorship as you work.
          </p>
        {:else}
          {#if personaList.defaultError}
            <p class="text-xs text-ink-mute mt-0 mb-3" data-default-error>{personaList.defaultError}</p>
          {/if}
          <div class="mb-3" data-sign-in-pending><SignInPending pending={personaList.pending} /></div>
          <div class="flex flex-col gap-4" data-personas>
            {#each personaList.personas as p (p.label)}
              <div
                class="border border-paper-edge rounded-lg overflow-hidden"
                data-identity={p.label}
                data-persona={p.label}
                data-tone={personaList.tone(p, now)}
              >
                <div class="grid grid-cols-[1fr_auto] gap-6 items-center px-4 py-3 bg-paper-soft">
                  <div>
                    <div class="text-sm text-ink">
                      <span data-identity-name>{p.label}</span>
                      {#if p.isDefault}
                        <span class="ml-1 text-xs text-ink-mute" data-default-persona>default</span>
                      {:else}
                        <button
                          type="button"
                          class="ml-2 text-xs text-ink-soft underline cursor-pointer bg-transparent border-0 p-0
                                 disabled:opacity-50"
                          disabled={personaList.isBusy(p)}
                          onclick={() => personaList.makeDefault(p)}
                          data-make-default
                        >
                          Make default
                        </button>
                      {/if}
                    </div>
                    <!-- `describe` is PersonaList's, not this screen's: the overlay
                         shows the same sentence, and a second copy would drift. -->
                    <div class="text-xs mt-0.5 {toneClass(personaList.tone(p, now))}" data-standing>
                      {personaList.describe(p, now)}
                    </div>
                  </div>
                  {#if p.action !== 'none'}
                    <button
                      type="button"
                      class="text-xs px-3 py-1.5 rounded-md border border-paper-edge text-ink
                             hover:bg-paper-soft cursor-pointer disabled:opacity-50"
                      disabled={personaList.isBusy(p)}
                      onclick={async () => { await personaList.act(p); await personaList.load(); }}
                    >
                      {personaList.actionLabel(p)}
                    </button>
                  {/if}
                </div>
                {@render syncDays(syncOf(p.label)?.days ?? [])}
              </div>
            {/each}

            {#if unattributed}
              <!-- Written before each record named its identity; the next pass fills it in. -->
              <div class="border border-paper-edge rounded-lg overflow-hidden" data-identity="">
                <div class="px-4 py-3 bg-paper-soft text-sm text-ink-mute">Not yet attributed to an identity</div>
                {@render syncDays(unattributed.days)}
              </div>
            {/if}
          </div>
        {/if}

        <p class="text-xs text-ink-faint mt-3 mb-0">
          Connections themselves — which dōjōs, their kind and tenant — are managed
          under <a class="underline" href="/dojo/connections">Dōjō · Connections</a>.
        </p>
      </section>
    </div>

</Screen>

{#snippet syncDays(days: import('./dojo-status-state.svelte.js').SyncDay[])}
  {#each days as d (d.day)}
    <div class="px-4 pt-3 pb-1 text-xs uppercase tracking-wide text-ink-mute" data-day={d.day ?? ''}>
      {d.day ?? 'never'}
    </div>
    {#each d.own as item (item.row.entity + item.row.direction)}
      <div class="px-4 py-2">{@render syncLine(item)}</div>
    {/each}
    {#each d.repositories as r (r.name)}
      <div class="px-4 py-2 border-t border-paper-edge" data-repository={r.name}>
        <div class="text-sm text-ink truncate">{r.name}</div>
        {#each r.items as item (item.row.entity_key + item.row.entity + item.row.direction)}
          <div class="pl-3 mt-1">{@render syncLine(item)}</div>
        {/each}
      </div>
    {/each}
  {/each}
{/snippet}

{#snippet syncLine(item: SyncItem)}
  {@const detail = detailLine(item.row)}
  <div data-entity={item.row.entity} data-state={item.row.state} data-branch={item.branch ?? undefined}>
    <div class="flex flex-wrap items-baseline gap-x-3 text-xs">
      <span class="text-ink-soft">
        {entityLabel(item.row.entity)} · {item.row.direction}{#if item.branch}<span class="text-ink-mute"> · {item.branch}</span>{/if}
      </span>
      <span class={toneClass(syncTone(item.row.state))} data-agreement>{agreementLine(item.row)}</span>
    </div>
    {#if detail}
      <!-- Shown for `skipped` too, where it is the REASON (in words) rather than a
           failure — the tone is what distinguishes them. -->
      <div class="text-xs text-ink-faint mt-0.5" data-detail>{detail}</div>
    {/if}
  </div>
{/snippet}
