
# Ghostty Zed Extension

Zed extension that adds syntax highlighting for Ghostty's configuration file.

## Features

- Syntax highlighting for Ghostty configuration files using the [`tree-sitter-ghostty`](https://github.com/bezhermoso/tree-sitter-ghostty) grammar.
- ⚠️ Tries to automatically apply to files whose path ends in `ghostty/config`, for example:
  - `~/.config/ghostty/config`
  - `~/Library/Application Support/com.mitchellh.ghostty/config`  
  In some cases Zed may open the file with language `Unknown`; if that happens, use the language selector in the bottom-right and switch it to `Ghostty` manually.

## How it works

This extension defines a `Ghostty` language that:

- Uses the `tree-sitter-ghostty` grammar to parse Ghostty configuration files.
- Registers `ghostty/config` as a `path_suffix`, so any file whose path ends with `ghostty/config` is treated as a Ghostty config file.
- Provides Tree-sitter highlight queries that map Ghostty keys, values, comments and keybindings to Zed scopes.

The result is proper syntax highlighting for your Ghostty config without having to rename the file or add an artificial extension.

## Editing the Ghostty config in Zed

Ghostty already has a keyboard shortcut to open its configuration file:

- macOS: `Cmd+,`
- Linux: `Ctrl+,`

That shortcut triggers the `open_config` action, which opens the config file in the default OS editor for plain text. Because the Ghostty config file is called simply `config` and has no file extension, it is treated as generic plain text and is not easy to re-associate from Finder's “Open With” dialog on macOS.

The sections below describe how to point that shortcut at Zed.

### macOS: make `Cmd+,` open the config in Zed

On macOS, `open_config` uses the `open -t` command under the hood, so the file is opened in the default application for plain text documents. That default is defined at the system level for the `public.plain-text` UTI.

To make `Cmd+,` open Ghostty's config in Zed, you can change the default handler for `public.plain-text` using [`duti`](https://github.com/moretension/duti).

1. Install `duti`:

```bash
brew install duti
```

2. Make sure Zed is installed (for example in `/Applications`) and that its bundle identifier is `dev.zed.Zed` (this is the default for the official builds).

3. Tell macOS to use Zed for all plain text files:

```bash
duti -s dev.zed.Zed public.plain-text all
```

4. Restart Ghostty if it was running, then press `Cmd+,` inside Ghostty. The config file should now open in Zed with Ghostty syntax highlighting from this extension.

> Note
> This changes the default editor for **all** plain text files on your system, not only the Ghostty config. If you later want to use another app as your default editor, you can run `duti` again with that app's bundle identifier instead of `dev.zed.Zed`.

### Linux (not verified)

On Linux, `open_config` calls `xdg-open`, which uses your desktop environment's default application for the `text/plain` MIME type. In principle you can point that at Zed as well.

The steps below are a starting point and **have not been verified**. Behaviour can differ between distributions and desktop environments.

1. Locate Zed's desktop file, usually named `dev.zed.Zed.desktop`, under one of:

   * `~/.local/share/applications/`
   * `/usr/share/applications/`

2. Set Zed as the default handler for plain text:

```bash
xdg-mime default dev.zed.Zed.desktop text/plain
```

3. Log out and back in if necessary so your desktop environment picks up the new association.

4. Open Ghostty and press `Ctrl+,`. If everything is wired correctly, the Ghostty config file should open in Zed and use this extension's Ghostty syntax highlighting.

Because Linux desktop integration varies a lot, you might need to adapt these commands for your particular distribution or desktop environment. If something behaves differently, contributions to this section are welcome.
