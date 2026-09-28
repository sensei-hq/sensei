<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { PageHeader, ScreenState } from '$lib/components';
  import { personaList } from '$lib/personas.svelte.js';
  import { agreementLine, entityLabel, summarise, syncTone } from './dojo-status-state.svelte.js';

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

<PageHeader
  eyebrow="Settings"
  kanji="結"
  title="Dōjō"
  description="The credential that reaches your dōjō, and what has been agreed with it. A forge token lasts about eight hours and renews itself silently; this is where you can see one running out before it does."
/>

<div class="max-w-[820px] mx-auto px-12 pt-8 pb-16" data-screen="settings-dojo">
  <!-- ── Credentials ──────────────────────────────────────────────────────── -->
  <section class="mb-10">
    <h2 class="display text-base font-medium m-0 mb-1 text-ink">Credentials</h2>
    <p class="text-xs text-ink-soft mt-0 mb-3 max-w-[620px] leading-normal">
      One per identity sensei knows. Renewal is automatic and takes a few seconds;
      it only needs you when the session behind it has gone.
    </p>

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
      <div class="border border-paper-edge rounded-lg overflow-hidden" data-personas>
        {#each personaList.personas as p (p.label)}
          <div
            class="grid grid-cols-[1fr_auto] gap-6 items-center px-4 py-3 border-b
                   border-paper-edge last:border-b-0"
            data-persona={p.label}
            data-tone={personaList.tone(p, now)}
          >
            <div>
              <div class="text-sm text-ink">{p.label}</div>
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
                onclick={() => personaList.signIn(p)}
              >
                {personaList.actionLabel(p)}
              </button>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- ── Sync state ───────────────────────────────────────────────────────── -->
  <section>
    <h2 class="display text-base font-medium m-0 mb-1 text-ink">What has been agreed</h2>
    <p class="text-xs text-ink-soft mt-0 mb-3 max-w-[620px] leading-normal">
      Per entity, when this machine and the dōjō last agreed. A skipped entity is a
      decision, not a fault — a private repository has nothing to send.
    </p>

    {#if data.error || !data.sync}
      <ScreenState
        status="error"
        error={data.error ?? 'Failed to load sync state'}
        onretry={invalidateAll}
      />
    {:else if data.sync.count === 0}
      <!-- Honest-empty. "all agreed" over zero entities would report a healthy
           sync on an install that has never synced anything. -->
      <p class="text-sm text-ink-mute m-0" data-empty>
        Nothing has been offered to a dōjō yet, so there is nothing to agree on.
      </p>
    {:else}
      <p class="text-xs text-ink-mute mt-0 mb-3" data-summary>{summarise(data.sync.counts)}</p>
      <div class="border border-paper-edge rounded-lg overflow-hidden" data-sync-rows>
        {#each data.sync.entities as row (`${row.entity}:${row.entity_key}:${row.direction}`)}
          <div
            class="px-4 py-3 border-b border-paper-edge last:border-b-0"
            data-entity={row.entity}
            data-state={row.state}
          >
            <div class="grid grid-cols-[1fr_auto] gap-6 items-baseline">
              <div class="text-sm text-ink truncate">{row.entity_key}</div>
              <div class="text-xs {toneClass(syncTone(row.state))}" data-agreement>
                {agreementLine(row)}
              </div>
            </div>
            <div class="text-xs text-ink-faint mt-0.5">
              {entityLabel(row.entity)} · {row.direction}
            </div>
            {#if row.last_error}
              <!-- Shown for `skipped` too, where it is the REASON rather than a
                   failure — the tone above is what distinguishes them. -->
              <div class="text-xs text-ink-soft mt-1" data-detail>{row.last_error}</div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    <p class="text-xs text-ink-faint mt-3 mb-0">
      Connections themselves — which dōjōs, their kind and tenant — are managed
      under <a class="underline" href="/dojo/connections">Dōjō · Connections</a>.
    </p>
  </section>
</div>
