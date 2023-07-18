// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod accelerator;
mod icon;
mod util;

pub(crate) use self::icon::WinIcon as PlatformIcon;

use crate::{
    accelerator::Accelerator,
    icon::{Icon, NativeIcon},
    items::PredfinedMenuItemType,
    util::{AddOp, Counter},
    AboutMetadata, CheckMenuItem, IconMenuItem, IsMenuItem, MenuEvent, MenuItem, MenuItemType,
    PredefinedMenuItem, Submenu,
};
use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    fmt::Debug,
    rc::Rc,
};
use util::{decode_wide, encode_wide, Accel};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    Graphics::Gdi::{ClientToScreen, HBITMAP},
    UI::{
        Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, VK_CONTROL},
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            AppendMenuW, CreateAcceleratorTableW, CreateMenu, CreatePopupMenu,
            DestroyAcceleratorTable, DrawMenuBar, EnableMenuItem, GetMenu, GetMenuItemInfoW,
            InsertMenuW, PostQuitMessage, RemoveMenu, SendMessageW, SetMenu, SetMenuItemInfoW,
            ShowWindow, TrackPopupMenu, HACCEL, HMENU, MENUITEMINFOW, MFS_CHECKED, MFS_DISABLED,
            MF_BYCOMMAND, MF_BYPOSITION, MF_CHECKED, MF_DISABLED, MF_ENABLED, MF_GRAYED, MF_POPUP,
            MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MIIM_BITMAP, MIIM_STATE, MIIM_STRING, SW_HIDE,
            SW_MAXIMIZE, SW_MINIMIZE, TPM_LEFTALIGN, WM_CLOSE, WM_COMMAND, WM_DESTROY,
        },
    },
};

static COUNTER: Counter = Counter::new_with_start(1000);

type AccelWrapper = (HACCEL, HashMap<u32, Accel>);

macro_rules! inner_menu_child_and_flags {
    ($item:ident) => {{
        let mut flags = 0;
        let child = match $item.type_() {
            MenuItemType::Submenu => {
                flags |= MF_POPUP;
                &$item.as_any().downcast_ref::<Submenu>().unwrap().0
            }
            MenuItemType::Normal => {
                flags |= MF_STRING;
                &$item.as_any().downcast_ref::<MenuItem>().unwrap().0
            }

            MenuItemType::Predefined => {
                let item = $item.as_any().downcast_ref::<PredefinedMenuItem>().unwrap();
                let child = &item.0;
                let child_ = child.borrow();
                match child_.predefined_item_type {
                    PredfinedMenuItemType::None => return Ok(()),
                    PredfinedMenuItemType::Separator => {
                        flags |= MF_SEPARATOR;
                    }
                    _ => {
                        flags |= MF_STRING;
                    }
                }
                child
            }
            MenuItemType::Check => {
                let item = $item.as_any().downcast_ref::<CheckMenuItem>().unwrap();
                let child = &item.0;
                flags |= MF_STRING;
                if child.borrow().checked {
                    flags |= MF_CHECKED;
                }
                child
            }
            MenuItemType::Icon => {
                flags |= MF_STRING;
                &$item.as_any().downcast_ref::<IconMenuItem>().unwrap().0
            }
        };

        (child.clone(), flags)
    }};
}

#[derive(Debug)]
pub(crate) struct Menu {
    id: u32,
    hmenu: HMENU,
    hpopupmenu: HMENU,
    hwnds: Vec<HWND>,
    haccel_store: Rc<RefCell<AccelWrapper>>,
    children: Vec<Rc<RefCell<MenuChild>>>,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            id: COUNTER.next(),
            hmenu: unsafe { CreateMenu() },
            hpopupmenu: unsafe { CreatePopupMenu() },
            haccel_store: Rc::new(RefCell::new((0, HashMap::new()))),
            children: Vec::new(),
            hwnds: Vec::new(),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn add_menu_item(&mut self, item: &dyn IsMenuItem, op: AddOp) -> crate::Result<()> {
        let (child, mut flags) = inner_menu_child_and_flags!(item);

        {
            child
                .borrow_mut()
                .root_menu_haccel_stores
                .as_mut()
                .unwrap()
                .push(self.haccel_store.clone());
        }

        {
            let child_ = child.borrow();
            if !child_.enabled {
                flags |= MF_GRAYED;
            }

            let mut text = child_.text.clone();

            if let Some(accelerator) = &child_.accelerator {
                let accel_str = accelerator.to_string();

                text.push('\t');
                text.push_str(&accel_str);

                AccelAction::add(
                    &mut self.haccel_store.borrow_mut(),
                    child_.id(),
                    accelerator,
                )?;
            }

            let id = child_.id() as usize;

            let text = encode_wide(text);
            unsafe {
                match op {
                    AddOp::Append => {
                        AppendMenuW(self.hmenu, flags, id, text.as_ptr());
                        AppendMenuW(self.hpopupmenu, flags, id, text.as_ptr());
                    }
                    AddOp::Insert(position) => {
                        InsertMenuW(
                            self.hmenu,
                            position as _,
                            flags | MF_BYPOSITION,
                            id,
                            text.as_ptr(),
                        );
                        InsertMenuW(
                            self.hpopupmenu,
                            position as _,
                            flags | MF_BYPOSITION,
                            id,
                            text.as_ptr(),
                        );
                    }
                }
            }
        }

        {
            let child_ = child.borrow();

            if child_.type_ == MenuItemType::Icon {
                let hbitmap = child_
                    .icon
                    .as_ref()
                    .map(|i| unsafe { i.inner.to_hbitmap() })
                    .unwrap_or(0);
                let info = create_icon_item_info(hbitmap);

                unsafe {
                    SetMenuItemInfoW(self.hmenu, child_.id, false.into(), &info);
                    SetMenuItemInfoW(self.hpopupmenu, child_.id, false.into(), &info);
                };
            }
        }

        {
            let mut child_ = child.borrow_mut();
            child_.parents_hemnu.push(self.hmenu);
            child_.parents_hemnu.push(self.hpopupmenu);
        }

        {
            match op {
                AddOp::Append => self.children.push(child),
                AddOp::Insert(position) => self.children.insert(position, child),
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn IsMenuItem) -> crate::Result<()> {
        unsafe {
            RemoveMenu(self.hmenu, item.id(), MF_BYCOMMAND);
            RemoveMenu(self.hpopupmenu, item.id(), MF_BYCOMMAND);

            for hwnd in &self.hwnds {
                DrawMenuBar(*hwnd);
            }
        }

        let child = item.child();

        {
            let mut child = child.borrow_mut();
            let index = child
                .parents_hemnu
                .iter()
                .position(|h| *h == self.hmenu)
                .ok_or(crate::Error::NotAChildOfThisMenu)?;
            child.parents_hemnu.remove(index);
            let index = child
                .parents_hemnu
                .iter()
                .position(|h| *h == self.hpopupmenu)
                .ok_or(crate::Error::NotAChildOfThisMenu)?;
            child.parents_hemnu.remove(index);
        }

        let index = self
            .children
            .iter()
            .position(|e| e.borrow().id() == item.id())
            .ok_or(crate::Error::NotAChildOfThisMenu)?;
        self.children.remove(index);

        Ok(())
    }

    pub fn items(&self) -> Vec<Box<dyn IsMenuItem>> {
        self.children
            .iter()
            .map(|c| c.borrow().boxed(c.clone()))
            .collect()
    }

    fn find_by_id(&self, id: u32) -> Option<Rc<RefCell<MenuChild>>> {
        find_by_id(id, &self.children)
    }

    pub fn haccel(&self) -> HACCEL {
        self.haccel_store.borrow().0
    }

    pub fn hpopupmenu(&self) -> HMENU {
        self.hpopupmenu
    }

    pub fn init_for_hwnd(&mut self, hwnd: isize) -> crate::Result<()> {
        if self.hwnds.iter().any(|h| *h == hwnd) {
            return Err(crate::Error::AlreadyInitialized);
        }

        self.hwnds.push(hwnd);
        unsafe {
            SetMenu(hwnd, self.hmenu);
            SetWindowSubclass(
                hwnd,
                Some(menu_subclass_proc),
                MENU_SUBCLASS_ID,
                Box::into_raw(Box::new(self)) as _,
            );
            DrawMenuBar(hwnd);
        };

        Ok(())
    }

    pub fn remove_for_hwnd(&mut self, hwnd: isize) -> crate::Result<()> {
        let index = self
            .hwnds
            .iter()
            .position(|h| *h == hwnd)
            .ok_or(crate::Error::NotInitialized)?;
        self.hwnds.remove(index);
        unsafe {
            SendMessageW(hwnd, WM_CLEAR_MENU_DATA, 0, 0);
            RemoveWindowSubclass(hwnd, Some(menu_subclass_proc), MENU_SUBCLASS_ID);
            SetMenu(hwnd, 0);
            DrawMenuBar(hwnd);
        }

        Ok(())
    }

    pub fn attach_menu_subclass_for_hwnd(&self, hwnd: isize) {
        unsafe {
            SetWindowSubclass(
                hwnd,
                Some(menu_subclass_proc),
                MENU_SUBCLASS_ID,
                Box::into_raw(Box::new(self)) as _,
            );
        }
    }

    pub fn detach_menu_subclass_from_hwnd(&self, hwnd: isize) {
        unsafe {
            SendMessageW(hwnd, WM_CLEAR_MENU_DATA, 0, 0);
            RemoveWindowSubclass(hwnd, Some(menu_subclass_proc), MENU_SUBCLASS_ID);
        }
    }

    pub fn hide_for_hwnd(&self, hwnd: isize) -> crate::Result<()> {
        if !self.hwnds.iter().any(|h| *h == hwnd) {
            return Err(crate::Error::NotInitialized);
        }

        unsafe {
            SetMenu(hwnd, HMENU::default());
            DrawMenuBar(hwnd);
        }

        Ok(())
    }

    pub fn show_for_hwnd(&self, hwnd: isize) -> crate::Result<()> {
        if !self.hwnds.iter().any(|h| *h == hwnd) {
            return Err(crate::Error::NotInitialized);
        }

        unsafe {
            SetMenu(hwnd, self.hmenu);
            DrawMenuBar(hwnd);
        }

        Ok(())
    }

    pub fn is_visible_on_hwnd(&self, hwnd: isize) -> bool {
        self.hwnds
            .iter()
            .find(|h| **h == hwnd)
            .map(|hwnd| unsafe { GetMenu(*hwnd) } != HMENU::default())
            .unwrap_or(false)
    }

    pub fn show_context_menu_for_hwnd(&self, hwnd: isize, x: f64, y: f64) {
        show_context_menu(hwnd, self.hpopupmenu, x, y)
    }
}

/// A generic child in a menu
#[derive(Debug, Default)]
pub(crate) struct MenuChild {
    // shared fields between submenus and menu items
    pub type_: MenuItemType,
    text: String,
    enabled: bool,
    parents_hemnu: Vec<HMENU>,
    root_menu_haccel_stores: Option<Vec<Rc<RefCell<AccelWrapper>>>>,

    // menu item fields
    id: u32,
    accelerator: Option<Accelerator>,

    // predefined menu item fields
    predefined_item_type: PredfinedMenuItemType,

    // check menu item fields
    checked: bool,

    // icon menu item fields
    icon: Option<Icon>,

    // submenu fields
    hmenu: HMENU,
    hpopupmenu: HMENU,
    pub children: Option<Vec<Rc<RefCell<MenuChild>>>>,
}

/// Constructors
impl MenuChild {
    pub fn new(text: &str, enabled: bool, accelerator: Option<Accelerator>) -> Self {
        Self {
            type_: MenuItemType::Normal,
            text: text.to_string(),
            enabled,
            parents_hemnu: Vec::new(),
            id: COUNTER.next(),
            accelerator,
            root_menu_haccel_stores: Some(Vec::new()),
            ..Default::default()
        }
    }

    pub fn new_submenu(text: &str, enabled: bool) -> Self {
        Self {
            type_: MenuItemType::Submenu,
            text: text.to_string(),
            enabled,
            parents_hemnu: Vec::new(),
            children: Some(Vec::new()),
            hmenu: unsafe { CreateMenu() },
            hpopupmenu: unsafe { CreatePopupMenu() },
            root_menu_haccel_stores: Some(Vec::new()),
            ..Default::default()
        }
    }

    pub fn new_predefined(item_type: PredfinedMenuItemType, text: Option<String>) -> Self {
        Self {
            type_: MenuItemType::Predefined,
            text: text.unwrap_or_else(|| item_type.text().to_string()),
            enabled: true,
            parents_hemnu: Vec::new(),
            id: COUNTER.next(),
            accelerator: item_type.accelerator(),
            predefined_item_type: item_type,
            root_menu_haccel_stores: Some(Vec::new()),
            ..Default::default()
        }
    }

    pub fn new_check(
        text: &str,
        enabled: bool,
        checked: bool,
        accelerator: Option<Accelerator>,
    ) -> Self {
        Self {
            type_: MenuItemType::Check,
            text: text.to_string(),
            enabled,
            parents_hemnu: Vec::new(),
            id: COUNTER.next(),
            accelerator,
            checked,
            root_menu_haccel_stores: Some(Vec::new()),
            ..Default::default()
        }
    }

    pub fn new_icon(
        text: &str,
        enabled: bool,
        icon: Option<Icon>,
        accelerator: Option<Accelerator>,
    ) -> Self {
        Self {
            type_: MenuItemType::Icon,
            text: text.to_string(),
            enabled,
            parents_hemnu: Vec::new(),
            id: COUNTER.next(),
            accelerator,
            icon,
            root_menu_haccel_stores: Some(Vec::new()),
            ..Default::default()
        }
    }

    pub fn new_native_icon(
        text: &str,
        enabled: bool,
        _native_icon: Option<NativeIcon>,
        accelerator: Option<Accelerator>,
    ) -> Self {
        Self {
            type_: MenuItemType::Icon,
            text: text.to_string(),
            enabled,
            parents_hemnu: Vec::new(),
            id: COUNTER.next(),
            accelerator,
            root_menu_haccel_stores: Some(Vec::new()),
            ..Default::default()
        }
    }
}

/// Shared methods
impl MenuChild {
    pub fn id(&self) -> u32 {
        match self.type_ {
            MenuItemType::Submenu => self.hmenu as u32,
            _ => self.id,
        }
    }

    pub fn text(&self) -> String {
        self.parents_hemnu
            .first()
            .map(|hmenu| {
                let mut label = Vec::<u16>::new();

                let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
                info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
                info.fMask = MIIM_STRING;
                info.dwTypeData = label.as_mut_ptr();

                unsafe { GetMenuItemInfoW(*hmenu, self.id(), false.into(), &mut info) };

                info.cch += 1;
                info.dwTypeData = Vec::with_capacity(info.cch as usize).as_mut_ptr();

                unsafe { GetMenuItemInfoW(*hmenu, self.id(), false.into(), &mut info) };

                let text = decode_wide(info.dwTypeData);
                text.split('\t').next().unwrap().to_string()
            })
            .unwrap_or_else(|| self.text.clone())
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = if let Some(accelerator) = self.accelerator {
            format!("{text}\t{}", accelerator)
        } else {
            text.to_string()
        };
        for parent in &self.parents_hemnu {
            let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
            info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
            info.fMask = MIIM_STRING;
            info.dwTypeData = encode_wide(&self.text).as_mut_ptr();

            unsafe { SetMenuItemInfoW(*parent, self.id(), false.into(), &info) };
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.parents_hemnu
            .first()
            .map(|hmenu| {
                let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
                info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
                info.fMask = MIIM_STATE;

                unsafe { GetMenuItemInfoW(*hmenu, self.id(), false.into(), &mut info) };

                (info.fState & MFS_DISABLED) == 0
            })
            .unwrap_or(self.enabled)
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        for parent in &self.parents_hemnu {
            unsafe {
                EnableMenuItem(
                    *parent,
                    self.id(),
                    if enabled { MF_ENABLED } else { MF_DISABLED },
                )
            };
        }
    }

    pub fn set_accelerator(&mut self, accelerator: Option<Accelerator>) -> crate::Result<()> {
        self.accelerator = accelerator;
        self.set_text(&self.text.clone());

        let haccel_stores = self.root_menu_haccel_stores.as_mut().unwrap();
        for store in haccel_stores {
            let mut store = store.borrow_mut();
            if let Some(accelerator) = self.accelerator {
                AccelAction::add(&mut store, self.id, &accelerator)?
            } else {
                AccelAction::remove(&mut store, self.id)
            }
        }

        Ok(())
    }
}

/// CheckMenuItem methods
impl MenuChild {
    pub fn is_checked(&self) -> bool {
        self.parents_hemnu
            .first()
            .map(|hmenu| {
                let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
                info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
                info.fMask = MIIM_STATE;

                unsafe { GetMenuItemInfoW(*hmenu, self.id(), false.into(), &mut info) };

                (info.fState & MFS_CHECKED) != 0
            })
            .unwrap_or(self.enabled)
    }

    pub fn set_checked(&mut self, checked: bool) {
        use windows_sys::Win32::UI::WindowsAndMessaging;

        self.checked = checked;
        for parent in &self.parents_hemnu {
            unsafe {
                WindowsAndMessaging::CheckMenuItem(
                    *parent,
                    self.id(),
                    if checked { MF_CHECKED } else { MF_UNCHECKED },
                )
            };
        }
    }
}

/// IconMenuItem methods
impl MenuChild {
    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon = icon.clone();

        let hbitmap = icon.map(|i| unsafe { i.inner.to_hbitmap() }).unwrap_or(0);
        let info = create_icon_item_info(hbitmap);
        for parent in &self.parents_hemnu {
            unsafe { SetMenuItemInfoW(*parent, self.id(), false.into(), &info) };
        }
    }
}

/// Submenu methods
impl MenuChild {
    pub fn hpopupmenu(&self) -> HMENU {
        self.hpopupmenu
    }

    pub fn add_menu_item(&mut self, item: &dyn IsMenuItem, op: AddOp) -> crate::Result<()> {
        let (child, mut flags) = inner_menu_child_and_flags!(item);

        {
            child
                .borrow_mut()
                .root_menu_haccel_stores
                .as_mut()
                .unwrap()
                .extend_from_slice(self.root_menu_haccel_stores.as_ref().unwrap());
        }

        {
            let child_ = child.borrow();
            if !child_.enabled {
                flags |= MF_GRAYED;
            }

            let mut text = child_.text.clone();

            if let Some(accelerator) = &child_.accelerator {
                let accel_str = accelerator.to_string();

                text.push('\t');
                text.push_str(&accel_str);

                for root_menu in self.root_menu_haccel_stores.as_mut().unwrap() {
                    let mut haccel = root_menu.borrow_mut();
                    AccelAction::add(&mut haccel, child_.id(), accelerator)?;
                }
            }

            let id = child_.id() as usize;
            let text = encode_wide(text);
            unsafe {
                match op {
                    AddOp::Append => {
                        AppendMenuW(self.hmenu, flags, id, text.as_ptr());
                        AppendMenuW(self.hpopupmenu, flags, id, text.as_ptr());
                    }
                    AddOp::Insert(position) => {
                        InsertMenuW(
                            self.hmenu,
                            position as _,
                            flags | MF_BYPOSITION,
                            id,
                            text.as_ptr(),
                        );
                        InsertMenuW(
                            self.hpopupmenu,
                            position as _,
                            flags | MF_BYPOSITION,
                            id,
                            text.as_ptr(),
                        );
                    }
                }
            }
        }

        {
            let child_ = child.borrow();

            if child_.type_ == MenuItemType::Icon {
                let hbitmap = child_
                    .icon
                    .as_ref()
                    .map(|i| unsafe { i.inner.to_hbitmap() })
                    .unwrap_or(0);
                let info = create_icon_item_info(hbitmap);

                unsafe {
                    SetMenuItemInfoW(self.hmenu, child_.id, false.into(), &info);
                    SetMenuItemInfoW(self.hpopupmenu, child_.id, false.into(), &info);
                };
            }
        }

        {
            let mut child_ = child.borrow_mut();
            child_.parents_hemnu.push(self.hmenu);
            child_.parents_hemnu.push(self.hpopupmenu);
        }

        {
            let children = self.children.as_mut().unwrap();
            match op {
                AddOp::Append => children.push(child),
                AddOp::Insert(position) => children.insert(position, child),
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn IsMenuItem) -> crate::Result<()> {
        unsafe {
            RemoveMenu(self.hmenu, item.id(), MF_BYCOMMAND);
            RemoveMenu(self.hpopupmenu, item.id(), MF_BYCOMMAND);
        }

        let child = item.child();

        {
            let mut child = child.borrow_mut();
            let index = child
                .parents_hemnu
                .iter()
                .position(|h| *h == self.hmenu)
                .ok_or(crate::Error::NotAChildOfThisMenu)?;
            child.parents_hemnu.remove(index);
            let index = child
                .parents_hemnu
                .iter()
                .position(|h| *h == self.hpopupmenu)
                .ok_or(crate::Error::NotAChildOfThisMenu)?;
            child.parents_hemnu.remove(index);
        }

        let children = self.children.as_mut().unwrap();
        let index = children
            .iter()
            .position(|e| e.borrow().id() == item.id())
            .ok_or(crate::Error::NotAChildOfThisMenu)?;
        children.remove(index);

        Ok(())
    }

    pub fn items(&self) -> Vec<Box<dyn IsMenuItem>> {
        self.children
            .as_ref()
            .unwrap()
            .iter()
            .map(|c| c.borrow().boxed(c.clone()))
            .collect()
    }

    pub fn show_context_menu_for_hwnd(&self, hwnd: isize, x: f64, y: f64) {
        show_context_menu(hwnd, self.hpopupmenu, x, y)
    }

    pub fn attach_menu_subclass_for_hwnd(&self, hwnd: isize) {
        unsafe {
            SetWindowSubclass(
                hwnd,
                Some(menu_subclass_proc),
                SUBMENU_SUBCLASS_ID,
                Box::into_raw(Box::new(self)) as _,
            );
        }
    }

    pub fn detach_menu_subclass_from_hwnd(&self, hwnd: isize) {
        unsafe {
            SendMessageW(hwnd, WM_CLEAR_MENU_DATA, 0, 0);
            RemoveWindowSubclass(hwnd, Some(menu_subclass_proc), SUBMENU_SUBCLASS_ID);
        }
    }
}

/// Internal Utilitles
impl MenuChild {
    fn find_by_id(&self, id: u32) -> Option<Rc<RefCell<MenuChild>>> {
        let children = self.children.as_ref().unwrap();
        find_by_id(id, children)
    }
}

fn find_by_id(id: u32, children: &Vec<Rc<RefCell<MenuChild>>>) -> Option<Rc<RefCell<MenuChild>>> {
    for i in children {
        let item = i.borrow();
        if item.id() == id {
            return Some(i.clone());
        }

        if item.type_ == MenuItemType::Submenu {
            if let Some(child) = item.find_by_id(id) {
                return Some(child);
            }
        }
    }
    None
}

fn show_context_menu(hwnd: HWND, hmenu: HMENU, x: f64, y: f64) {
    unsafe {
        let mut point = POINT {
            x: x as _,
            y: y as _,
        };
        ClientToScreen(hwnd, &mut point);
        TrackPopupMenu(
            hmenu,
            TPM_LEFTALIGN,
            point.x,
            point.y,
            0,
            hwnd,
            std::ptr::null(),
        );
    }
}

struct AccelAction;

impl AccelAction {
    fn add(
        haccel_store: &mut RefMut<AccelWrapper>,
        id: u32,
        accelerator: &Accelerator,
    ) -> crate::Result<()> {
        let accel = accelerator.to_accel(id as _)?;
        haccel_store.1.insert(id, Accel(accel));

        Self::update_store(haccel_store);

        Ok(())
    }

    fn remove(haccel_store: &mut RefMut<AccelWrapper>, id: u32) {
        haccel_store.1.remove(&id);

        Self::update_store(haccel_store)
    }

    fn update_store(haccel_store: &mut RefMut<AccelWrapper>) {
        unsafe {
            DestroyAcceleratorTable(haccel_store.0);
            haccel_store.0 = CreateAcceleratorTableW(
                haccel_store
                    .1
                    .values()
                    .map(|i| i.0)
                    .collect::<Vec<_>>()
                    .as_ptr(),
                haccel_store.1.len() as _,
            );
        }
    }
}

fn create_icon_item_info(hbitmap: HBITMAP) -> MENUITEMINFOW {
    let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
    info.fMask = MIIM_BITMAP;
    info.hbmpItem = hbitmap;
    info
}

const MENU_SUBCLASS_ID: usize = 200;
const SUBMENU_SUBCLASS_ID: usize = 201;
const WM_CLEAR_MENU_DATA: u32 = 600;

unsafe extern "system" fn menu_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    uidsubclass: usize,
    dwrefdata: usize,
) -> LRESULT {
    let mut ret = -1;
    if msg == WM_DESTROY || msg == WM_CLEAR_MENU_DATA {
        if uidsubclass == MENU_SUBCLASS_ID {
            drop(Box::from_raw(dwrefdata as *mut Menu));
        } else {
            drop(Box::from_raw(dwrefdata as *mut MenuChild));
        }
    }

    if msg == WM_COMMAND {
        let id = util::LOWORD(wparam as _) as u32;
        let item = if uidsubclass == MENU_SUBCLASS_ID {
            let menu = dwrefdata as *mut Box<Menu>;
            (*menu).find_by_id(id)
        } else {
            let menu = dwrefdata as *mut Box<MenuChild>;
            (*menu).find_by_id(id)
        };

        if let Some(item) = item {
            ret = 0;

            let mut dispatch = true;

            {
                let mut item = item.borrow_mut();

                if item.type_ == MenuItemType::Predefined {
                    dispatch = false;
                }

                match item.type_ {
                    MenuItemType::Check => {
                        let checked = !item.checked;
                        item.set_checked(checked);
                    }
                    MenuItemType::Predefined => match &item.predefined_item_type {
                        PredfinedMenuItemType::Copy => execute_edit_command(EditCommand::Copy),
                        PredfinedMenuItemType::Cut => execute_edit_command(EditCommand::Cut),
                        PredfinedMenuItemType::Paste => execute_edit_command(EditCommand::Paste),
                        PredfinedMenuItemType::SelectAll => {
                            execute_edit_command(EditCommand::SelectAll)
                        }
                        PredfinedMenuItemType::Separator => {}
                        PredfinedMenuItemType::Minimize => {
                            ShowWindow(hwnd, SW_MINIMIZE);
                        }
                        PredfinedMenuItemType::Maximize => {
                            ShowWindow(hwnd, SW_MAXIMIZE);
                        }
                        PredfinedMenuItemType::Hide => {
                            ShowWindow(hwnd, SW_HIDE);
                        }
                        PredfinedMenuItemType::CloseWindow => {
                            SendMessageW(hwnd, WM_CLOSE, 0, 0);
                        }
                        PredfinedMenuItemType::Quit => {
                            PostQuitMessage(0);
                        }
                        PredfinedMenuItemType::About(Some(ref metadata)) => {
                            show_about_dialog(hwnd, metadata)
                        }

                        _ => {}
                    },
                    _ => {}
                }
            }

            if dispatch {
                MenuEvent::send(MenuEvent { id });
            }
        }
    }

    if ret == -1 {
        DefSubclassProc(hwnd, msg, wparam, lparam)
    } else {
        ret
    }
}

enum EditCommand {
    Copy,
    Cut,
    Paste,
    SelectAll,
}

fn execute_edit_command(command: EditCommand) {
    let key = match command {
        EditCommand::Copy => 0x43,      // c
        EditCommand::Cut => 0x58,       // x
        EditCommand::Paste => 0x56,     // v
        EditCommand::SelectAll => 0x41, // a
    };

    unsafe {
        let mut inputs: [INPUT; 4] = std::mem::zeroed();
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki.wVk = VK_CONTROL;
        inputs[2].Anonymous.ki.dwFlags = 0;

        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki.wVk = key;
        inputs[2].Anonymous.ki.dwFlags = 0;

        inputs[2].r#type = INPUT_KEYBOARD;
        inputs[2].Anonymous.ki.wVk = key;
        inputs[2].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

        inputs[3].r#type = INPUT_KEYBOARD;
        inputs[3].Anonymous.ki.wVk = VK_CONTROL;
        inputs[3].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

        SendInput(4, &inputs as *const _, std::mem::size_of::<INPUT>() as _);
    }
}

fn show_about_dialog(hwnd: HWND, metadata: &AboutMetadata) {
    use std::fmt::Write;

    let mut message = String::new();
    if let Some(name) = &metadata.name {
        let _ = writeln!(&mut message, "Name: {}", name);
    }
    if let Some(version) = &metadata.full_version() {
        let _ = writeln!(&mut message, "Version: {}", version);
    }
    if let Some(authors) = &metadata.authors {
        let _ = writeln!(&mut message, "Authors: {}", authors.join(", "));
    }
    if let Some(license) = &metadata.license {
        let _ = writeln!(&mut message, "License: {}", license);
    }
    match (&metadata.website_label, &metadata.website) {
        (Some(label), None) => {
            let _ = writeln!(&mut message, "Website: {}", label);
        }
        (None, Some(url)) => {
            let _ = writeln!(&mut message, "Website: {}", url);
        }
        (Some(label), Some(url)) => {
            let _ = writeln!(&mut message, "Website: {} {}", label, url);
        }
        _ => {}
    }
    if let Some(comments) = &metadata.comments {
        let _ = writeln!(&mut message, "\n{}", comments);
    }
    if let Some(copyright) = &metadata.copyright {
        let _ = writeln!(&mut message, "\n{}", copyright);
    }

    let message = encode_wide(message);
    let title = encode_wide(format!(
        "About {}",
        metadata.name.as_deref().unwrap_or_default()
    ));

    #[cfg(not(feature = "common-controls-v6"))]
    std::thread::spawn(move || unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION};
        MessageBoxW(hwnd, message.as_ptr(), title.as_ptr(), MB_ICONINFORMATION);
    });

    #[cfg(feature = "common-controls-v6")]
    {
        use windows_sys::Win32::UI::Controls::{
            TaskDialogIndirect, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TASKDIALOGCONFIG_1,
            TDCBF_OK_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION, TD_INFORMATION_ICON,
        };

        std::thread::spawn(move || unsafe {
            let task_dialog_config = TASKDIALOGCONFIG {
                cbSize: core::mem::size_of::<TASKDIALOGCONFIG>() as u32,
                hwndParent: hwnd,
                dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
                pszWindowTitle: title.as_ptr(),
                pszContent: message.as_ptr(),
                Anonymous1: TASKDIALOGCONFIG_0 {
                    pszMainIcon: TD_INFORMATION_ICON,
                },
                Anonymous2: TASKDIALOGCONFIG_1 {
                    pszFooterIcon: std::ptr::null(),
                },
                dwCommonButtons: TDCBF_OK_BUTTON,
                pButtons: std::ptr::null(),
                cButtons: 0,
                pRadioButtons: std::ptr::null(),
                cRadioButtons: 0,
                cxWidth: 0,
                hInstance: 0,
                pfCallback: None,
                lpCallbackData: 0,
                nDefaultButton: 0,
                nDefaultRadioButton: 0,
                pszCollapsedControlText: std::ptr::null(),
                pszExpandedControlText: std::ptr::null(),
                pszExpandedInformation: std::ptr::null(),
                pszMainInstruction: std::ptr::null(),
                pszVerificationText: std::ptr::null(),
                pszFooter: std::ptr::null(),
            };

            let mut pf_verification_flag_checked = 0;
            let mut pn_button = 0;
            let mut pn_radio_button = 0;

            TaskDialogIndirect(
                &task_dialog_config,
                &mut pn_button,
                &mut pn_radio_button,
                &mut pf_verification_flag_checked,
            )
        });
    }
}
