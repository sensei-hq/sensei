import { defineConfig } from "bumpp";
import { readFileSync, writeFileSync } from "fs";

export default defineConfig({
  files: [
    "package.json",
    "apps/*/package.json",
    // Rust crates
    "crates/senseid/Cargo.toml",
    "crates/sensei-mcp/Cargo.toml",
    "crates/sensei-cli/Cargo.toml",
    // Homebrew (subtree)
    "homebrew/Formula/sensei.rb",
    "homebrew/Casks/sensei-app.rb",
    // Marketplace (subtree)
    "marketplace/package.json",
    "marketplace/catalog.json",
    // Plugin manifests (updated via execute — bumpp's text replace
    // would match the version but miss nested structures)
    "marketplace/plugins/sensei/.claude-plugin/plugin.json",
    "marketplace/.claude-plugin/marketplace.json",
  ],
  recursive: true,
});
