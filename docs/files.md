---
layout: default
title: Working with files
---

## Selecting

| To select | Mouse | Keyboard |
|---|---|---|
| One item | Click | <kbd>↑</kbd> / <kbd>↓</kbd>, <kbd>Home</kbd> / <kbd>End</kbd> |
| Add or remove one | <kbd>Ctrl</kbd>+click | — |
| A range | <kbd>Shift</kbd>+click | <kbd>Shift</kbd>+<kbd>↑</kbd> / <kbd>↓</kbd> / <kbd>Home</kbd> / <kbd>End</kbd> |
| Everything | — | <kbd>Ctrl</kbd>+<kbd>A</kbd> |

Right-click an item for its menu: Open, Open with…, View, Edit, Calculate
size, Cut, Copy, Paste, Rename…, Compress…, Delete and Show details.

## Opening

- **Folders:** double-click, <kbd>Enter</kbd> or <kbd>→</kbd>. Go back up
  with <kbd>Backspace</kbd> or <kbd>←</kbd>; 2fip selects the folder you came
  from.
- **Files:** double-click or <kbd>Enter</kbd> opens a file with its default
  app.
- **<kbd>F3</kbd> View** opens the file's default app (images: the image
  viewer).
- **<kbd>F4</kbd> Edit** always opens the text editor (images: the image
  viewer), even for files whose default app is something else.
- **Open with…** (right-click) lists the apps for the file's type first, then
  all the others.

## Copying and moving

- **<kbd>F5</kbd> Copy** and **<kbd>F6</kbd> Move** send the selection to the
  **other panel's** folder.
- **Clipboard:** <kbd>Ctrl</kbd>+<kbd>C</kbd> (copy) or
  <kbd>Ctrl</kbd>+<kbd>X</kbd> (cut), then <kbd>Ctrl</kbd>+<kbd>V</kbd> in any
  folder.
- **Drag and drop** between the panels, into a folder, or from and to other
  apps. Dropping copies; hold <kbd>Shift</kbd> to move.

Operations run in the background, with a progress panel and **Cancel**. If a
file already exists, 2fip asks: **Replace**, **Skip**, or the same for all.

## Renaming and new folders

- **<kbd>F2</kbd>** renames the selected item.
- **<kbd>F7</kbd>** creates a new folder.

## Deleting

- **<kbd>F8</kbd>** or **<kbd>Delete</kbd>** moves the selection **to the
  trash**, after asking.
- **<kbd>Shift</kbd>+<kbd>Delete</kbd>** or **<kbd>Shift</kbd>+<kbd>F8</kbd>**
  deletes **permanently**, without the trash. This can't be undone, so 2fip
  asks first, with a red warning.
- On a server, deleting is always permanent (servers have no trash).

In the question, <kbd>←</kbd> / <kbd>→</kbd> or <kbd>Tab</kbd> choose the
button and <kbd>Enter</kbd> confirms; <kbd>Esc</kbd> cancels.

## More

- **Folder sizes:** select folders and press <kbd>Space</kbd>; their total size
  appears in the Size column.
- **Details:** <kbd>Alt</kbd>+<kbd>Enter</kbd> shows type, size, dates,
  permissions and owner.
- **Compress…** (right-click) packs the selection into a ZIP archive.
- **Terminal:** <kbd>F9</kbd> opens your terminal in the current folder. 2fip
  uses the terminal chosen in COSMIC Settings, then `$TERMINAL`, then the
  usual terminals of other desktops.
- **Hidden files:** <kbd>Ctrl</kbd>+<kbd>H</kbd> shows or hides files whose
  name starts with a dot.
- **Refresh:** <kbd>Ctrl</kbd>+<kbd>R</kbd> re-reads both panels.
