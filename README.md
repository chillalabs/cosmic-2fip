# 2fip

A keyboard-driven, dual-pane file manager for the [COSMIC](https://system76.com/cosmic)
desktop, inspired by Total Commander. Written in Rust with
[libcosmic](https://github.com/pop-os/libcosmic).

*2fip* — **2** **Fi**le **P**anel: two file panels, side by side.

## Features

- **Two panels with tabs**: each tab has its own folder, history, sort order and
  list/grid view; tabs, folders and the active panel are restored on the next start.
- **Classic function keys**: F2 rename, F3 view, F4 edit, F5 copy, F6 move,
  F7 new folder, F8 delete (to the trash), F9 terminal.
- **Copy, move, delete and compress in the background**, with a progress panel,
  cancel, and a "file already exists" prompt (replace / skip, for one or all).
- **List view** with Name, Ext, Size and Modified columns (the extension can also
  stay part of the name), or a **grid view** with large icons.
- **Thumbnails** of images, PDFs, videos and fonts, shared with COSMIC Files
  through the freedesktop thumbnail cache.
- **Quick filter** (Ctrl+S), editable **path bar** with clickable folders,
  **favorites**, **folder sizes** on demand (Space), Open With, file details.
- **Languages**: English, German, French, Italian, Portuguese (Brazil and
  Portugal), Spanish (Spain and Latin America).
- **Color themes**: follow the desktop, or pick Light, Dark, Dracula, Nord,
  Catppuccin, Gruvbox, Tokyo Night, Solarized and more.
- **Drag and drop** between the panels and other apps.

## Keyboard

| Key | Action |
|---|---|
| Tab | Switch panel |
| ↑ / ↓, Home / End | Move the selection |
| Shift + ↑ / ↓ / Home / End | Extend the selection |
| Enter | Open the selected file or folder |
| → | Enter the selected folder |
| Backspace, ← | Parent folder |
| Space | Calculate the size of the selected folders |
| F2 … F9 | Rename, View, Edit, Copy, Move, New folder, Delete, Terminal |
| Ctrl+C / Ctrl+X / Ctrl+V | Copy / cut / paste |
| Ctrl+A | Select all |
| Ctrl+S | Quick filter |
| Ctrl+L | Type a path |
| Ctrl+D | Favorites |
| Ctrl+T / Ctrl+W, Ctrl+Tab | New / close tab, next tab |
| Ctrl+1 / Ctrl+2 | List / grid view |
| Ctrl+H | Show / hide hidden files |
| Alt+Enter | File details |
| Ctrl+, | Settings |
| Ctrl+Q, Alt+F4 | Quit |

## Build and install

Requirements: Rust (stable), [just](https://github.com/casey/just), and the
usual COSMIC/libcosmic build dependencies (e.g. `libxkbcommon-dev`).

```sh
just run        # build and run a development version
just test       # run the tests
just install    # install to ~/.local (binary, launcher entry, icon)
just uninstall  # remove it again (settings in ~/.config/2fip are kept)
just package    # dist/2fip-<version>-<arch>.tar.gz for another computer
```

To install a package on another computer, unpack it and run `./install.sh`.
The release binary needs glibc 2.39 or newer (e.g. Pop!_OS / Ubuntu 24.04).

## Project layout

- `app/` — the COSMIC application (UI, panels, dialogs, translations in `app/i18n/`)
- `core/` — `fs-ops`, the file operations engine (listing, copy/move/delete,
  compression, thumbnails, settings and session storage)
- `res/` — icon, desktop entry and install script

## License

[GPL-3.0](LICENSE)
