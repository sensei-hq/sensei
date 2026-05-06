# Homebrew cask for the Sensei desktop app (.app bundle).
# Depends on the sensei formula for CLI tools (senseid, sensei-mcp).
#
# Usage:
#   brew tap sensei-hq/tap
#   brew install --cask sensei-hq/tap/sensei

cask "sensei" do
  version "0.1.0"

  if Hardware::CPU.arm?
    url "https://github.com/sensei-hq/sensei/releases/download/v#{version}/Sensei_aarch64.dmg"
    sha256 "REPLACE_WITH_ARM64_DMG_SHA256"
  else
    url "https://github.com/sensei-hq/sensei/releases/download/v#{version}/Sensei_x86_64.dmg"
    sha256 "REPLACE_WITH_X86_64_DMG_SHA256"
  end

  name "Sensei"
  desc "AI development intelligence desktop app"
  homepage "https://github.com/sensei-hq/sensei"

  depends_on formula: "sensei"

  app "Sensei.app"

  zap trash: [
    "~/.sensei",
    "~/Library/Application Support/dev.sensei.desktop",
    "~/Library/Caches/dev.sensei.desktop",
    "~/Library/Logs/dev.sensei.desktop",
    "~/Library/Preferences/dev.sensei.desktop.plist",
    "~/Library/Saved Application State/dev.sensei.desktop.savedState",
  ]
end
