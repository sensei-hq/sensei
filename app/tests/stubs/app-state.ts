// Vitest stub for SvelteKit's generated `$app/state` (see vitest.config.ts).
// A component under test reads `page.params` / `page.url`; a spec that needs
// particular values sets them on this object before mounting.
export const page = {
  params: {} as Record<string, string>,
  url: new URL('http://localhost/'),
  route: { id: null as string | null },
  status: 200,
  error: null as unknown,
  data: {} as Record<string, unknown>,
  form: null as unknown,
  state: {} as Record<string, unknown>,
};
export const navigating = { from: null, to: null, type: null, willUnload: null, delta: null, complete: null };
export const updated = { current: false, check: async () => false };
