import { defineConfig } from 'bumpp'
import { readFileSync, writeFileSync } from 'fs'

function updateCargoVersions(newVersion) {
	const crates = [
		'crates/senseid/Cargo.toml',
		'crates/sensei-mcp/Cargo.toml',
		'crates/sensei-cli/Cargo.toml',
	]
	for (const path of crates) {
		const content = readFileSync(path, 'utf8')
		const updated = content.replace(
			/^(version\s*=\s*")[^"]*(")/m,
			`$1${newVersion}$2`
		)
		writeFileSync(path, updated)
	}
}

function updateJsonVersion(path, newVersion, nested) {
	const data = JSON.parse(readFileSync(path, 'utf8'))
	if (nested) {
		for (const plugin of data.plugins) {
			if (plugin.name === 'sensei') plugin.version = newVersion
		}
	} else {
		data.version = newVersion
	}
	writeFileSync(path, JSON.stringify(data, null, 2) + '\n')
}

export default defineConfig({
	files: [
		'package.json',
		'apps/*/package.json',
		// Rust crates (updated via execute, listed here so bumpp commits them)
		'crates/senseid/Cargo.toml',
		'crates/sensei-mcp/Cargo.toml',
		'crates/sensei-cli/Cargo.toml',
		// Homebrew (subtree)
		'homebrew/Formula/sensei.rb',
		'homebrew/Casks/sensei-app.rb',
		// Marketplace (subtree)
		'marketplace/package.json',
		'marketplace/catalog.json',
		'marketplace/plugins/sensei/.claude-plugin/plugin.json',
		'marketplace/.claude-plugin/marketplace.json',
	],
	recursive: true,
	execute: (operation) => {
		const v = operation.state.newVersion
		updateCargoVersions(v)
		updateJsonVersion('marketplace/plugins/sensei/.claude-plugin/plugin.json', v)
		updateJsonVersion('marketplace/.claude-plugin/marketplace.json', v, true)
	},
})
