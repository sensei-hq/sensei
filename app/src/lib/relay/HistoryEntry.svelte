<script lang="ts">
  // One entry of a task's history (CONTRACT §1e.6; COMPONENTS §3d HistoryEntry): a node on
  // the timeline in its outcome's tone, and a card — the title, the outcome as a pill and
  // the time; who; what was said in a block edged in that tone; for a work/rework turn its
  // files, gates and telemetry folded. Each review its outcome, summary and comments; the
  // merge its note; a message to the crew with the receipts folded into it. A number
  // nobody reported reads "not reported", never 0 (D83).
  import { Pill } from '@rokkit/ui';
  import type { RelayAction, RelayLaneRef } from '$lib/relay-api.js';
  import type { RelayTone } from '@sensei/state';
  import { codeOf, hhmm } from './relay-format.js';

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

  /** The entry's tone: the node's ring, the edge of what was said, the outcome pill. */
  const tone = $derived<RelayTone>(
    action.kind === 'merged'
      ? 'success'
      : action.kind === 'reviewed'
        ? action.props.verdict === 'approved'
          ? 'success'
          : 'warning'
        : action.kind === 'interrupted'
          ? 'danger'
          : action.kind === 'asked'
            ? 'warning'
            : 'accent',
  );
  // Literal, so UnoCSS finds each.
  const NODE: Record<RelayTone, string> = {
    success: 'border-success',
    warning: 'border-warning',
    danger: 'border-danger',
    accent: 'border-accent',
    neutral: 'border-paper-edge-strong',
  };
  const EDGE: Record<RelayTone, string> = {
    success: 'border-l-success',
    warning: 'border-l-warning',
    danger: 'border-l-danger',
    accent: 'border-l-accent',
    neutral: 'border-l-paper-edge-strong',
  };

  const outcome = $derived<{ label: string; tone: RelayTone } | null>(
    action.kind === 'reviewed'
      ? { label: action.props.verdict === 'approved' ? 'approved' : 'changes requested', tone }
      : action.kind === 'merged'
        ? { label: 'merged', tone: 'success' }
        : null,
  );
</script>

{#snippet say(label: string | null, body: string | null)}
  {#if body}
    <div data-entry-say class="flex flex-col gap-0.5 py-2 px-3.5 rounded-r-lg border-l-[3px] border-solid border-y-0 border-r-0 bg-paper-soft {EDGE[tone]}">
      {#if label}<span data-entry-say-label class="text-xs text-ink-soft">{label}</span>{/if}
      <p class="m-0 text-sm text-ink">{body}</p>
    </div>
  {/if}
{/snippet}

<article
  data-history-entry={action.id}
  class="relative flex flex-col gap-2 py-2.5 px-3.5 rounded-[10px] border border-solid border-paper-edge bg-paper"
>
  <span
    data-entry-node
    data-tone={tone}
    aria-hidden="true"
    class="absolute -left-[29px] top-3 w-3 h-3 rounded-full border-2 border-solid bg-paper {NODE[tone]}"
  ></span>

  <div class="flex flex-wrap items-center gap-x-2.5 gap-y-1">
    <p data-entry-title class="m-0 text-base font-semibold">
      {#if action.kind === 'assigned'}Assigned
      {:else if action.kind === 'reported'}{action.props.partial ? 'Partial · ' : ''}{turnTitle(action)}
      {:else if action.kind === 'reviewed'}{reviewTitle(action)}
      {:else if action.kind === 'merged'}Merged
      {:else if action.kind === 'message'}{messageTitle(action)}
      {:else if action.kind === 'brief_updated'}Brief updated
      {:else if action.kind === 'interrupted'}Lease lost
      {:else if action.kind === 'reoffered'}{action.props.to_lane ? `Re-offered to ${codeOf(lanes, action.props.to_lane)}` : 'Returned to the queue'}
      {:else if action.kind === 'started'}Started
      {:else if action.kind === 'asked'}{askTitle(action)}
      {:else if action.kind === 'answered'}Answered
      {:else if action.kind === 'compacted'}Compacted
      {:else if action.kind === 'handed_over'}Handed over from {codeOf(lanes, action.props.from_lane)} to {codeOf(lanes, action.props.to_lane)}
      {/if}
    </p>
    {#if outcome}<span data-entry-outcome><Pill value={outcome.label} tone={outcome.tone} size="sm" /></span>{/if}
    <span data-entry-time class="ml-auto font-mono text-xs text-ink-mute">{hhmm(action.created_at)}</span>
  </div>
  {#if action.lane_code}<p data-entry-by class="m-0 text-xs text-ink-soft">{action.lane_code}</p>{/if}

  {#if action.kind === 'assigned'}
    {@render say(null, action.body_md)}

  {:else if action.kind === 'reported'}
    {@render say(action.props.partial ? 'Progress so far (still working)' : 'Summary sent with the request for review', action.body_md)}
    {#if action.props.look_at}<p class="text-xs m-0">Look at <code>{action.props.look_at}</code></p>{/if}
    {#if action.props.flags}<p class="text-xs m-0">{action.props.flags}</p>{/if}
    <details data-turn-more class="text-xs text-ink-mute">
      <summary class="cursor-pointer text-accent">files · gates · telemetry</summary>
      <div class="flex flex-col gap-1 pt-1.5">
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
    </details>

  {:else if action.kind === 'reviewed'}
    {@render say('Review summary', action.body_md)}
    {#each action.props.findings as finding, i (finding.class + i)}
      <p class="text-xs m-0" data-finding>
        <span class="font-medium">{finding.class}</span>
        {#if finding.file} · <code>{finding.file}{finding.line ? `:${finding.line}` : ''}</code>{/if}
        · {finding.text}
      </p>
    {/each}

  {:else if action.kind === 'merged'}
    {@render say('Merge note', action.body_md)}
    <p class="text-xs text-ink-mute m-0">{action.props.commits.length} commits into {action.props.into}</p>

  {:else if action.kind === 'message'}
    <p class="text-xs text-ink-mute m-0">
      To {action.props.to.kind === 'lane' ? codeOf(lanes, action.props.to.lane_id) : action.props.to.kind}
      <span class="rounded-sm bg-paper-soft px-1" data-message-state={messageState ?? 'sent'}>{messageState ?? 'sent'}</span>
    </p>
    {@render say(null, action.body_md)}

  {:else if action.kind === 'brief_updated'}
    {@render say(null, action.body_md)}
    {#each action.props.docs as doc (doc)}<code class="text-xs">{doc}</code>{/each}

  {:else if action.kind === 'asked'}
    {@render say(null, action.body_md)}

  {:else if action.kind === 'answered'}
    <p class="text-xs text-ink-mute m-0">{action.props.option} · from the {action.props.surface}</p>

  {:else if action.kind === 'compacted'}
    <p class="text-xs text-ink-mute m-0">{num(action.props.context_tokens_before)} → {num(action.props.context_tokens_after)} tokens</p>

  {:else if action.kind === 'handed_over'}
    {@render say(null, action.body_md)}
  {/if}
</article>
