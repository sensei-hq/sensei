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
		// Homebrew (subtree at homebrew/)
		'homebrew/Formula/sensei.rb',
		'homebrew/Casks/sensei-app.rb',
		// Marketplace (subtree at marketplace/)
		'marketplace/package.json',
		'marketplace/catalog.json',
	],
	recursive: true,
})
