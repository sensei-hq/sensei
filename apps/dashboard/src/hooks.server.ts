import type { Handle } from '@sveltejs/kit';
import kavach from 'kavach';

export const handle: Handle = ({ event, resolve }) => kavach.handle({ event, resolve });
