use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateMenu, EnableMenuItem, SetMenu, SetMenuItemInfoW, HMENU, MENUITEMINFOW,
    MF_BYCOMMAND, MF_DISABLED, MF_ENABLED, MF_GRAYED, MF_POPUP, MF_STRING, MIIM_STRING,
};

use crate::{util::encode_wide, MenuEntry, IDS_COUNTER};

pub struct MenuBar(HMENU);

impl MenuBar {
    pub fn new() -> Self {
        Self(unsafe { CreateMenu() })
    }

    pub fn add_entry<M: MenuEntry>(&mut self, entry: &mut M) {
        let mut flags = 0;
        let id;

        if entry.is_menu() {
            flags |= MF_POPUP;
            let menu = entry.platform_menu().unwrap();
            id = menu.hmenu as _;
            menu.parent = self.0;
        } else {
            flags |= MF_STRING;
            let item = entry.platform_item().unwrap();
            id = item.id as _;
            item.parent = self.0;
        };

        if !entry.enabled() {
            flags |= MF_GRAYED;
        }

        unsafe {
            AppendMenuW(self.0, flags, id, encode_wide(entry.title()).as_mut_ptr());
        }
    }

    pub fn init_for_hwnd(&self, hwnd: isize) {
        unsafe { SetMenu(hwnd, self.0) };
    }
}

pub struct Menu {
    hmenu: HMENU,
    parent: HMENU,
}

impl Menu {
    pub fn new(_title: impl Into<String>) -> Self {
        Self {
            hmenu: unsafe { CreateMenu() },
            parent: 0,
        }
    }

    pub fn id(&self) -> u64 {
        self.hmenu as u64
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        let mut item_info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        item_info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        item_info.fMask = MIIM_STRING;
        item_info.dwTypeData = encode_wide(title.into()).as_mut_ptr();

        unsafe { SetMenuItemInfoW(self.parent, self.hmenu as u32, false.into(), &item_info) };
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        let enabled = if enabled { MF_ENABLED } else { MF_DISABLED };
        unsafe { EnableMenuItem(self.parent, self.hmenu as u32, MF_BYCOMMAND | enabled) };
    }

    pub fn add_entry<M: MenuEntry>(&mut self, entry: &mut M) {
        let mut flags = 0;
        let id;

        if entry.is_menu() {
            flags |= MF_POPUP;
            let menu = entry.platform_menu().unwrap();
            id = menu.hmenu as _;
            menu.parent = self.hmenu;
        } else {
            flags |= MF_STRING;
            let item = entry.platform_item().unwrap();
            id = item.id as _;
            item.parent = self.hmenu;
        };

        if !entry.enabled() {
            flags |= MF_GRAYED;
        }

        unsafe {
            AppendMenuW(
                self.hmenu,
                flags,
                id,
                encode_wide(entry.title()).as_mut_ptr(),
            );
        }
    }
}

pub struct MenuItem {
    id: u64,
    parent: HMENU,
}

impl MenuItem {
    pub fn new(_title: impl Into<String>) -> Self {
        Self {
            id: IDS_COUNTER.next(),
            parent: 0,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn set_title(&self, title: impl Into<String>) {
        let mut item_info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        item_info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        item_info.fMask = MIIM_STRING;
        item_info.dwTypeData = encode_wide(title.into()).as_mut_ptr();

        unsafe { SetMenuItemInfoW(self.parent, self.id as u32, false.into(), &item_info) };
    }

    pub fn set_enabled(&self, enabled: bool) {
        let enabled = if enabled { MF_ENABLED } else { MF_DISABLED };
        unsafe { EnableMenuItem(self.parent, self.id as u32, MF_BYCOMMAND | enabled) };
    }
}
