---
layout: default
title: Questions and problems
---

### Nothing happens when I start 2fip

2fip allows **one window** at a time. If it's already open (maybe on another
workspace or screen), starting it again brings that window to the front. This
also counts the Flatpak, the AppImage and a regular install: only one of them
runs at a time.

### F9 doesn't open a terminal

2fip uses the terminal chosen in **COSMIC Settings**, then `$TERMINAL`, then
`x-terminal-emulator`, `xdg-terminal-exec` and common terminals (COSMIC
Terminal, GNOME Console and Terminal, Ptyxis, Konsole, Xfce Terminal, Tilix,
Alacritty, Kitty, Foot, xterm…). Install one of them, or set `$TERMINAL`. Inside
a Distrobox container with no terminal, 2fip opens your system's terminal.

### Thumbnails don't appear

Check that **Show thumbnails** is on in Settings. 2fip uses the thumbnail
programs of your system (for images, PDFs, videos, fonts); install the one for
the type you need (for example `ffmpegthumbnailer` for videos). There are no
thumbnails for files on servers.

### A server asks for my password every time

Edit the connection in the **Connections** panel and tick **Remember the
password in the system keyring**. If your system has no keyring service
(GNOME Keyring or KWallet), passwords can't be stored.

### "The server's key has changed"

The server's SSH key doesn't match the one in `~/.ssh/known_hosts`. If you
know the server was reinstalled, remove its old line from that file and
connect again; otherwise, don't connect and ask its administrator.

### FTPS says the certificate isn't valid

2fip only accepts valid certificates (like a browser). Servers with a
self-signed certificate can't be used over FTPS yet; use SFTP if the server
offers it.

### Where are my settings?

In `~/.config/2fip/`: `settings.json`, `favorites.json`, `connections.json`
and `session.json` (the open tabs). Delete a file to reset that part.

### The AppImage doesn't start

It needs glibc 2.39 or newer (Ubuntu / Pop!_OS 24.04, Fedora 40, or later). On
older systems, use the Flatpak.

### I found a bug or have an idea

Please tell us on [GitHub](https://github.com/chillalabs/twofip/issues).
