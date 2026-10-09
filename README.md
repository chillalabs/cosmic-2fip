# 2fip

A keyboard-driven, dual-pane file manager for the [COSMIC](https://system76.com/cosmic)
desktop, inspired by Total Commander and COSMIC Files. Written in Rust with
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
- **Icon styles:** your icon theme, or 2fip's own Vivid, Windows style and
  macOS style sets, or monochrome.
- **Help:** press F1 for the [user's guide](https://chillalabs.github.io/twofip/).
- **Servers** (Ctrl+K, File → Connect to server): browse **SFTP**, **FTP** and
  **FTPS** folders in a panel and copy, move, rename and delete like local ones,
  with progress. 2fip connects by itself (no system mounts), so it works the
  same on any distribution and in the Flatpak/AppImage. SFTP logs in with your
  SSH agent or `~/.ssh` keys (or a password) and checks `~/.ssh/known_hosts`.
- **Find files** (Ctrl+F) by name or pattern (`*.pdf`) in the active folder and
  its subfolders; jump to a result with Enter or a double-click.

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
| Shift+Delete, Shift+F8 | Delete permanently (without the trash), after a confirmation |
| Ctrl+C / Ctrl+X / Ctrl+V | Copy / cut / paste |
| Ctrl+A | Select all |
| Ctrl+S | Quick filter |
| Ctrl+L | Type a path |
| Ctrl+D | Favorites |
| Ctrl+T / Ctrl+W, Ctrl+Tab | New / close tab, next tab |
| Ctrl+1 / Ctrl+2 | List / grid view |
| Ctrl+H | Show / hide hidden files |
| Ctrl+R | Refresh both panels |
| Ctrl+F | Find files in the active folder and its subfolders |
| Ctrl+K | Connect to a server (SFTP, FTP, FTPS) |
| F1 | Help (the online user's guide) |
| Alt+Enter | File details |
| Ctrl+, | Settings |
| Ctrl+Q, Alt+F4 | Quit |

## Install with Flatpak

```sh
flatpak install --user https://chillalabs.github.io/flatpak/2fip.flatpakref
```

Updates come with `flatpak update`. More at <https://chillalabs.github.io/flatpak/>.
The Flatpak runs the terminal, other apps and thumbnailers on the host system
(with `flatpak-spawn --host`), so it works like a regular install.

## AppImage

Download `2fip-<version>-x86_64.AppImage` from the
[latest release](https://github.com/chillalabs/twofip/releases/latest),
make it executable and run it:

```sh
chmod +x 2fip-*-x86_64.AppImage
./2fip-*-x86_64.AppImage
```

It needs glibc 2.39 or newer (e.g. Pop!_OS / Ubuntu 24.04) and doesn't update
itself; the Flatpak does.

## Build and install

Requirements: Rust (stable), [just](https://github.com/casey/just), and the
usual COSMIC/libcosmic build dependencies (e.g. `libxkbcommon-dev`).

```sh
just run        # build and run a development version
just test       # run the tests
just install    # install to ~/.local (binary, launcher entry, icon)
just uninstall  # remove it again (settings in ~/.config/2fip are kept)
just package    # dist/2fip-<version>-<arch>.tar.gz for another computer
just flatpak    # build and install the Flatpak locally
just appimage   # dist/2fip-<version>-x86_64.AppImage
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
