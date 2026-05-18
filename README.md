
# Ghostty Zed Extension

Zed extension that adds syntax highlighting for Ghostty's configuration file.

## Features

- Syntax highlighting for Ghostty configuration files using the [`tree-sitter-ghostty`](https://github.com/bezhermoso/tree-sitter-ghostty) grammar.
- Auto-completion for configuration keys and values (boolean, enum, colour, keybind, theme, etc.)
- Tries to automatically apply to files whose path ends in `ghostty/config`, `com.mitchellh.ghostty/config`, or `config/ghostty/config`

### File detection workaround

If Zed detects your Ghostty config as a different language (e.g. INI), add this to your Zed `settings.json`:

```json
{
  "file_types": {
    "Ghostty": [
      "**/ghostty/config",
      "**/ghostty/themes/*",
      "**/com.mitchellh.ghostty/config"
    ]
  }
}
```

Alternatively, use the language selector in the bottom-right corner to manually switch to `Ghostty`.

## How it works

This extension defines a `Ghostty` language that:

- Uses the `tree-sitter-ghostty` grammar to parse Ghostty configuration files.
- Registers `ghostty/config` and `.ghostty` with `path_suffix`, so any file whose path ends with those are treated as a Ghostty config file.
- Provides Tree-sitter highlight queries that map Ghostty keys, values, comments and keybindings to Zed scopes.

The result is proper syntax highlighting for your Ghostty config without having to rename the file or add an artificial extension.

## Editing the Ghostty config in Zed

Ghostty already has a keyboard shortcut to open its configuration file:

- macOS: `Cmd+,`
- Linux: `Ctrl+,`

By default this shortcut triggers the `open_config` action, which opens the config file in the default OS editor for plain text, if you instead want to open it with Zed you can set `$EDITOR` to the path to zed (e.g. `/usr/local/bin/zed`) in your environment and can set a keybinding:

```ini
# Edit ghostty config with $EDITOR
keybind = super+,=text:ghostty +edit-config\n
```

## Development

### Building the LSP server

The extension includes a language server (`ghostty-lsp`) that provides auto-completion. To build it locally:

```bash
cargo build --release -p ghostty-lsp
```

The binary will be at `target/release/ghostty-lsp`.

### Testing locally

To test the extension with a local LSP binary (without requiring a GitHub release):

1. Build the LSP server as above
2. Set the `GHOSTTY_LSP_PATH` environment variable in your shell:

```bash
export GHOSTTY_LSP_PATH="$HOME/path/to/ghostty-zed-extension/target/release/ghostty-lsp"
```

3. Launch Zed from that shell (so it inherits the environment variable)
4. Install the extension as a dev extension in Zed (Extensions → Install Dev Extension)

When `GHOSTTY_LSP_PATH` is set, the extension uses that binary instead of downloading from GitHub releases.
