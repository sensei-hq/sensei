# homebrew-tap

Homebrew tap for [sensei](https://github.com/sensei-dev/sensei) — AI development intelligence for coding agents.

## Install

```sh
brew tap sensei-dev/tap
brew install sensei-dev/tap/sensei
```

This installs two binaries:

| Binary | Purpose |
|--------|---------|
| `sensei` | Interactive CLI — `sensei init`, `sensei index`, `sensei skills`, etc. |
| `senseid` | Background daemon — MCP server + indexing queue |

## Run as a background service

```sh
brew services start sensei-dev/tap/sensei
```

`senseid` starts automatically on login, runs the MCP server on `localhost:7320`, and indexes your repos in the background (up to 4 parallel workers).

## Install the desktop app

```sh
brew install --cask sensei-dev/tap/sensei-app
```

## Install everything (CLI + app + Ollama)

Save this as a `Brewfile` and run `brew bundle`:

```sh
tap "sensei-dev/tap"
brew "sensei-dev/tap/sensei"
cask "sensei-dev/tap/sensei-app"
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
brew untap sensei-dev/tap
```

To also remove all app data:

```sh
rm -rf ~/.sensei
```
