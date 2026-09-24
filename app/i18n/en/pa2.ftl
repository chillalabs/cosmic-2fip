# pa2 — English (base language; every key must exist here)

## Menu bar
menu-file = File
menu-edit = Edit
menu-view = View
new-tab = New Tab
new-folder = New Folder
close-tab = Close Tab
quit = Quit
cut = Cut
copy = Copy
paste = Paste
select-all = Select All
rename-ellipsis = Rename…
delete = Delete
list-view = List View
grid-view = Grid View
favorites = Favorites
settings = Settings

## File context menu
open = Open
open-with-ellipsis = Open With…
view = View
edit = Edit
calculate-size = Calculate Size
compress-ellipsis = Compress…
show-details = Show Details

## List columns
column-name = Name
column-ext = Ext
column-size = Size
column-modified = Modified

## Running operations (progress panel)
op-copying = Copying
op-moving = Moving
op-deleting = Deleting
op-compressing = Compressing
op-copy-failed = Copy failed
op-move-failed = Move failed
op-delete-failed = Delete failed
op-compress-failed = Compress failed
op-progress = { $operation } { $percent }%
op-files-done = { $done } of { $total } files
op-bytes-done = { $done } of { $total }
op-preparing = Preparing…
close = Close
cancel = Cancel

## Panels
path-not-found = Not found: { $path }
status-items = { $count ->
    [one] 1 item
   *[other] { $count } items
}
status-items-filtered = { $count ->
    [one] 1 item matches the filter
   *[other] { $count } items match the filter
}
status-selected = { $selected } of { $total } selected ({ $size })
tooltip-back = Back (Alt+←)
tooltip-forward = Forward (Alt+→)
tooltip-up = Up one folder (Backspace)
tooltip-new-tab = New tab (Ctrl+T)
tooltip-edit-path = Edit path (Ctrl+L)
tooltip-cancel-esc = Cancel (Esc)
no-tab-open = No tab open
filter-placeholder = Filter (e.g. report or *.txt)
path-placeholder = Type a folder path

## Settings
settings-general = General
settings-hide-hidden = Hide hidden files
settings-hide-hidden-description = Files and folders whose name starts with a dot
settings-separate-ext = Show extension in its own column
settings-separate-ext-description = List view: separate Name and Ext columns, like Total Commander
settings-thumbnails = Show thumbnails
settings-thumbnails-description = Previews of images, PDFs, videos and fonts
settings-language = Language
language-system = System default
settings-theme = Theme
settings-icon-style = Icon style
settings-icon-style-description = Colorful matches the COSMIC Files app
icon-style-colorful = Colorful
icon-style-monochrome = Monochrome
settings-font-size = File name size
settings-font-size-description = Smaller text fits more files on screen
font-size-default = Default ({ $px } px)
font-size-small = Small ({ $px } px)
font-size-smaller = Smaller ({ $px } px)
font-size-tiny = Tiny ({ $px } px)

## Favorites
favorites-saved = Saved folders
favorites-empty = No favorites saved yet.
favorites-move-up = Move up (Ctrl+↑)
favorites-move-down = Move down (Ctrl+↓)
favorites-add-current = Add current folder
favorites-add = Add

## "File Already Exists" dialog
conflict-title = File Already Exists
conflict-body = "{ $existing }" already exists in the destination. Replace it with "{ $new }"?
skip-all = Skip All
replace-all = Replace All
replace = Replace
skip = Skip

## Compress dialog
compress = Compress
compress-one = Compress "{ $name }" into a zip archive.
compress-many = Compress { $count } items into a zip archive.
archive-name = Archive name
default-archive-name = Archive

## New folder dialog
new-folder-default-name = New folder
folder-name = Folder name
create = Create

## Rename dialog
rename = Rename
renaming = Renaming "{ $name }"
new-name = New name

## Delete dialog
delete-one = Move "{ $name }" to the trash?
delete-many = Move { $count } items to the trash?

## Function key bar (the "F2" etc. prefix is added by the app)
fkey-rename = Rename
fkey-view = View
fkey-edit = Edit
fkey-copy = Copy
fkey-move = Move
fkey-mkdir = MkDir
fkey-delete = Delete
fkey-terminal = Terminal
fkey-exit = Exit

## Open With dialog
open-with = Open With
open-with-one = Choose an application to open "{ $name }".
open-with-many = Choose an application to open { $count } items.
loading-apps = Loading applications…
no-apps = No applications found.
recommended-apps = Recommended Applications
other-apps = Other Applications
default-app = Default

## Details dialog
details-items-title = { $count } items
details-calculating = Calculating…
details-error = Error
details-contents = { $files } files, { $folders } folders inside
details-size = { $size } ({ $bytes } bytes)
details-items = Items
details-location = Location
details-total-size = Total size
details-type = Type
details-name = Name
details-link-target = Link target
details-size-label = Size
details-modified = Modified
details-accessed = Accessed
details-created = Created
details-permissions = Permissions
details-owner = Owner
details-group = Group
details-unknown = Unknown
kind-folder = Folder
kind-file = File
kind-symlink = Symbolic link
