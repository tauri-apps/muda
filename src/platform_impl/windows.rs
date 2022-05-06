#![cfg(target_os = "windows")]

use crate::util::{encode_wide, Counter, LOWORD};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Shell::{DefSubclassProc, SetWindowSubclass},
        WindowsAndMessaging::{
            AppendMenuW, CreateMenu, EnableMenuItem, SetMenu, SetMenuItemInfoW, MENUITEMINFOW,
            MF_DISABLED, MF_ENABLED, MF_GRAYED, MF_POPUP, MIIM_STRING, WM_COMMAND,
        },
    },
};

static COUNTER: Counter = Counter::new();

pub struct Menu(isize);

impl Menu {
    pub fn new() -> Self {
        Self(unsafe { CreateMenu() })
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let hmenu = unsafe { CreateMenu() };
        let mut flags = MF_POPUP;
        if !enabled {
            flags |= MF_GRAYED;
        }
        unsafe {
            AppendMenuW(
                self.0,
                flags,
                hmenu as _,
                encode_wide(label.as_ref()).as_ptr(),
            )
        };
        Submenu {
            label: label.as_ref().to_string(),
            enabled,
            hmenu,
            parent_hmenu: self.0,
        }
    }

    pub fn init_for_hwnd(&self, hwnd: isize) {
        unsafe {
            SetMenu(hwnd, self.0);
            SetWindowSubclass(hwnd, Some(menu_subclass_proc), 22, 0);
        };
    }
}

#[derive(Clone)]
pub struct Submenu {
    label: String,
    enabled: bool,
    hmenu: isize,
    parent_hmenu: isize,
}

impl Submenu {
    pub fn label(&self) -> String {
        self.label.clone()
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        self.label = label.as_ref().to_string();

        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        info.fMask = MIIM_STRING;
        info.dwTypeData = encode_wide(&self.label).as_mut_ptr();

        unsafe { SetMenuItemInfoW(self.parent_hmenu, self.hmenu as u32, false.into(), &info) };
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        unsafe {
            EnableMenuItem(
                self.parent_hmenu,
                self.hmenu as _,
                if enabled { MF_ENABLED } else { MF_DISABLED },
            )
        };
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let hmenu = unsafe { CreateMenu() };
        let mut flags = MF_POPUP;
        if !enabled {
            flags |= MF_GRAYED;
        }
        unsafe {
            AppendMenuW(
                self.hmenu,
                flags,
                hmenu as _,
                encode_wide(label.as_ref()).as_ptr(),
            )
        };
        Submenu {
            label: label.as_ref().to_string(),
            enabled,
            hmenu,
            parent_hmenu: self.hmenu,
        }
    }

    pub fn add_text_item(&mut self, label: impl AsRef<str>, enabled: bool) -> TextMenuItem {
        let id = COUNTER.next();
        let mut flags = MF_POPUP;
        if !enabled {
            flags |= MF_GRAYED;
        }
        unsafe {
            AppendMenuW(
                self.hmenu,
                flags,
                id as _,
                encode_wide(label.as_ref()).as_ptr(),
            )
        };
        TextMenuItem {
            label: label.as_ref().to_string(),
            enabled,
            id,
            parent_hmenu: self.hmenu,
        }
    }
}

#[derive(Clone)]
pub struct TextMenuItem {
    label: String,
    enabled: bool,
    id: u64,
    parent_hmenu: isize,
}

impl TextMenuItem {
    pub fn label(&self) -> String {
        self.label.clone()
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        self.label = label.as_ref().to_string();

        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        info.fMask = MIIM_STRING;
        info.dwTypeData = encode_wide(&self.label).as_mut_ptr();

        unsafe { SetMenuItemInfoW(self.parent_hmenu, self.id as u32, false.into(), &info) };
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        unsafe {
            EnableMenuItem(
                self.parent_hmenu,
                self.id as _,
                if enabled { MF_ENABLED } else { MF_DISABLED },
            )
        };
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

unsafe extern "system" fn menu_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uidsubclass: usize,
    _dwrefdata: usize,
) -> LRESULT {
    let id = LOWORD(wparam as _);
    if msg == WM_COMMAND && 0 < id && (id as u64) < COUNTER.current() {
        let _ = crate::MENU_CHANNEL.0.send(crate::MenuEvent { id: id as _ });
    };

    DefSubclassProc(hwnd, msg, wparam, lparam)
}
