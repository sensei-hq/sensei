import { defineConfig } from 'bumpp'

export default defineConfig({
	files: [
		// JS packages
		'package.json',
		'packages/*/package.json',
		'apps/*/package.json',
		// Rust crates
		'crates/senseid/Cargo.toml',
		'crates/sensei-mcp/Cargo.toml',
		'crates/sensei-cli/Cargo.toml',
		// Homebrew (sibling repo)
		'../sensei-homebrew/Formula/sensei.rb',
		'../sensei-homebrew/Casks/sensei-app.rb',
		// Marketplace (sibling repo)
		'../sensei-marketplace/package.json',
		'../sensei-marketplace/catalog.json',
	],
	recursive: true,
})
