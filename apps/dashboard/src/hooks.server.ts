import type { Handle } from '@sveltejs/kit';

// Auth disabled — restore when kavach default export / handle pattern is resolved
export const handle: Handle = ({ event, resolve }) => resolve(event);
