// The layering payload the diagram specs share: `sensei`'s own shape, shrunk.
import type { LayeringPayload } from '$lib/types.js';

/** `sensei`'s own shape, shrunk: `db` and `tasks` close a cycle, `tasks`
 *  depends on `util` below it, and `db → tasks` is the weakest link. */
export function payload(over: Partial<LayeringPayload> = {}): LayeringPayload {
  return {
    level: 'module',
    kinds: ['calls'],
    layerSource: 'derived',
    depth: 2,
    nodes: [
      {
        id: 'senseid/db',
        label: 'db',
        group: 'senseid',
        weight: 40,
        files: 9,
        language: 'rust',
        layer: 0,
        component: 0,
        componentSize: 2,
      },
      {
        id: 'senseid/tasks',
        label: 'tasks',
        group: 'senseid',
        weight: 30,
        files: 7,
        language: 'rust',
        layer: 0,
        component: 0,
        componentSize: 2,
      },
      {
        id: 'senseid/util',
        label: 'util',
        group: 'senseid',
        weight: 5,
        files: 1,
        language: 'rust',
        layer: 1,
        component: 1,
        componentSize: 1,
      },
    ],
    edges: [
      {
        id: 'senseid/db→senseid/tasks',
        source: 'senseid/db',
        target: 'senseid/tasks',
        kind: 'dependency',
        weight: 149,
        conformance: 'up',
        weakest: true,
      },
      {
        id: 'senseid/tasks→senseid/db',
        source: 'senseid/tasks',
        target: 'senseid/db',
        kind: 'dependency',
        weight: 200,
        conformance: 'level',
        weakest: false,
      },
      {
        id: 'senseid/tasks→senseid/util',
        source: 'senseid/tasks',
        target: 'senseid/util',
        kind: 'dependency',
        weight: 12,
        conformance: 'down',
        weakest: false,
      },
    ],
    cycles: [
      {
        component: 0,
        members: ['senseid/db', 'senseid/tasks'],
        layer: 0,
        inner: [
          { source: 'senseid/tasks', target: 'senseid/db', occurrences: 200 },
          { source: 'senseid/db', target: 'senseid/tasks', occurrences: 149 },
        ],
        cut: { source: 'senseid/db', target: 'senseid/tasks', occurrences: 149 },
      },
    ],
    selfDependencies: [{ source: 'senseid/db', target: 'senseid/db', occurrences: 4 }],
    coverage: { drawn: 3, unplaced: 41, units: 3, unknownUnit: 32 },
    ...over,
  };
}
