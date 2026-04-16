<script lang="ts">
  import { browser } from '$app/environment';

  const RELEASES = 'https://github.com/mizukisu/sensei-releases';
  const RELEASE_BASE = `${RELEASES}/releases/latest/download`;

  type Platform = { os: string; icon: string; file: string; label: string };

  const platforms: Platform[] = [
    { os: 'mac-arm',   icon: '🍎', file: 'sensei-cli-macos-arm64.tar.gz',   label: 'macOS Apple Silicon' },
    { os: 'mac-intel', icon: '🍎', file: 'sensei-cli-macos-x86_64.tar.gz',  label: 'macOS Intel' },
    { os: 'linux-x64', icon: '🐧', file: 'sensei-cli-linux-x86_64.tar.gz',  label: 'Linux x86_64' },
    { os: 'linux-arm', icon: '🐧', file: 'sensei-cli-linux-arm64.tar.gz',   label: 'Linux ARM64' },
    { os: 'windows',   icon: '🪟', file: 'sensei-cli-windows-x86_64.zip',   label: 'Windows x86_64' },
  ];

  function detectPlatform(): Platform {
    if (!browser) return platforms[0];
    const ua = navigator.userAgent.toLowerCase();
    const platform = (navigator as any).userAgentData?.platform?.toLowerCase() ?? navigator.platform?.toLowerCase() ?? '';

    if (ua.includes('win')) return platforms[4];
    if (ua.includes('linux')) {
      return ua.includes('aarch64') || ua.includes('arm') ? platforms[3] : platforms[2];
    }
    // macOS — check for Apple Silicon
    if (platform.includes('mac') || ua.includes('mac')) {
      // Safari and Chrome on Apple Silicon don't reliably expose arch in UA,
      // but arm64 Macs are the vast majority now — default to arm64
      return platforms[0];
    }
    return platforms[0];
  }

  let detected = $derived(detectPlatform());
</script>

<div class="mx-auto max-w-3xl px-8 py-16">

  <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Setup</div>
  <h1 class="mb-4 text-4xl font-extrabold tracking-tight">Install Sensei</h1>
  <p class="mb-12 text-lg text-surface-z5">Get the CLI and daemon running in under two minutes.</p>

  <!-- Step 1: Install -->
  <section class="mb-14">
    <div class="flex items-center gap-3 mb-5">
      <span class="flex items-center justify-center w-8 h-8 rounded-full bg-primary-z2 text-primary-z7 text-sm font-bold">1</span>
      <h2 class="text-xl font-bold">Install the binaries</h2>
    </div>

    <!-- Platform-detected download -->
    <div class="mb-5 rounded-xl border border-primary-z4 bg-primary-z1 p-5">
      <div class="flex items-center justify-between gap-4 flex-wrap">
        <div class="flex items-center gap-3">
          <span class="text-2xl">{detected.icon}</span>
          <div>
            <p class="text-sm font-semibold text-surface-z8">Download for {detected.label}</p>
            <p class="text-xs text-surface-z4 font-mono">{detected.file}</p>
          </div>
        </div>
        <a href="{RELEASE_BASE}/{detected.file}"
           class="rounded-lg bg-primary-z5 px-5 py-2.5 text-sm font-semibold text-white hover:bg-primary-z6 transition-colors whitespace-nowrap">
          Download
        </a>
      </div>
    </div>

    <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
      <!-- Homebrew -->
      <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
        <div class="flex items-center gap-2 mb-3">
          <span class="text-lg">🍺</span>
          <span class="font-semibold">Homebrew</span>
          <span class="ml-auto rounded bg-surface-z3 px-2 py-0.5 text-[10px] font-medium text-surface-z5">macOS / Linux</span>
        </div>
        <pre class="rounded-lg bg-surface-z0 p-3 text-xs text-surface-z7 overflow-x-auto"><code>brew install mizukisu/tap/sensei</code></pre>
        <p class="mt-2 text-xs text-surface-z4">Installs <code class="bg-surface-z3 px-1 rounded">sensei</code>, <code class="bg-surface-z3 px-1 rounded">senseid</code>, and <code class="bg-surface-z3 px-1 rounded">sensei-mcp</code>.</p>
      </div>

      <!-- All platforms -->
      <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
        <div class="flex items-center gap-2 mb-3">
          <span class="text-lg">📦</span>
          <span class="font-semibold">All platforms</span>
        </div>
        <div class="space-y-1.5 text-xs">
          {#each platforms as p}
            <a href="{RELEASE_BASE}/{p.file}"
               class="flex items-center gap-2 {p.os === detected.os ? 'text-primary-z6 font-semibold' : 'text-surface-z5'} hover:text-primary-z6 transition-colors">
              <span class="text-sm">{p.icon}</span>
              <span>{p.label}</span>
              {#if p.os === detected.os}
                <span class="ml-auto rounded bg-primary-z2 px-1.5 py-0.5 text-[9px] font-bold text-primary-z7">detected</span>
              {/if}
            </a>
          {/each}
        </div>
      </div>
    </div>
  </section>

  <!-- Step 2: Configure -->
  <section class="mb-14">
    <div class="flex items-center gap-3 mb-5">
      <span class="flex items-center justify-center w-8 h-8 rounded-full bg-primary-z2 text-primary-z7 text-sm font-bold">2</span>
      <h2 class="text-xl font-bold">Configure your AI coding platform</h2>
    </div>
    <p class="mb-4 text-sm text-surface-z5">Sensei auto-detects installed platforms and installs MCP servers, hooks, and skills for each one.</p>
    <pre class="rounded-lg bg-surface-z0 p-4 text-xs text-surface-z7 overflow-x-auto"><code># Auto-detect and configure all platforms
sensei configure
sensei install

# Or target a specific platform
sensei install --acp claude-code</code></pre>
    <p class="mt-3 text-xs text-surface-z4">Supported platforms: Claude Code, Cursor, Windsurf, and more.</p>
  </section>

  <!-- Step 3: Start -->
  <section class="mb-14">
    <div class="flex items-center gap-3 mb-5">
      <span class="flex items-center justify-center w-8 h-8 rounded-full bg-primary-z2 text-primary-z7 text-sm font-bold">3</span>
      <h2 class="text-xl font-bold">Start the daemon and scan</h2>
    </div>
    <pre class="rounded-lg bg-surface-z0 p-4 text-xs text-surface-z7 overflow-x-auto"><code># Start the indexer daemon
sensei start

# Scan your projects folder
sensei scan ~/Developer

# Check status
sensei status</code></pre>
    <p class="mt-3 text-sm text-surface-z5">The daemon indexes your repos in the background and serves a local API on port 7744. Your AI coding platform connects via the MCP server automatically.</p>
  </section>

  <!-- Step 4: Verify -->
  <section class="mb-14">
    <div class="flex items-center gap-3 mb-5">
      <span class="flex items-center justify-center w-8 h-8 rounded-full bg-primary-z2 text-primary-z7 text-sm font-bold">4</span>
      <h2 class="text-xl font-bold">Verify it works</h2>
    </div>
    <pre class="rounded-lg bg-surface-z0 p-4 text-xs text-surface-z7 overflow-x-auto"><code># Check the daemon is running
sensei status

# Open Claude Code in any indexed repo — sensei tools are available
claude</code></pre>
    <p class="mt-3 text-sm text-surface-z5">In Claude Code, you should see sensei MCP tools like <code class="bg-surface-z3 px-1 rounded">search</code>, <code class="bg-surface-z3 px-1 rounded">context_pack</code>, and <code class="bg-surface-z3 px-1 rounded">get_session_context</code>.</p>
  </section>

  <!-- Uninstall -->
  <section class="border-t border-surface-z0 pt-10">
    <h2 class="mb-3 text-lg font-bold">Uninstalling</h2>
    <pre class="rounded-lg bg-surface-z0 p-4 text-xs text-surface-z7 overflow-x-auto"><code># Remove hooks, skills, and MCP config from all platforms
sensei stop
sensei uninstall

# Then remove the binaries
brew uninstall sensei   # if installed via Homebrew</code></pre>
  </section>

</div>
