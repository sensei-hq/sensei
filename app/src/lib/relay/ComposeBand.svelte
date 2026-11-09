<script lang="ts">
  // Message the crew (CONTRACT §1a, §3h-bis). *To*: the coordinator, any
  // contributor lane, or everyone. *About*: no particular task, or one of the
  // crew's tasks. The person uses the same bus as the assistants; the message
  // reaches a lane ahead of new work and shows in its task's history as sent,
  // delivered or read.
  //
  // The main window draws more than one crew, so it also asks which crew: with a
  // single crew the field is not there and the choice is not the person's to make.
  import { untrack } from 'svelte';
  import { Notice } from '@sensei/ui';
  import type { RelayCrew, RelayLane, RelayMessageTo, RelaySendMessage } from '$lib/relay-api.js';
  import { crewRepoLabel } from './relay-format.js';

  let {
    crews,
    lanes,
    send,
    onclose,
  }: {
    crews: RelayCrew[];
    lanes: RelayLane[];
    send: (body: RelaySendMessage) => Promise<string | null>;
    onclose: () => void;
  } = $props();

  const LIMIT = 600;

  // Seeded once: the band opens on the first crew the window shows.
  let crewId = $state(untrack(() => crews[0]?.id ?? ''));
  let to = $state('coordinator');
  let about = $state('');
  let text = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);
  let queued = $state(false);

  const crewLanes = $derived(lanes.filter((l) => l.crew_id === crewId));
  const contributors = $derived(crewLanes.filter((l) => l.role === 'contributor'));
  const tasks = $derived(
    crewLanes
      .flatMap((l) => (l.task ? [{ id: l.task.id, label: `${l.code} · ${l.task.title}` }] : []))
      .filter((t, i, all) => all.findIndex((o) => o.id === t.id) === i),
  );

  const target = $derived.by<RelayMessageTo>(() =>
    to === 'coordinator'
      ? { kind: 'coordinator' }
      : to === 'everyone'
        ? { kind: 'everyone' }
        : { kind: 'lane', lane_id: to },
  );
  const ready = $derived(text.trim().length > 0 && text.length <= LIMIT);

  async function queue(): Promise<void> {
    if (busy || !ready || !crewId) return;
    busy = true;
    error = null;
    const refused = await send({
      crew_id: crewId,
      to: target,
      task_id: about || null,
      text,
      surface: 'desktop',
    });
    busy = false;
    if (refused) {
      error = refused;
      return;
    }
    queued = true;
    text = '';
  }
</script>

<div class="flex flex-col gap-2 border-b border-paper-edge bg-paper-soft p-4" data-compose>
  <div class="flex flex-wrap items-center gap-3">
    {#if crews.length > 1}
      <label class="flex items-center gap-2 text-xs text-ink-mute">
        <span>Crew</span>
        <select
          data-compose-crew
          class="rounded-sm border border-paper-edge bg-paper py-1 px-2 text-sm text-ink"
          bind:value={crewId}
        >
          {#each crews as crew (crew.id)}
            <option value={crew.id}>{crewRepoLabel(crew)}</option>
          {/each}
        </select>
      </label>
    {/if}

    <label class="flex items-center gap-2 text-xs text-ink-mute">
      <span>To</span>
      <select
        data-compose-to
        class="rounded-sm border border-paper-edge bg-paper py-1 px-2 text-sm text-ink"
        bind:value={to}
      >
        <option value="coordinator">The coordinator</option>
        {#each contributors as lane (lane.id)}
          <option value={lane.id}>{lane.code} · {lane.title ?? lane.code}</option>
        {/each}
        <option value="everyone">Everyone</option>
      </select>
    </label>

    <label class="flex items-center gap-2 text-xs text-ink-mute">
      <span>About</span>
      <select
        data-compose-about
        class="rounded-sm border border-paper-edge bg-paper py-1 px-2 text-sm text-ink"
        bind:value={about}
      >
        <option value="">No particular task</option>
        {#each tasks as task (task.id)}
          <option value={task.id}>{task.label}</option>
        {/each}
      </select>
    </label>

    <button type="button" class="ml-auto text-xs underline" onclick={onclose}>Close</button>
  </div>

  <textarea
    class="w-full min-h-20 rounded-sm border border-paper-edge bg-paper py-2 px-3 text-sm text-ink"
    placeholder="What should the crew know?"
    data-compose-text
    bind:value={text}
  ></textarea>

  <div class="flex items-center gap-3">
    <span class="text-xs text-ink-faint">{text.length} / {LIMIT}</span>
    {#if queued}<span class="text-xs text-success" data-compose-queued>Queued for the crew.</span>{/if}
    {#if error}<Notice size="inline" tone="fault" title={error} data-compose-error />{/if}
    <button
      type="button"
      class="ml-auto rounded-sm bg-accent px-3 py-1 text-xs text-on-accent disabled:opacity-60"
      data-compose-send
      disabled={!ready || busy}
      onclick={() => void queue()}
    >
      Queue message
    </button>
  </div>
</div>
