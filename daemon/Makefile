## Sensei daemon — build targets
##
## Binaries:
##   senseid   — HTTP daemon (API server)
##   sensei    — CLI (sensei-cli crate, binary: sensei)
##   sensei-mcp — MCP server
##
## Dev builds connect to PostgreSQL database: sensei_dev
## Release builds connect to:               sensei

.PHONY: dev release dev-daemon dev-cli dev-mcp \
        release-daemon release-cli release-mcp \
        install-dev install-release clean test

# ── Dev (debug) ───────────────────────────────────────────────────────────────

dev: dev-daemon dev-cli dev-mcp
	@echo "Dev binaries ready in target/debug/"

dev-daemon:
	cargo build -p senseid

dev-cli:
	cargo build -p sensei-cli

dev-mcp:
	cargo build -p sensei-mcp

# ── Release ───────────────────────────────────────────────────────────────────

release: release-daemon release-cli release-mcp
	@echo "Release binaries ready in target/release/"

release-daemon:
	cargo build --release -p senseid

release-cli:
	cargo build --release -p sensei-cli

release-mcp:
	cargo build --release -p sensei-mcp

# ── Install ───────────────────────────────────────────────────────────────────

# Copy dev binaries to ~/.local/bin (ahead of Homebrew in PATH)
install-dev: dev
	@mkdir -p ~/.local/bin
	cp target/debug/senseid   ~/.local/bin/senseid
	cp target/debug/sensei    ~/.local/bin/sensei
	cp target/debug/sensei-mcp ~/.local/bin/sensei-mcp
	@echo "Installed dev binaries to ~/.local/bin"
	@echo "Make sure ~/.local/bin is before /opt/homebrew/bin in PATH"

# Copy release binaries to ~/.local/bin
install-release: release
	@mkdir -p ~/.local/bin
	cp target/release/senseid   ~/.local/bin/senseid
	cp target/release/sensei    ~/.local/bin/sensei
	cp target/release/sensei-mcp ~/.local/bin/sensei-mcp
	@echo "Installed release binaries to ~/.local/bin"

# ── Other ─────────────────────────────────────────────────────────────────────

test:
	cargo test --workspace

clean:
	cargo clean
