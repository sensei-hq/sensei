<script lang="ts">
  // Test wrapper — exposes the EnrichedProject surface as plain props so the
  // spec can drive every card variant without assembling wire objects.
  import ProjectCard from './ProjectCard.svelte';
  import type { EnrichedProject } from './buckets.js';

  let {
    name = 'sensei',
    client = null,
    vision = null,
    maturity = 'learning',
    ftr14d = 0.9,
    sessions7d = 4,
    warn = false,
    repos_count = 3,
    libs_count = 12,
    last_session_at = null,
    icon = null,
    onOpen,
  }: {
    name?: string;
    client?: string | null;
    vision?: string | null;
    maturity?: string;
    ftr14d?: number;
    sessions7d?: number;
    warn?: boolean;
    repos_count?: number;
    libs_count?: number;
    last_session_at?: string | null;
    icon?: { kind?: string; value?: string } | null;
    onOpen?: (id: string, name: string) => void;
  } = $props();

  const p = $derived<EnrichedProject>({
    id: 'p-1',
    name,
    client,
    vision,
    maturity,
    stack: {},
    ftr14d,
    sessions7d,
    warn,
    repos_count,
    libs_count,
    last_session_at,
    icon,
  });
</script>

<ProjectCard {p} {onOpen} />
