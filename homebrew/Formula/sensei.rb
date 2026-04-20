# Homebrew formula for the Sensei CLI.
# Usage:
#   brew tap mizukisu/tap
#   brew install mizukisu/tap/sensei

class Sensei < Formula
  GITHUB_ORG = "mizukisu".freeze
  REPO_NAME = "sensei-releases".freeze
  REPO_URL = "https://github.com/#{GITHUB_ORG}/#{REPO_NAME}".freeze

  desc "AI development intelligence — CLI for indexing, MCP server, and telemetry"
  homepage "https://github.com/#{GITHUB_ORG}/sensei-dev"
  version "0.2.1"

  # Release archives (produced by `bun run build` + packaging)
  if OS.mac? && Hardware::CPU.arm?
    url "#{REPO_URL}/releases/download/v#{version}/sensei-cli-macos-arm64.tar.gz"
    sha256 "c87a6bfdcae2d4868b56fd19a050257b64e0be6d65e959fee79da0437737a1e3"
  elsif OS.mac? && Hardware::CPU.intel?
    url "#{REPO_URL}/releases/download/v#{version}/sensei-cli-macos-x86_64.tar.gz"
    sha256 "3ad4c6565a6bb0e0d3db9c3c2161116c818555e378f4671001abcdeb0603910d"
  elsif OS.linux? && Hardware::CPU.arm?
    url "#{REPO_URL}/releases/download/v#{version}/sensei-cli-linux-arm64.tar.gz"
    sha256 "620db1834c0b160b1d8a49b1be1480d00689c94b5187d6ac51bcd2b602e1c54d"
  else
    url "#{REPO_URL}/releases/download/v#{version}/sensei-cli-linux-x86_64.tar.gz"
    sha256 "ca9395ad6bce190cc74030cdc21dfec56c4b02209f6fabda594d62a9c9a1c91e"
  end

  bottle :unneeded

  def install
    bin.install "sensei"
    bin.install "senseid"
    bin.install "sensei-mcp"

    # Install marketplace alongside binaries for plugin install support
    share.install "marketplace" if File.directory?("marketplace")
  end

  def post_install
    (Pathname.new(ENV["HOME"]) / ".sensei").mkpath
  end

  service do
    run [opt_bin/"senseid", "start", "--port", "7744"]
    keep_alive true
    log_path var/"log/sensei.log"
    error_log_path var/"log/sensei.error.log"
    working_dir ENV["HOME"]
    environment_variables HOME: ENV["HOME"], PATH: "#{HOMEBREW_PREFIX}/bin:/usr/local/bin:/usr/bin:/bin"
  end

  def caveats
    <<~EOS
      1. Start the daemon:

        brew services start sensei

      2. Configure your AI coding platform:

        sensei install --acp claude-code   # installs Claude Code plugin (agents, hooks, MCP)
        sensei install --acp cursor        # or cursor, windsurf, zed, kiro, opencode

      3. Initialize a repo:

        cd ~/your-project
        sensei init                        # sets up mindsets, personas, rules

      4. Scan and index your repos:

        sensei scan ~/Developer

      Desktop app available separately:

        brew install --cask mizukisu/tap/sensei-app

      Before uninstalling:

        brew services stop sensei
        sensei uninstall
    EOS
  end

  test do
    system "#{bin}/sensei", "--version"
    system "#{bin}/senseid", "--help"
    system "#{bin}/sensei-mcp", "--help"
  end
end
