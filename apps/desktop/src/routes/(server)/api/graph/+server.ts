export const prerender = true;
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = () =>
  json({ summary: null, projects: [], communities: [], godNodes: [], rationale: [] });
