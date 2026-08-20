<script lang="ts">
  // Test wrapper — exposes the EnrichedSession surface as plain props so the
  // spec can drive every row variant without assembling wire rows + enrich().
  import SessionRow from './SessionRow.svelte';
  import type { EnrichedSession, QualityTone } from '$lib/sessions-digest.js';

  let {
    id = 's-1',
    title = 'Fix auth',
    project = 'sensei',
    agent = 'claude',
    outcome = 'completed',
    ftr = true,
    turns = 10,
    corrections = 0,
    quality = 'good',
    mins = 42,
    time = '09:14',
    duration = '42m',
    when = 'today',
    folderRole = null,
    tokens = 7732,
    tokensIn = 6000,
    tokensOut = 1732,
    tokensLabel = '7.7k',
    activeMins = 30,
    activeLabel = '30m',
    model = 'claude-opus-4-8',
    onselect,
  }: {
    id?: string;
    title?: string;
    project?: string;
    agent?: string | null;
    outcome?: string;
    ftr?: boolean | null;
    turns?: number;
    corrections?: number;
    quality?: QualityTone;
    mins?: number | null;
    time?: string;
    duration?: string;
    when?: string;
    folderRole?: string | null;
    tokens?: number | null;
    tokensIn?: number | null;
    tokensOut?: number | null;
    tokensLabel?: string;
    activeMins?: number | null;
    activeLabel?: string;
    model?: string | null;
    onselect?: (id: string) => void;
  } = $props();

  const session = $derived<EnrichedSession>({
    id,
    project,
    title,
    agent,
    outcome,
    ftr,
    turns,
    corrections,
    startedAt: '2026-07-08T09:14:00Z',
    completedAt: null,
    quality,
    mins,
    dayKey: '2026-07-08',
    when,
    time,
    duration,
    folderRole,
    tokens,
    tokensIn,
    tokensOut,
    tokensLabel,
    activeMins,
    activeLabel,
    model,
  });
</script>

<SessionRow {session} onselect={onselect ?? (() => {})} />
