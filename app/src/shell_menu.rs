//! Windows: the right-click menu of Windows Explorer (with the entries other
//! programs add, like 7-Zip or OneDrive), as Total Commander shows it.
//! Cut, Copy, Paste, Rename and Delete run as 2fip's own actions.

use std::cell::RefCell;
use std::ffi::CStr;
use std::path::PathBuf;

use windows::core::{Interface, HSTRING, PCSTR, PSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    DefSubclassProc, IContextMenu, IContextMenu2, IContextMenu3, ILFindLastID, ILFree,
    IShellFolder, RemoveWindowSubclass, SHBindToParent, SHParseDisplayName, SetWindowSubclass,
    CMF_CANRENAME, CMF_EXTENDEDVERBS, CMF_NORMAL, CMINVOKECOMMANDINFO, GCS_VERBA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, GetCursorPos, GetForegroundWindow, TrackPopupMenuEx,
    SW_SHOWNORMAL, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_DRAWITEM, WM_INITMENUPOPUP, WM_MEASUREITEM,
    WM_MENUCHAR,
};

use crate::keybinds::Action;

/// Menu command ids handed to the shell (0 means "nothing chosen").
const FIRST_ID: u32 = 1;
const LAST_ID: u32 = 0x7fff;
const SUBCLASS_ID: usize = 0x2f1b;

thread_local! {
    /// The menu being shown, for its submenus ("Open with", "Send to"),
    /// which fill themselves in through window messages.
    static ACTIVE: RefCell<Option<IContextMenu>> = const { RefCell::new(None) };
}

/// Shows Explorer's menu for `paths` (all in one folder) at the mouse
/// pointer. Returns the 2fip action to run: the chosen Cut/Copy/Paste/
/// Rename/Delete, or Refresh after a command Windows ran itself.
pub fn show(paths: &[PathBuf]) -> Option<Action> {
    // SAFETY: plain shell/COM calls on the UI thread; every pointer comes
    // from the shell and is freed by `Pidls` or the COM wrappers.
    match unsafe { show_menu(paths) } {
        Ok(action) => action,
        Err(err) => {
            eprintln!("Explorer menu: {err}");
            None
        }
    }
}

/// Absolute item ids, freed on drop.
struct Pidls(Vec<*mut ITEMIDLIST>);

impl Drop for Pidls {
    fn drop(&mut self) {
        for pidl in &self.0 {
            // SAFETY: allocated by SHParseDisplayName.
            unsafe { ILFree(Some(*pidl)) };
        }
    }
}

unsafe fn show_menu(paths: &[PathBuf]) -> windows::core::Result<Option<Action>> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let hwnd = GetForegroundWindow();

        let mut pidls = Pidls(Vec::new());
        for path in paths {
            let mut pidl = std::ptr::null_mut();
            SHParseDisplayName(&HSTRING::from(path.as_os_str()), None, &mut pidl, 0, None)?;
            pidls.0.push(pidl);
        }
        let Some(&first) = pidls.0.first() else {
            return Ok(None);
        };
        let folder: IShellFolder = SHBindToParent(first, None)?;
        let children: Vec<*const ITEMIDLIST> = pidls
            .0
            .iter()
            .map(|pidl| ILFindLastID(*pidl) as *const ITEMIDLIST)
            .collect();
        let menu: IContextMenu = folder.GetUIObjectOf(hwnd, &children, None)?;

        let hmenu = CreatePopupMenu()?;
        let mut flags = CMF_NORMAL | CMF_CANRENAME;
        if crate::windows::shift_held() {
            // Shift+right-click: "Copy as path", "Open PowerShell here", ...
            flags |= CMF_EXTENDEDVERBS;
        }
        if let Err(err) = menu
            .QueryContextMenu(hmenu, 0, FIRST_ID, LAST_ID, flags)
            .ok()
        {
            let _ = DestroyMenu(hmenu);
            return Err(err);
        }

        let mut point = POINT::default();
        GetCursorPos(&mut point)?;
        ACTIVE.with(|active| *active.borrow_mut() = Some(menu.clone()));
        let _ = SetWindowSubclass(hwnd, Some(forward_menu_messages), SUBCLASS_ID, 0);
        let chosen = TrackPopupMenuEx(
            hmenu,
            TPM_RETURNCMD.0 | TPM_RIGHTBUTTON.0,
            point.x,
            point.y,
            hwnd,
            None,
        )
        .0 as u32;
        let _ = RemoveWindowSubclass(hwnd, Some(forward_menu_messages), SUBCLASS_ID);
        ACTIVE.with(|active| *active.borrow_mut() = None);

        let action = if chosen >= FIRST_ID {
            run_command(&menu, hwnd, chosen - FIRST_ID)
        } else {
            None
        };
        let _ = DestroyMenu(hmenu);
        Ok(action)
    }
}

/// Runs the chosen menu entry `id`: as a 2fip action when it's one 2fip
/// has, otherwise by Windows.
unsafe fn run_command(menu: &IContextMenu, hwnd: HWND, id: u32) -> Option<Action> {
    let mut buffer = [0u8; 64];
    // SAFETY: the buffer's size is passed along.
    let verb = unsafe {
        menu.GetCommandString(
            id as usize,
            GCS_VERBA,
            None,
            PSTR(buffer.as_mut_ptr()),
            buffer.len() as u32,
        )
    }
    .ok()
    .and_then(|()| CStr::from_bytes_until_nul(&buffer).ok())
    .map(|verb| verb.to_string_lossy().to_lowercase())
    .unwrap_or_default();

    let own = match verb.as_str() {
        "rename" => Some(Action::Rename),
        "delete" if crate::windows::shift_held() => Some(Action::DeletePermanently),
        "delete" => Some(Action::Delete),
        "paste" => Some(Action::Paste),
        _ => None,
    };
    if own.is_some() {
        return own;
    }
    let info = CMINVOKECOMMANDINFO {
        cbSize: std::mem::size_of::<CMINVOKECOMMANDINFO>() as u32,
        hwnd,
        // MAKEINTRESOURCE(id): the command by number.
        lpVerb: PCSTR(id as usize as *const u8),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // SAFETY: `info` is a valid CMINVOKECOMMANDINFO.
    if let Err(err) = unsafe { menu.InvokeCommand(&info) } {
        eprintln!("Explorer menu command: {err}");
    }
    // Cut and Copy also go to 2fip's own clipboard (Windows got them too,
    // so Explorer can paste them).
    match verb.as_str() {
        "cut" => Some(Action::Cut),
        "copy" => Some(Action::CopyToClipboard),
        _ => Some(Action::Refresh),
    }
}

/// While the menu is open, passes the messages that build and draw its
/// submenus to the shell's menu.
unsafe extern "system" fn forward_menu_messages(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if matches!(
        msg,
        WM_INITMENUPOPUP | WM_DRAWITEM | WM_MEASUREITEM | WM_MENUCHAR
    ) {
        let handled = ACTIVE.with(|active| {
            let active = active.borrow();
            let menu = active.as_ref()?;
            // SAFETY: forwarding the window message as received.
            unsafe {
                if let Ok(menu) = menu.cast::<IContextMenu3>() {
                    let mut result = LRESULT(0);
                    return menu
                        .HandleMenuMsg2(msg, wparam, lparam, Some(&mut result))
                        .ok()
                        .map(|()| result);
                }
                let menu = menu.cast::<IContextMenu2>().ok()?;
                menu.HandleMenuMsg(msg, wparam, lparam)
                    .ok()
                    .map(|()| LRESULT(0))
            }
        });
        if let Some(result) = handled {
            return result;
        }
    }
    // SAFETY: the default handling of the window's own procedure.
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}
