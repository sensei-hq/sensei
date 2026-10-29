<script lang="ts">
  // The Wheel lens (M17): modules on a ring in their group's arc, dependencies
  // bundled through the groups' anchors. Hovering a module (or selecting one)
  // dims what does not touch it; a click opens it, a fold is selected.
  import { layoutWheel } from './wheel';
  import { bandTone } from './view';
  import type { Level0 } from './types';

  let { data, selected = null, onselect, onopen }: {
    data: Level0;
    selected?: string | null;
    onselect: (id: string | null) => void;
    onopen: (id: string) => void;
  } = $props();

  let hovered = $state<string | null>(null);
  const focus = $derived(hovered ?? selected);

  let width = $state(720);
  const size = $derived(Math.max(480, Math.min(width, 900)));
  const wheel = $derived(layoutWheel(data, size));
  const touches = (s: string, t: string) => !focus || s === focus || t === focus;
</script>

<div class="w-full overflow-auto" bind:clientWidth={width} data-testid="codemap-wheel">
  <svg width={size} height={size} role="img" aria-label="Modules on a ring, dependencies bundled by group">
    {#each wheel.arcs as a (a.group)}
      {@const mid = (a.start + a.end) / 2}
      <text
        x={wheel.center + (wheel.radius + 44) * Math.cos(mid)}
        y={wheel.center + (wheel.radius + 44) * Math.sin(mid)}
        text-anchor="middle"
        class="text-xs"
        fill="var(--ink-mute)">{a.group}</text>
    {/each}
    {#each wheel.bands as b (b.source + '>' + b.target)}
      <path d={b.path} fill="none" stroke={bandTone(b.direction, true)} stroke-width={b.width}
        stroke-opacity={touches(b.source, b.target) ? 0.6 : 0.05} data-testid={`wheel-band-${b.source}>${b.target}`}>
        <title>{b.source} → {b.target}: {b.weight}</title>
      </path>
    {/each}
    {#each [...wheel.nodes.values()] as n (n.id)}
      {@const folded = n.block.folded.length > 0}
      <g role="button" tabindex="0" class="cursor-pointer"
        onclick={() => (folded ? onselect(selected === n.id ? null : n.id) : onopen(n.id))}
        onmouseenter={() => (hovered = n.id)}
        onmouseleave={() => (hovered = null)}
        onkeydown={(e) => e.key === 'Enter' && (folded ? onselect(n.id) : onopen(n.id))}
        data-testid={`wheel-node-${n.id}`}>
        <circle cx={n.x} cy={n.y} r={selected === n.id ? 6 : 4}
          fill={selected === n.id ? 'var(--accent)' : 'var(--ink-soft)'} />
        <text
          x={n.x + 8 * Math.cos(n.angle)}
          y={n.y + 8 * Math.sin(n.angle)}
          text-anchor={Math.cos(n.angle) < 0 ? 'end' : 'start'}
          dominant-baseline="middle"
          class="text-xs"
          fill="var(--ink-soft)">{folded ? n.block.label : n.block.label.split('/').slice(-1)[0]}</text>
        <title>{n.block.id} — {n.block.symbols} symbols</title>
      </g>
    {/each}
  </svg>
</div>
