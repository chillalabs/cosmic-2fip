---
layout: default
title: Servers and connections
---

2fip can show folders on a server in a panel, and copy, move, rename and
delete there like on your computer. It supports **SFTP** (file transfer over
SSH), **FTP** and **FTPS** (FTP encrypted with TLS).

2fip connects by itself: it doesn't mount anything on your system, so it works
the same on every distribution, in the Flatpak and in the AppImage.

## Connecting

Press <kbd>Ctrl</kbd>+<kbd>K</kbd> (**File → Connect to server…**):

1. Choose the **protocol**, type the **server** name or address, and check the
   **port** (filled in for you: 22 for SFTP, 21 for FTP and FTPS).
2. Type your **user name** and, if needed, your **password**.
3. Press **Connect**. The active panel opens your home folder on the server.

You can also type an address in the path bar (<kbd>Ctrl</kbd>+<kbd>L</kbd>),
for example `sftp://me@example.com/home/me`.

### SFTP

- 2fip tries your **SSH agent** first, then your keys in `~/.ssh`
  (`id_ed25519`, `id_ecdsa`, `id_rsa`), and the password last. With SSH keys
  set up, you don't need a password.
- The first time you connect to a server, 2fip shows its **key fingerprint**.
  Compare it with the one your server administrator gave you, then choose
  **Trust and connect**. It's saved in `~/.ssh/known_hosts`, like `ssh` does.
- If a server's key has **changed**, 2fip warns you and doesn't connect:
  someone may be intercepting the connection. If the server was reinstalled,
  remove its old line from `~/.ssh/known_hosts`.

### FTP and FTPS

- Leave the user name empty to log in **anonymously**.
- Plain FTP sends your password **unencrypted**. Use FTPS or SFTP when the
  server offers them.
- FTPS checks the server's certificate; it must be valid (not self-signed).

## Working on a server

- **Copy and move** with <kbd>F5</kbd> / <kbd>F6</kbd>, the clipboard or drag
  and drop, in both directions and even between two servers. The progress
  panel shows the transferred bytes.
- **Rename** (<kbd>F2</kbd>), **new folder** (<kbd>F7</kbd>) and **delete**
  work as usual. Deleting on a server is always **permanent**.
- **Opening a file** (<kbd>Enter</kbd>, <kbd>F3</kbd>, <kbd>F4</kbd>, Open
  with…) downloads a copy to `~/.cache/2fip/remote/` and opens that. Changes
  to the copy are **not** uploaded back: copy the file to the server again
  when you're done.
- Not available on servers yet: thumbnails, folder sizes, Find files, Details
  and Compress.

## The Connections panel

The **Connections** button in the header (or **File → Connections**) lists:

- **Saved** connections, each with its status: green (connected), orange
  (connecting), red (connection lost) or grey (not connected), and these
  buttons:
  - **Open:** connects if needed (with the stored password) and shows the
    server in the active panel.
  - **Reconnect:** closes and opens the connection again, for example after
    the server closed it for being idle.
  - **Disconnect:** closes it; panels showing that server go to your home
    folder.
  - **Edit** and **Delete**.
- **Open, not saved:** connections made with <kbd>Ctrl</kbd>+<kbd>K</kbd>,
  with a **Save** button.

**Add connection** opens the connection dialog with a **Name** and two
options:

- **Save in Connections:** keep it in the list.
- **Remember the password in the system keyring:** see below.

**Save** stores the connection without connecting.

## Passwords

Passwords you choose to remember are stored **only in your system keyring**
(GNOME Keyring, KWallet, or the Secret portal in the Flatpak): encrypted, and
unlocked when you log in. They're never written to 2fip's files; the list of
saved connections (`~/.config/2fip/connections.json`) has no passwords. You
can see or delete them in **Passwords and Keys** (Seahorse), as "2fip: *name*".

Passwords you don't remember are kept in memory only while 2fip runs, so it
can reconnect if the server drops the connection.

## When connections close

- A connection closes by itself when **no tab** in either panel shows that
  server anymore (except while a copy or move is still running).
- If a server drops an idle connection, 2fip reconnects automatically the next
  time you use it.
- Connections don't survive a restart: open them again from the Connections
  panel.
