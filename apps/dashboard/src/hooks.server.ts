import { kavach } from '$kavach/auth';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = ({ event, resolve }) => kavach.handle({ event, resolve });
