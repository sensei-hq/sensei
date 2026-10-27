<script lang="ts">
  // The Integrations screen (docs/spec/screen/workspace-integrations.md).
  // PRESENTATIONAL: every decision, word and request is integrations-state's;
  // this renders it and reports clicks.
  import { PageHeader, Screen } from '$lib/components';
  import ModalShell from '$lib/components/ModalShell.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { CREATE_TOKEN_URL, LEVELS, type TrackerIntegration } from './integrations-state.svelte.js';

  let { screen }: { screen: TrackerIntegration } = $props();

  const ALERT_ID = 'tracker-alert';
  let alertEl = $state<HTMLElement | null>(null);

  // A failure takes focus, so it is read out and acted on at once.
  $effect(() => {
    if (screen.alertSeq > 0 && alertEl) alertEl.focus();
  });

  /** The alert describes the field it concerns; otherwise the field's own help. */
  const describedBy = (field: string, help: string | null) =>
    screen.alert?.field === field ? ALERT_ID : (help ?? undefined);

  /** External pages open in the system browser, not the app's webview. */
  function external(e: MouseEvent, href: string) {
    e.preventDefault();
    void openUrl(href).catch(() => window.open(href, '_blank', 'noopener'));
  }

  const button =
    'rounded-md border border-paper-edge px-3 py-1.5 text-sm text-ink hover:bg-paper-mute disabled:opacity-50';
  const input =
    'w-full px-3 py-2 border border-paper-edge rounded-md bg-paper-mute text-ink text-sm outline-none focus:border-focus-ring';
</script>

{#snippet field(
  id: string,
  name: 'board_url' | 'email' | 'token' | 'site_url' | 'board_id',
  label: string,
  type: string,
  value: string,
  set: (v: string) => void,
  help: string | null,
)}
  <div class="flex flex-col gap-1">
    <label for={id} class="text-sm text-ink">{label}</label>
    <input
      {id}
      {type}
      class={input}
      data-field={name}
      autocomplete={type === 'password' ? 'off' : undefined}
      spellcheck="false"
      aria-describedby={describedBy(name, help ? `${id}-help` : null)}
      {value}
      oninput={(e) => set((e.currentTarget as HTMLInputElement).value)}
    />
    {#if help}
      <p id="{id}-help" class="m-0 text-xs text-ink-soft">{help}</p>
    {/if}
  </div>
{/snippet}

<Screen>
  {#snippet header()}
    <PageHeader
      kanji="繋"
      icon="i-solar-link-bold-duotone"
      eyebrow="Integrations"
      title="Issue trackers"
      description="Connect this workspace’s Jira boards, each at a level, so sensei can link AI sessions to tickets and report time, tokens and cost per ticket. On tickets assigned to you, sensei writes these numbers back to Jira, as a comment or in the fields you map."
    />
  {/snippet}
<section data-screen="integrations" class="flex flex-col">

  <div class="flex max-w-[640px] flex-col gap-5 px-6 py-6">
    <p role="status" aria-live="polite" class="m-0 min-h-[1lh] text-sm text-ink-soft">{screen.statusText}</p>

    {#if screen.alert}
      <div
        id={ALERT_ID}
        bind:this={alertEl}
        role="alert"
        tabindex="-1"
        class="rounded-md border border-danger bg-danger-soft px-3 py-2 text-sm text-ink outline-none"
      >
        {screen.alert.text}
        {#if screen.alert.link}
          {#if screen.alert.link.href.startsWith('https://')}
            <a
              href={screen.alert.link.href}
              class="text-primary underline"
              onclick={(e) => external(e, screen.alert!.link!.href)}>{screen.alert.link.label}</a
            >
          {:else}
            <a href={screen.alert.link.href} class="text-primary underline">{screen.alert.link.label}</a>
          {/if}
        {/if}
      </div>
    {/if}

    {#if screen.view === 'loading'}
      <p class="m-0 text-sm text-ink-soft">Reading the connection…</p>
    {:else if screen.view === 'daemon_error'}
      <div data-view="daemon_error" class="flex flex-col gap-3 rounded-md border border-paper-edge bg-paper-soft p-4">
        <p class="m-0 text-sm text-danger">{screen.loadError}</p>
        <div>
          <button type="button" class={button} data-action="retry" onclick={() => screen.load()}>Retry</button>
        </div>
      </div>
    {:else}
      {#each screen.groups as group (group.level)}
        <section data-level={group.level} aria-labelledby="level-{group.level}" class="flex flex-col gap-3">
          <h2 id="level-{group.level}" class="m-0 font-heading text-sm text-ink">{group.label}</h2>
          {#each group.trackers as tracker (tracker.id)}
            <div data-tracker={tracker.id} class="flex flex-col gap-3 rounded-lg border border-paper-edge bg-paper-soft p-4">
              <p class="m-0 text-sm text-ink">{screen.cardTitle(tracker)}</p>
              <p class="m-0 flex items-center gap-2 text-xs text-ink-mute" data-status={tracker.status}>
                <span
                  aria-hidden="true"
                  class="inline-block size-2 rounded-full"
                  class:bg-success={tracker.status === 'connected'}
                  class:bg-warning={tracker.status === 'stale' || tracker.status === 'pending'}
                  class:bg-danger={tracker.status === 'error'}
                ></span>
                {screen.cardStatus(tracker)}
              </p>
              <div class="flex flex-wrap items-center gap-3">
                <label class="flex items-center gap-2 text-sm text-ink">
                  <input
                    type="checkbox"
                    data-action="writeback"
                    checked={tracker.writeback.enabled}
                    disabled={screen.busy !== null}
                    onclick={(e) => {
                      // The daemon's answer decides the state; never the click alone.
                      e.preventDefault();
                      void screen.toggleWriteback(tracker);
                    }}
                  />
                  Write back to Jira
                </label>
                <p class="m-0 text-xs text-ink-mute" data-writeback-state={tracker.writeback.state}>
                  {screen.writebackLine(tracker)}
                </p>
              </div>
              <div class="flex flex-wrap gap-2">
                <button
                  type="button"
                  class={button}
                  data-action="sync"
                  disabled={screen.busy !== null}
                  onclick={() => screen.sync(tracker.id)}
                  >{screen.busy === 'sync' && screen.busyTracker === tracker.id ? 'Syncing…' : 'Sync now'}</button
                >
                <button
                  type="button"
                  class={button}
                  data-action="replace"
                  disabled={screen.busy !== null}
                  onclick={() => screen.startReplace(tracker)}>Replace token</button
                >
                <button
                  type="button"
                  class="{button} text-danger"
                  data-action="disconnect"
                  disabled={screen.busy !== null}
                  onclick={(e) => {
                    // Focus it first so closing the dialog returns here, also where a
                    // click does not focus buttons (WebKit).
                    (e.currentTarget as HTMLElement).focus();
                    screen.askDisconnect(tracker);
                  }}>Disconnect</button
                >
              </div>
            </div>
          {/each}
        </section>
      {/each}

      {#if screen.view === 'connected' && !screen.showForm}
        <div>
          <button type="button" class={button} data-action="add" onclick={() => screen.startAdd()}>Add a board</button>
        </div>
      {/if}

      {#if screen.showForm}
        <form
          class="flex flex-col gap-4"
          onsubmit={(e) => {
            e.preventDefault();
            void screen.connect();
          }}
        >
          <fieldset
            class="m-0 flex flex-col gap-2 border-0 p-0"
            data-field="level"
            aria-describedby={describedBy('level', null)}
          >
            <legend class="mb-1 text-sm text-ink">What the board serves</legend>
            {#each LEVELS as l (l.level)}
              <label class="flex items-start gap-2 text-sm text-ink">
                <input
                  type="radio"
                  name="level"
                  value={l.level}
                  checked={screen.level === l.level}
                  onchange={() => (screen.level = l.level)}
                />
                <span>{l.label} <span class="text-xs text-ink-soft">· {l.help}</span></span>
              </label>
            {/each}
          </fieldset>

          <fieldset class="m-0 flex gap-4 border-0 p-0">
            <legend class="mb-1 text-sm text-ink">Tracker</legend>
            <label class="flex items-center gap-2 text-sm text-ink" data-tracker="jira">
              <input type="radio" name="tracker" value="jira" checked /> Jira
            </label>
            <span class="flex items-center gap-2 text-sm text-ink-faint" data-tracker="gitlab" aria-disabled="true">
              GitLab, coming soon
            </span>
          </fieldset>

          {#if !screen.advanced}
            {@render field(
              'tracker-board-url',
              'board_url',
              'Board URL',
              'text',
              screen.boardUrl,
              (v) => (screen.boardUrl = v),
              'Open your board in Jira and copy the address bar, e.g. https://acme.atlassian.net/jira/software/projects/AUTH/boards/42.',
            )}
          {/if}

          <div class="flex flex-col gap-3">
            <button
              type="button"
              class="self-start bg-transparent p-0 text-xs text-ink-soft underline"
              aria-expanded={screen.advanced}
              aria-controls="tracker-advanced"
              onclick={() => (screen.advanced = !screen.advanced)}
              >{screen.advanced ? 'Use a board URL instead' : 'Advanced ▸'}</button
            >
            {#if screen.advanced}
              <div id="tracker-advanced" class="flex flex-col gap-3">
                {@render field(
                  'tracker-site-url',
                  'site_url',
                  'Site URL',
                  'text',
                  screen.siteUrl,
                  (v) => (screen.siteUrl = v),
                  'Use instead of Board URL.',
                )}
                {@render field('tracker-board-id', 'board_id', 'Board id', 'text', screen.boardId, (v) => (screen.boardId = v), null)}
              </div>
            {/if}
          </div>

          {@render field(
            'tracker-email',
            'email',
            'Atlassian account email',
            'email',
            screen.email,
            (v) => (screen.email = v),
            'The email you sign in to Jira with. Atlassian needs it together with the API token.',
          )}
          {@render field(
            'tracker-token',
            'token',
            'API token',
            'password',
            screen.token,
            (v) => (screen.token = v),
            'Sign in to Atlassian with the same account as the email above, create a token, and paste it here.',
          )}
          <a
            href={CREATE_TOKEN_URL}
            data-link="create-token"
            class="self-start text-sm text-primary underline"
            onclick={(e) => external(e, CREATE_TOKEN_URL)}>Create an API token</a
          >

          <p class="m-0 text-xs text-ink-soft">
            Your token is stored in your system keychain, not in sensei’s database.
          </p>

          <div class="flex gap-2">
            <button
              type="submit"
              class="rounded-md bg-primary px-3 py-1.5 text-sm text-on-primary disabled:opacity-50"
              data-action="connect"
              disabled={!screen.canConnect || screen.busy !== null}
            >
              {screen.busy === 'connect'
                ? 'Checking credentials and board…'
                : screen.replacing !== null
                  ? 'Replace token'
                  : 'Connect'}
            </button>
            {#if screen.replacing !== null}
              <button type="button" class={button} data-action="cancel-replace" onclick={() => screen.cancelReplace()}
                >Cancel</button
              >
            {:else if screen.adding}
              <button type="button" class={button} data-action="cancel-add" onclick={() => screen.cancelAdd()}
                >Cancel</button
              >
            {/if}
          </div>
        </form>
      {/if}
    {/if}
  </div>
</section>
</Screen>

<ModalShell
  open={screen.confirmingDisconnect !== null}
  component="disconnect-dialog"
  labelledBy="disconnect-title"
  onClose={() => screen.cancelDisconnect()}
>
  <h2 id="disconnect-title" class="m-0 font-heading text-base text-ink">
    Disconnect board #{screen.confirmingDisconnect?.board_ref}?
  </h2>
  <p class="m-0 text-sm text-ink-soft">
    This removes this board’s synced sprints, tickets and session links from sensei. The workspace’s other boards stay, and Jira itself is not changed.
  </p>
  <div class="flex justify-end gap-2">
    <button type="button" class={button} data-action="cancel-disconnect" onclick={() => screen.cancelDisconnect()}
      >Cancel</button
    >
    <button
      type="button"
      class="rounded-md bg-danger px-3 py-1.5 text-sm text-on-primary"
      data-action="confirm-disconnect"
      onclick={() => screen.confirmDisconnect()}>Disconnect</button
    >
  </div>
</ModalShell>
