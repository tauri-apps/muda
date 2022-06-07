#![cfg(target_os = "windows")]

mod accelerator;
mod util;

use crate::counter::Counter;
use std::{cell::RefCell, rc::Rc};
use util::{decode_wide, encode_wide, LOWORD};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            AppendMenuW, CreateAcceleratorTableW, CreateMenu, EnableMenuItem, GetMenuItemInfoW,
            SetMenu, SetMenuItemInfoW, ACCEL, HACCEL, HMENU, MENUITEMINFOW, MFS_DISABLED,
            MF_DISABLED, MF_ENABLED, MF_GRAYED, MF_POPUP, MIIM_STATE, MIIM_STRING, WM_COMMAND,
        },
    },
};

use self::accelerator::parse_accelerator;

static COUNTER: Counter = Counter::new_with_start(563);
const MENU_SUBCLASS_ID: usize = 232;

struct InnerMenu {
    hmenu: HMENU,
    accelerators: Vec<ACCEL>,
    haccel: HACCEL,
}

#[derive(Clone)]
pub struct Menu(Rc<RefCell<InnerMenu>>);

impl Menu {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(InnerMenu {
            hmenu: unsafe { CreateMenu() },
            accelerators: Vec::new(),
            haccel: 0,
        })))
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let hmenu = unsafe { CreateMenu() };
        let mut flags = MF_POPUP;
        if !enabled {
            flags |= MF_GRAYED;
        }
        unsafe {
            AppendMenuW(
                self.0.borrow().hmenu,
                flags,
                hmenu as _,
                encode_wide(label.as_ref()).as_ptr(),
            )
        };
        Submenu {
            hmenu,
            parent_hmenu: self.0.borrow().hmenu,
            parent_menu: self.clone(),
        }
    }

    pub fn init_for_hwnd(&self, hwnd: isize) {
        unsafe {
            SetMenu(hwnd, self.0.borrow().hmenu);
            SetWindowSubclass(hwnd, Some(menu_subclass_proc), MENU_SUBCLASS_ID, 0);
        };
    }

    pub fn haccel(&self) -> HACCEL {
        self.0.borrow().haccel
    }

    fn update_haccel(&mut self) {
        let mut inner = self.0.borrow_mut();
        inner.haccel = unsafe {
            CreateAcceleratorTableW(inner.accelerators.as_ptr(), inner.accelerators.len() as _)
        };
    }

    pub fn remove_for_hwnd(&self, hwnd: isize) {
        unsafe {
            RemoveWindowSubclass(hwnd, Some(menu_subclass_proc), MENU_SUBCLASS_ID);
            SetMenu(hwnd, 0);
        }
    }

    pub fn hide_for_hwnd(&self, hwnd: isize) {
        unsafe {
            SetMenu(hwnd, 0);
        }
    }

    pub fn show_for_hwnd(&self, hwnd: isize) {
        unsafe {
            SetMenu(hwnd, self.0.borrow().hmenu);
        }
    }
}

#[derive(Clone)]
pub struct Submenu {
    hmenu: HMENU,
    parent_hmenu: HMENU,
    parent_menu: Menu,
}

impl Submenu {
    pub fn label(&self) -> String {
        let mut label = Vec::<u16>::new();

        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        info.fMask = MIIM_STRING;
        info.dwTypeData = label.as_mut_ptr();

        unsafe { GetMenuItemInfoW(self.parent_hmenu, self.hmenu as _, false.into(), &mut info) };

        info.cch += 1;
        info.dwTypeData = Vec::with_capacity(info.cch as usize).as_mut_ptr();

        unsafe { GetMenuItemInfoW(self.parent_hmenu, self.hmenu as _, false.into(), &mut info) };

        // TOOD: check if it returns the label containing an ambersand and make gtk comply to that
        decode_wide(info.dwTypeData)
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        info.fMask = MIIM_STRING;
        info.dwTypeData = encode_wide(label.as_ref()).as_mut_ptr();

        unsafe { SetMenuItemInfoW(self.parent_hmenu, self.hmenu as u32, false.into(), &info) };
    }

    pub fn enabled(&self) -> bool {
        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        info.fMask = MIIM_STATE;

        unsafe { GetMenuItemInfoW(self.parent_hmenu, self.hmenu as _, false.into(), &mut info) };

        (info.fState & MFS_DISABLED) == 0
    }

    pub fn set_enabled(&mut self, enabled: bool) {
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
            hmenu,
            parent_hmenu: self.hmenu,
            parent_menu: self.parent_menu.clone(),
        }
    }

    pub fn add_text_item(
        &mut self,
        label: impl AsRef<str>,
        enabled: bool,
        accelerator: Option<&str>,
    ) -> TextMenuItem {
        let id = COUNTER.next();
        let mut flags = MF_POPUP;
        if !enabled {
            flags |= MF_GRAYED;
        }

        let mut label = label.as_ref().to_string();
        if let Some(accelerator) = accelerator {
            let (key, mods, accel_str) = parse_accelerator(accelerator);
            let accel = ACCEL {
                key,
                fVirt: mods as _,
                cmd: id as _,
            };

            label.push_str("\t");
            label.push_str(&accel_str);
            {
                let mut parent_inner = self.parent_menu.0.borrow_mut();
                parent_inner.accelerators.push(accel);
            }
            self.parent_menu.update_haccel();
        }

        unsafe { AppendMenuW(self.hmenu, flags, id as _, encode_wide(label).as_ptr()) };
        TextMenuItem {
            id,
            parent_hmenu: self.hmenu,
        }
    }
}

#[derive(Clone)]
pub struct TextMenuItem {
    id: u64,
    parent_hmenu: HMENU,
}

impl TextMenuItem {
    pub fn label(&self) -> String {
        self.label_with_accel()
            .split("\t")
            .next()
            .unwrap_or_default()
            .to_string()
    }

    fn label_with_accel(&self) -> String {
        let mut label = Vec::<u16>::new();

        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        info.fMask = MIIM_STRING;
        info.dwTypeData = label.as_mut_ptr();

        unsafe { GetMenuItemInfoW(self.parent_hmenu, self.id as _, false.into(), &mut info) };

        info.cch += 1;
        info.dwTypeData = Vec::with_capacity(info.cch as usize).as_mut_ptr();

        unsafe { GetMenuItemInfoW(self.parent_hmenu, self.id as _, false.into(), &mut info) };

        decode_wide(info.dwTypeData)
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        let mut label = label.as_ref().to_string();
        let prev_label = self.label_with_accel();
        if let Some(accel_str) = prev_label.split("\t").nth(1) {
            label.push_str("\t");
            label.push_str(accel_str);
        }

        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        info.fMask = MIIM_STRING;
        info.dwTypeData = encode_wide(label).as_mut_ptr();

        unsafe { SetMenuItemInfoW(self.parent_hmenu, self.id as u32, false.into(), &info) };
    }

    pub fn enabled(&self) -> bool {
        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        info.fMask = MIIM_STATE;

        unsafe { GetMenuItemInfoW(self.parent_hmenu, self.id as _, false.into(), &mut info) };

        (info.fState & MFS_DISABLED) == 0
    }

    pub fn set_enabled(&mut self, enabled: bool) {
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
