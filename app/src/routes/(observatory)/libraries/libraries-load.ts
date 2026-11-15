// The Libraries list read, shared by the route's loader (+page.ts) and the
// screen's state (Retry re-reads through it, so Retry touches only /api/libs).
// A list that could not be read is the error it was, never an empty index
// (#288) — and it asks for every library, never `shared: true` only (#63:
// `shared` hid single-repo libraries, which is why most @rokkit/* packages
// never appeared).
import { failureReason, type ApiResult } from '$lib/api.js';
import type { LibEntry } from '$lib/types.js';
import type { LibrariesApi } from './libraries-state.svelte.js';

/** What the screen shows for the list: the libraries, or why they could not be read. */
export interface LibrariesRead {
  libs: LibEntry[];
  error: string | null;
}

export async function readLibraries(
  api: Pick<LibrariesApi, 'tryGetLibs'>,
): Promise<LibrariesRead> {
  const read: ApiResult<{ total: number; libs: LibEntry[] }> = await api.tryGetLibs({});
  return read.ok
    ? { libs: read.data.libs, error: null }
    : { libs: [], error: failureReason(read.error) };
}
