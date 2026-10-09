---
layout: default
title: Installation
---

2fip runs on Linux. It is made for the COSMIC desktop (Pop!_OS 24.04 and
later, Fedora COSMIC, and others) but works on other Wayland desktops too.

## Flatpak (recommended)

The Flatpak works on any distribution and updates itself with the rest of
your Flatpak apps:

```sh
flatpak install --user https://chillalabs.github.io/flatpak/2fip.flatpakref
```

Or open [chillalabs.github.io/flatpak](https://chillalabs.github.io/flatpak/)
and click **Install with Flatpak**. The runtime it needs is installed from
Flathub automatically.

To update: `flatpak update`. To remove: `flatpak uninstall io.github.chillalabs.TwoFip`.

> The Flatpak runs your terminal, other apps and thumbnailers on the system
> (outside the sandbox), so it behaves like a regular install.

## AppImage

Download `2fip-<version>-x86_64.AppImage` from the
[latest release](https://github.com/chillalabs/cosmic-2fip/releases/latest),
make it executable and run it:

```sh
chmod +x 2fip-*-x86_64.AppImage
./2fip-*-x86_64.AppImage
```

It needs glibc 2.39 or newer (Ubuntu / Pop!_OS 24.04, Fedora 40, or later)
and doesn't update itself.

## Package for your user

The `2fip-<version>-x86_64.tar.gz` file on the releases page installs 2fip
into `~/.local` (the program, its launcher and icon):

```sh
tar xzf 2fip-*-x86_64.tar.gz
cd 2fip-*-x86_64
./install.sh
```

Remove it with `./install.sh --uninstall`. Your settings are kept.

## Build from source

You need Rust (stable), [just](https://github.com/casey/just) and the usual
COSMIC build packages (for example `libxkbcommon-dev`):

```sh
git clone https://github.com/chillalabs/cosmic-2fip
cd cosmic-2fip
just run        # build and start
just install    # install into ~/.local
```

## Where 2fip keeps its data

| What | Where |
|---|---|
| Settings, favorites, saved connections, open tabs | `~/.config/2fip/` |
| Passwords you chose to remember | the system keyring (GNOME Keyring, KWallet) |
| Thumbnails (shared with other file managers) | `~/.cache/thumbnails/` |
| Local copies of files opened from servers | `~/.cache/2fip/remote/` |
