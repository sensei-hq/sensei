<script lang="ts">
  // One entry of a task's history (CONTRACT §1e.6): each work/rework turn with the
  // summary sent with the request for review and a fold of its files, gates and
  // telemetry; each review its outcome, summary and comments; the merge its note;
  // a message to the crew with the receipts folded into it. A number nobody
  // reported reads "not reported", never 0 (D83).
  import type { RelayAction, RelayLaneRef } from '$lib/relay-api.js';
  import { codeOf, hhmm, providerLabel } from './relay-format.js';

  let {
    action,
    lanes,
    messageState = null,
  }: {
    action: RelayAction;
    /** The crew's lanes, so a uuid in props reads as its code. */
    lanes: RelayLaneRef[];
    /** A message's receipts, folded: sent, delivered or read. */
    messageState?: 'sent' | 'delivered' | 'read' | null;
  } = $props();

  const num = (n: number | null | undefined) => (n == null ? 'not reported' : new Intl.NumberFormat('en').format(n));
  const turnTitle = (a: Extract<RelayAction, { kind: 'reported' }>) =>
    `${(a.round ?? 1) > 1 ? 'Rework' : 'Work'} · ${a.phase ?? 'work'} r${a.round ?? 1}`;
  const reviewTitle = (a: Extract<RelayAction, { kind: 'reviewed' }>) =>
    `Review · ${a.phase ?? 'work'} r${a.round ?? 1}`;
  // Q12 puts lanes in this history: they send messages and ask other lanes. `lane_code`
  // is who acted (null = the person); `asked_of` is `person` or the lane asked.
  const messageTitle = (a: Extract<RelayAction, { kind: 'message' }>) =>
    a.lane_code ? `Message from ${a.lane_code}` : 'Message from you';
  const askTitle = (a: Extract<RelayAction, { kind: 'asked' }>) =>
    a.props.asked_of === 'person' ? 'Asked you' : `Asked ${codeOf(lanes, a.props.asked_of)}`;
</script>

<article class="flex flex-col gap-2 rounded-sm border border-paper-edge p-3" data-history-entry={action.id}>
  {#if action.kind === 'assigned'}
    <p class="text-sm font-medium m-0" data-entry-title>Assigned</p>
    {#if action.body_md}<p class="text-sm text-ink-soft m-0">{action.body_md}</p>{/if}

  {:else if action.kind === 'reported'}
    <p class="text-sm font-medium m-0" data-entry-title>{turnTitle(action)}</p>
    <p class="text-xs text-ink-faint m-0">Summary sent with the request for review</p>
    {#if action.body_md}<p class="text-sm text-ink-soft m-0">{action.body_md}</p>{/if}
    {#if action.props.look_at}<p class="text-xs m-0">Look at <code>{action.props.look_at}</code></p>{/if}
    {#if action.props.flags}<p class="text-xs m-0">{action.props.flags}</p>{/if}
    <div class="flex flex-col gap-1 text-xs text-ink-mute" data-turn-more>
      {#each action.props.files ?? [] as file (file.path)}
        <span class="flex items-center gap-2" data-turn-file={file.path}>
          <code>{file.path}</code>
          <span class="text-success">+{file.added}</span>
          <span class="text-ink-mute">−{file.removed}</span>
          {#if file.outside_owned}<span class="rounded-sm bg-warning-soft px-1" data-outside-owned>outside owned paths</span>{/if}
        </span>
      {/each}
      {#each action.props.gates ?? [] as gate (gate.cmd)}
        <span><code>{gate.cmd}</code> · exit {gate.exit}{gate.counts ? ` · ${gate.counts}` : ''}</span>
      {/each}
      <span data-turn-telemetry>
        tokens in {num(action.props.tokens_in)} · out {num(action.props.tokens_out)} · cache {num(action.props.tokens_cache)}
      </span>
    </div>

  {:else if action.kind === 'reviewed'}
    <p class="text-sm font-medium m-0" data-entry-title>{reviewTitle(action)}</p>
    <p class="text-xs rounded-sm bg-paper-soft px-2 py-0.5 w-fit m-0" data-entry-outcome>
      {action.props.verdict === 'approved' ? 'approved' : 'changes requested'}
    </p>
    <p class="text-xs text-ink-faint m-0">Review summary</p>
    {#if action.body_md}<p class="text-sm text-ink-soft m-0">{action.body_md}</p>{/if}
    {#each action.props.findings as finding, i (finding.class + i)}
      <p class="text-xs m-0" data-finding>
        <span class="font-medium">{finding.class}</span>
        {#if finding.file} · <code>{finding.file}{finding.line ? `:${finding.line}` : ''}</code>{/if}
        · {finding.text}
      </p>
    {/each}

  {:else if action.kind === 'merged'}
    <p class="text-sm font-medium m-0" data-entry-title>Merged</p>
    <p class="text-xs text-ink-faint m-0">Merge note</p>
    {#if action.body_md}<p class="text-sm text-ink-soft m-0">{action.body_md}</p>{/if}
    <p class="text-xs text-ink-mute m-0">
      {action.props.commits.length} commits into {action.props.into}
    </p>

  {:else if action.kind === 'message'}
    <p class="text-sm font-medium m-0" data-entry-title>{messageTitle(action)}</p>
    <p class="text-xs text-ink-mute m-0">
      To {action.props.to.kind === 'lane' ? codeOf(lanes, action.props.to.lane_id) : action.props.to.kind}
      <span class="rounded-sm bg-paper-soft px-1" data-message-state={messageState ?? 'sent'}>{messageState ?? 'sent'}</span>
    </p>
    {#if action.body_md}<p class="text-sm text-ink-soft m-0">{action.body_md}</p>{/if}

  {:else if action.kind === 'brief_updated'}
    <p class="text-sm font-medium m-0" data-entry-title>Brief updated</p>
    {#if action.body_md}<p class="text-sm text-ink-soft m-0">{action.body_md}</p>{/if}
    {#each action.props.docs as doc (doc)}<code class="text-xs">{doc}</code>{/each}

  {:else if action.kind === 'interrupted'}
    <p class="text-sm font-medium m-0" data-entry-title>Lease lost</p>

  {:else if action.kind === 'reoffered'}
    <p class="text-sm font-medium m-0" data-entry-title>
      {action.props.to_lane ? `Re-offered to ${codeOf(lanes, action.props.to_lane)}` : 'Returned to the queue'}
    </p>

  {:else if action.kind === 'started'}
    <p class="text-sm font-medium m-0" data-entry-title>Started</p>

  {:else if action.kind === 'asked'}
    <p class="text-sm font-medium m-0" data-entry-title>{askTitle(action)}</p>
    {#if action.body_md}<p class="text-sm text-ink-soft m-0">{action.body_md}</p>{/if}

  {:else if action.kind === 'answered'}
    <p class="text-sm font-medium m-0" data-entry-title>Answered</p>
    <p class="text-xs text-ink-mute m-0">
      {action.props.option} · from the {action.props.surface}
    </p>

  {:else if action.kind === 'compacted'}
    <p class="text-sm font-medium m-0" data-entry-title>Compacted</p>
    <p class="text-xs text-ink-mute m-0">
      {num(action.props.context_tokens_before)} → {num(action.props.context_tokens_after)} tokens
    </p>
  {/if}

  <p class="text-xs text-ink-faint m-0">{hhmm(action.created_at)}{action.lane_code ? ` · ${action.lane_code}` : ''}</p>
</article>
