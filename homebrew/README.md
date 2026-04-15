# homebrew-tap

Homebrew tap for [sensei](https://github.com/mizukisu/sensei) — AI development intelligence for coding agents.

## Install

```sh
brew tap mizukisu/tap
brew install mizukisu/tap/sensei
```

This installs two binaries:

| Binary | Purpose |
|--------|---------|
| `sensei` | Interactive CLI — `sensei init`, `sensei index`, `sensei skills`, etc. |
| `senseid` | Background daemon — MCP server + indexing queue |

## Run as a background service

```sh
brew services start mizukisu/tap/sensei
```

`senseid` starts automatically on login, runs the MCP server on `localhost:7320`, and indexes your repos in the background (up to 4 parallel workers).

## Install the desktop app

```sh
brew install --cask mizukisu/tap/sensei-app
```

## Install everything (CLI + app + Ollama)

Save this as a `Brewfile` and run `brew bundle`:

```sh
tap "mizukisu/tap"
brew "mizukisu/tap/sensei"
cask "mizukisu/tap/sensei-app"
brew "ollama"
```

Or use the included Brewfile in this repo:

```sh
brew bundle --file=Brewfile
```

## Formulas

| Formula | Description |
|---------|-------------|
| [`Formula/sensei.rb`](Formula/sensei.rb) | CLI + daemon binaries |
| [`Casks/sensei-app.rb`](Casks/sensei-app.rb) | macOS desktop app |

## Updating

```sh
brew update
brew upgrade sensei
```

## Uninstall

```sh
brew uninstall sensei
brew untap mizukisu/tap
```

To also remove all app data:

```sh
rm -rf ~/.sensei
```
