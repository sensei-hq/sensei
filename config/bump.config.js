import { defineConfig } from 'bumpp'

export default defineConfig({
	files: ['package.json', 'packages/*/package.json', 'apps/*/package.json'],
	recursive: true
})
