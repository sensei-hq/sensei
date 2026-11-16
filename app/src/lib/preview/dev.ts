// Everything the preview draws, in one module the routes import only in development:
// the fixture catalogue and the two pages. A production build drops the whole chunk.
export { RELAY_CATALOGUE as CATALOGUE } from '$lib/relay/catalogue.js';
export { default as PreviewIndex } from './PreviewIndex.svelte';
export { default as PreviewComponent } from './PreviewComponent.svelte';
