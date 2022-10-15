mod accelerator;
mod util;

use crate::{
    accelerator::Accelerator,
    internal::MenuItemType,
    predefined::PredfinedMenuItemType,
    util::{AddOp, Counter},
};
use std::{cell::RefCell, fmt::Debug, rc::Rc};
use util::{decode_wide, encode_wide, Accel};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    Graphics::Gdi::ClientToScreen,
    UI::{
        Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, VK_CONTROL},
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            AppendMenuW, CreateAcceleratorTableW, CreateMenu, CreatePopupMenu,
            DestroyAcceleratorTable, DestroyWindow, DrawMenuBar, EnableMenuItem, GetMenuItemInfoW,
            InsertMenuW, MessageBoxW, PostQuitMessage, RemoveMenu, SendMessageW, SetMenu,
            SetMenuItemInfoW, ShowWindow, TrackPopupMenu, HACCEL, HMENU, MB_ICONINFORMATION,
            MENUITEMINFOW, MFS_CHECKED, MFS_DISABLED, MF_BYCOMMAND, MF_BYPOSITION, MF_CHECKED,
            MF_DISABLED, MF_ENABLED, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
            MIIM_STATE, MIIM_STRING, SW_MINIMIZE, TPM_LEFTALIGN, WM_COMMAND, WM_DESTROY,
        },
    },
};

const COUNTER_START: u32 = 1000;
static COUNTER: Counter = Counter::new_with_start(COUNTER_START);

/// A generic child in a menu
///
/// Be careful when cloning this item and treat it as read-only
#[derive(Debug, Default)]
struct MenuChild {
    // shared fields between submenus and menu items
    type_: MenuItemType,
    text: String,
    enabled: bool,
    parents_hemnu: Vec<HMENU>,

    // menu item fields
    id: u32,
    accelerator: Option<Accelerator>,

    // predefined menu item fields
    predefined_item_type: PredfinedMenuItemType,

    // check menu item fields
    checked: bool,

    // submenu fields
    hmenu: HMENU,
    hpopupmenu: HMENU,
    children: Option<Vec<Rc<RefCell<MenuChild>>>>,
    root_menu_haccel: Option<Vec<Rc<RefCell<(HACCEL, Vec<Accel>)>>>>,
}

impl MenuChild {
    fn id(&self) -> u32 {
        match self.type_ {
            MenuItemType::Submenu => self.hmenu as u32,
            _ => self.id,
        }
    }
    fn text(&self) -> String {
        self.parents_hemnu
            .get(0)
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
            .unwrap_or(self.text.clone())
    }

    fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        for parent in &self.parents_hemnu {
            let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
            info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
            info.fMask = MIIM_STRING;
            info.dwTypeData = encode_wide(text).as_mut_ptr();

            unsafe { SetMenuItemInfoW(*parent, self.id(), false.into(), &info) };
        }
    }

    fn is_enabled(&self) -> bool {
        self.parents_hemnu
            .get(0)
            .map(|hmenu| {
                let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
                info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
                info.fMask = MIIM_STATE;

                unsafe { GetMenuItemInfoW(*hmenu, self.id(), false.into(), &mut info) };

                (info.fState & MFS_DISABLED) == 0
            })
            .unwrap_or(self.enabled)
    }

    fn set_enabled(&mut self, enabled: bool) {
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

    fn is_checked(&self) -> bool {
        self.parents_hemnu
            .get(0)
            .map(|hmenu| {
                let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
                info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
                info.fMask = MIIM_STATE;

                unsafe { GetMenuItemInfoW(*hmenu, self.id(), false.into(), &mut info) };

                !((info.fState & MFS_CHECKED) == 0)
            })
            .unwrap_or(self.enabled)
    }

    fn set_checked(&mut self, checked: bool) {
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

#[derive(Clone)]
pub(crate) struct Menu {
    hmenu: HMENU,
    hpopupmenu: HMENU,
    hwnds: Rc<RefCell<Vec<HWND>>>,
    haccel: Rc<RefCell<(HACCEL, Vec<Accel>)>>,
    children: Rc<RefCell<Vec<Rc<RefCell<MenuChild>>>>>,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            hmenu: unsafe { CreateMenu() },
            hpopupmenu: unsafe { CreatePopupMenu() },
            haccel: Rc::new(RefCell::new((0, Vec::new()))),
            children: Rc::new(RefCell::new(Vec::new())),
            hwnds: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn append(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Append)
    }

    pub fn prepend(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Insert(0))
    }

    pub fn insert(&self, item: &dyn crate::MenuEntry, position: usize) {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    fn add_menu_item(&self, item: &dyn crate::MenuEntry, op: AddOp) {
        let mut flags = 0;
        let child = match item.type_() {
            MenuItemType::Submenu => {
                let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                let child = &submenu.0 .0;

                flags |= MF_POPUP;
                child
                    .borrow_mut()
                    .root_menu_haccel
                    .as_mut()
                    .unwrap()
                    .push(self.haccel.clone());

                child
            }
            MenuItemType::Normal => {
                let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                let child = &item.0 .0;

                flags |= MF_STRING;

                child
            }
            MenuItemType::Predefined => {
                let item = item
                    .as_any()
                    .downcast_ref::<crate::PredefinedMenuItem>()
                    .unwrap();
                let child = &item.0 .0;

                let child_ = child.borrow();

                match child_.predefined_item_type {
                    PredfinedMenuItemType::None => return,
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
                let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                let child = &item.0 .0;

                flags |= MF_STRING;
                if child.borrow().checked {
                    flags |= MF_CHECKED;
                }

                child
            }
        }
        .clone();

        {
            let child_ = child.borrow();
            if !child_.enabled {
                flags |= MF_GRAYED;
            }

            let mut text = child_.text.clone();

            if let Some(accelerator) = &child_.accelerator {
                let accel_str = accelerator.to_string();
                let accel = accelerator.to_accel(child_.id() as u16);

                text.push_str("\t");
                text.push_str(&accel_str);

                let mut haccel = self.haccel.borrow_mut();
                haccel.1.push(Accel(accel));
                let accels = haccel.1.clone();
                update_haccel(&mut haccel.0, accels)
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
            let mut child_ = child.borrow_mut();
            child_.parents_hemnu.push(self.hmenu);
            child_.parents_hemnu.push(self.hpopupmenu);
        }
        {
            let mut children = self.children.borrow_mut();
            match op {
                AddOp::Append => children.push(child),
                AddOp::Insert(position) => children.insert(position, child),
            }
        }
    }

    pub fn remove(&self, item: &dyn crate::MenuEntry) {
        unsafe {
            // TODO: remove self.hmenu and self.hpopupmenu from item.parents_hmenu
            RemoveMenu(self.hmenu, item.id(), MF_BYCOMMAND);
            RemoveMenu(self.hpopupmenu, item.id(), MF_BYCOMMAND);

            for hwnd in self.hwnds.borrow().iter() {
                DrawMenuBar(*hwnd);
            }
        }

        let mut children = self.children.borrow_mut();
        let index = children
            .iter()
            .position(|e| e.borrow().id() == item.id())
            .unwrap();
        children.remove(index);
    }

    pub fn items(&self) -> Vec<Box<dyn crate::MenuEntry>> {
        self.children
            .borrow()
            .iter()
            .map(|c| -> Box<dyn crate::MenuEntry> {
                let child = c.borrow();
                match child.type_ {
                    MenuItemType::Submenu => Box::new(crate::Submenu(Submenu(c.clone()))),
                    MenuItemType::Normal => Box::new(crate::MenuItem(MenuItem(c.clone()))),
                    MenuItemType::Predefined => {
                        Box::new(crate::PredefinedMenuItem(PredefinedMenuItem(c.clone())))
                    }
                    MenuItemType::Check => Box::new(crate::CheckMenuItem(CheckMenuItem(c.clone()))),
                }
            })
            .collect()
    }

    fn find_by_id(&self, id: u32) -> Option<Rc<RefCell<MenuChild>>> {
        let children = self.children.borrow();
        for i in children.iter() {
            let item = i.borrow();
            if item.id == id {
                return Some(i.clone());
            }

            if item.type_ == MenuItemType::Submenu {
                let submenu = Submenu(i.clone());
                if let Some(child) = submenu.find_by_id(id) {
                    return Some(child);
                }
            }
        }
        None
    }

    pub fn haccel(&self) -> HACCEL {
        self.haccel.borrow().0
    }

    pub fn init_for_hwnd(&self, hwnd: isize) {
        self.hwnds.borrow_mut().push(hwnd);
        unsafe {
            SetMenu(hwnd, self.hmenu);
            SetWindowSubclass(
                hwnd,
                Some(menu_subclass_proc),
                MENU_SUBCLASS_ID,
                Box::into_raw(Box::new(self.clone())) as _,
            );
            DrawMenuBar(hwnd);
        };
    }

    pub fn remove_for_hwnd(&self, hwnd: isize) {
        let mut hwnds = self.hwnds.borrow_mut();
        let index = hwnds.iter().position(|h| *h == hwnd).unwrap();
        hwnds.remove(index);
        unsafe {
            SendMessageW(hwnd, WM_CLEAR_MENU_DATA, 0, 0);
            RemoveWindowSubclass(hwnd, Some(menu_subclass_proc), MENU_SUBCLASS_ID);
            SetMenu(hwnd, 0);
            DrawMenuBar(hwnd);
        }
    }

    pub fn hide_for_hwnd(&self, hwnd: isize) {
        unsafe {
            SetMenu(hwnd, 0);
            DrawMenuBar(hwnd);
        }
    }

    pub fn show_for_hwnd(&self, hwnd: isize) {
        unsafe {
            SetMenu(hwnd, self.hmenu);
            DrawMenuBar(hwnd);
        }
    }

    pub fn show_context_menu_for_hwnd(&self, hwnd: isize, x: f64, y: f64) {
        show_context_menu(hwnd, self.hpopupmenu, x, y)
    }
}

#[derive(Clone)]
pub(crate) struct Submenu(Rc<RefCell<MenuChild>>);

impl Submenu {
    pub fn new(text: &str, enabled: bool) -> Self {
        Self(Rc::new(RefCell::new(MenuChild {
            type_: MenuItemType::Submenu,
            text: text.to_string(),
            enabled,
            parents_hemnu: Vec::new(),
            children: Some(Vec::new()),
            hmenu: unsafe { CreateMenu() },
            hpopupmenu: unsafe { CreatePopupMenu() },
            root_menu_haccel: Some(Vec::new()),
            ..Default::default()
        })))
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }

    pub fn append(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Append)
    }

    pub fn prepend(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Insert(0))
    }

    pub fn insert(&self, item: &dyn crate::MenuEntry, position: usize) {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    fn add_menu_item(&self, item: &dyn crate::MenuEntry, op: AddOp) {
        let mut flags = 0;
        let child = match item.type_() {
            MenuItemType::Submenu => {
                let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                let child = &submenu.0 .0;

                flags |= MF_POPUP;

                child
                    .borrow_mut()
                    .root_menu_haccel
                    .as_mut()
                    .unwrap()
                    .extend_from_slice(self.0.borrow_mut().root_menu_haccel.as_ref().unwrap());

                child
            }
            MenuItemType::Normal => {
                let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                let child = &item.0 .0;

                flags |= MF_STRING;

                child
            }

            MenuItemType::Predefined => {
                let item = item
                    .as_any()
                    .downcast_ref::<crate::PredefinedMenuItem>()
                    .unwrap();
                let child = &item.0 .0;

                let child_ = child.borrow();

                match child_.predefined_item_type {
                    PredfinedMenuItemType::None => return,
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
                let item = item
                    .as_any()
                    .downcast_ref::<crate::CheckMenuItem>()
                    .unwrap();
                let child = &item.0 .0;

                flags |= MF_STRING;
                if child.borrow().checked {
                    flags |= MF_CHECKED;
                }

                child
            }
        }
        .clone();

        {
            let mut self_ = self.0.borrow_mut();

            let child_ = child.borrow();
            if !child_.enabled {
                flags |= MF_GRAYED;
            }

            let mut text = child_.text.clone();

            if let Some(accelerator) = &child_.accelerator {
                let accel_str = accelerator.to_string();
                let accel = accelerator.to_accel(child_.id() as u16);

                text.push_str("\t");
                text.push_str(&accel_str);

                for root_menu in self_.root_menu_haccel.as_mut().unwrap() {
                    let mut haccel = root_menu.borrow_mut();
                    haccel.1.push(Accel(accel));
                    let accels = haccel.1.clone();
                    update_haccel(&mut haccel.0, accels)
                }
            }

            let id = child_.id() as usize;
            let text = encode_wide(text);
            unsafe {
                match op {
                    AddOp::Append => {
                        AppendMenuW(self_.hmenu, flags, id, text.as_ptr());
                        AppendMenuW(self_.hpopupmenu, flags, id, text.as_ptr());
                    }
                    AddOp::Insert(position) => {
                        InsertMenuW(
                            self_.hmenu,
                            position as _,
                            flags | MF_BYPOSITION,
                            id,
                            text.as_ptr(),
                        );
                        InsertMenuW(
                            self_.hpopupmenu,
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
            let self_ = self.0.borrow();
            let mut child_ = child.borrow_mut();
            child_.parents_hemnu.push(self_.hmenu);
            child_.parents_hemnu.push(self_.hpopupmenu);
        }
        {
            let mut self_ = self.0.borrow_mut();
            let children = self_.children.as_mut().unwrap();
            match op {
                AddOp::Append => children.push(child),
                AddOp::Insert(position) => children.insert(position, child),
            }
        }
    }

    pub fn remove(&self, item: &dyn crate::MenuEntry) {
        unsafe {
            // TODO: remove self.hmenu and self.hpopupmenu from item.parents_hmenu
            RemoveMenu(self.0.borrow().hmenu, item.id(), MF_BYCOMMAND);
            RemoveMenu(self.0.borrow().hpopupmenu, item.id(), MF_BYCOMMAND);
        }

        let mut self_ = self.0.borrow_mut();
        let children = self_.children.as_mut().unwrap();
        let index = children
            .iter()
            .position(|e| e.borrow().id() == item.id())
            .unwrap();
        children.remove(index);
    }

    pub fn items(&self) -> Vec<Box<dyn crate::MenuEntry>> {
        self.0
            .borrow()
            .children
            .as_ref()
            .unwrap()
            .iter()
            .map(|c| -> Box<dyn crate::MenuEntry> {
                let child = c.borrow();
                match child.type_ {
                    MenuItemType::Submenu => Box::new(crate::Submenu(Submenu(c.clone()))),
                    MenuItemType::Normal => Box::new(crate::MenuItem(MenuItem(c.clone()))),
                    MenuItemType::Predefined => {
                        Box::new(crate::PredefinedMenuItem(PredefinedMenuItem(c.clone())))
                    }
                    MenuItemType::Check => Box::new(crate::CheckMenuItem(CheckMenuItem(c.clone()))),
                }
            })
            .collect()
    }

    fn find_by_id(&self, id: u32) -> Option<Rc<RefCell<MenuChild>>> {
        let self_ = self.0.borrow();
        let children = self_.children.as_ref().unwrap();
        for i in children.iter() {
            let item = i.borrow();
            if item.id == id {
                return Some(i.clone());
            }

            if item.type_ == MenuItemType::Submenu {
                let submenu = Submenu(i.clone());
                if let Some(child) = submenu.find_by_id(id) {
                    return Some(child);
                }
            }
        }
        None
    }
    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    pub fn set_text(&self, text: &str) {
        self.0.borrow_mut().set_text(text)
    }

    pub fn is_enabled(&self) -> bool {
        self.0.borrow().is_enabled()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.0.borrow_mut().set_enabled(enabled)
    }

    pub fn show_context_menu_for_hwnd(&self, hwnd: isize, x: f64, y: f64) {
        show_context_menu(hwnd, self.0.borrow().hpopupmenu, x, y)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MenuItem(Rc<RefCell<MenuChild>>);

impl MenuItem {
    pub fn new(text: &str, enabled: bool, accelerator: Option<Accelerator>) -> Self {
        Self(Rc::new(RefCell::new(MenuChild {
            type_: MenuItemType::Normal,
            text: text.to_string(),
            enabled,
            parents_hemnu: Vec::new(),
            id: COUNTER.next(),
            accelerator,
            ..Default::default()
        })))
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }

    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    pub fn set_text(&self, text: &str) {
        self.0.borrow_mut().set_text(text)
    }

    pub fn is_enabled(&self) -> bool {
        self.0.borrow().is_enabled()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.0.borrow_mut().set_enabled(enabled)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PredefinedMenuItem(Rc<RefCell<MenuChild>>);

impl PredefinedMenuItem {
    pub fn new(item_type: PredfinedMenuItemType, text: Option<String>) -> Self {
        Self(Rc::new(RefCell::new(MenuChild {
            type_: MenuItemType::Predefined,
            text: text.unwrap_or_else(|| item_type.text().to_string()),
            enabled: true,
            parents_hemnu: Vec::new(),
            id: COUNTER.next(),
            accelerator: item_type.accelerator(),
            predefined_item_type: item_type,
            ..Default::default()
        })))
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }

    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    pub fn set_text(&self, text: &str) {
        self.0.borrow_mut().set_text(text)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CheckMenuItem(Rc<RefCell<MenuChild>>);

impl CheckMenuItem {
    pub fn new(text: &str, enabled: bool, checked: bool, accelerator: Option<Accelerator>) -> Self {
        Self(Rc::new(RefCell::new(MenuChild {
            type_: MenuItemType::Check,
            text: text.to_string(),
            enabled,
            parents_hemnu: Vec::new(),
            id: COUNTER.next(),
            accelerator,
            checked,
            ..Default::default()
        })))
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }

    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    pub fn set_text(&self, text: &str) {
        self.0.borrow_mut().set_text(text)
    }

    pub fn is_enabled(&self) -> bool {
        self.0.borrow().is_enabled()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.0.borrow_mut().set_enabled(enabled)
    }

    pub fn is_checked(&self) -> bool {
        self.0.borrow().is_checked()
    }

    pub fn set_checked(&self, checked: bool) {
        self.0.borrow_mut().set_checked(checked)
    }
}

const MENU_SUBCLASS_ID: usize = 200;
const WM_CLEAR_MENU_DATA: u32 = 600;

unsafe extern "system" fn menu_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uidsubclass: usize,
    dwrefdata: usize,
) -> LRESULT {
    let mut ret = -1;
    if msg == WM_DESTROY || msg == WM_CLEAR_MENU_DATA {
        drop(Box::from_raw(dwrefdata as *mut Menu));
    }

    if msg == WM_COMMAND {
        let id = util::LOWORD(wparam as _) as u32;
        let menu = dwrefdata as *mut Menu;

        if let Some(item) = (*menu).find_by_id(id) {
            ret = 0;

            let mut dispatch = false;

            {
                let mut item = item.borrow_mut();
                match item.type_ {
                    MenuItemType::Normal => {
                        dispatch = true;
                    }
                    MenuItemType::Check => {
                        dispatch = true;

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
                        PredfinedMenuItemType::CloseWindow => {
                            DestroyWindow(hwnd);
                        }
                        PredfinedMenuItemType::Quit => {
                            PostQuitMessage(0);
                        }
                        PredfinedMenuItemType::About(metadata) => {
                            if let Some(metadata) = metadata {
                                MessageBoxW(
                                    hwnd,
                                    encode_wide(format!(
                                        r#"
        {}
        version: {}
        authors: {}
        license: {}
        website: {} {}
        {}
        {}
                                        "#,
                                        metadata.name.as_deref().unwrap_or_default(),
                                        metadata.version.as_deref().unwrap_or_default(),
                                        metadata.authors.as_deref().unwrap_or_default().join(","),
                                        metadata.license.as_deref().unwrap_or_default(),
                                        metadata.website_label.as_deref().unwrap_or_default(),
                                        metadata.website.as_deref().unwrap_or_default(),
                                        metadata.comments.as_deref().unwrap_or_default(),
                                        metadata.copyright.as_deref().unwrap_or_default(),
                                    ))
                                    .as_ptr(),
                                    encode_wide(format!(
                                        "About {}",
                                        metadata.name.as_deref().unwrap_or_default()
                                    ))
                                    .as_ptr(),
                                    MB_ICONINFORMATION,
                                );
                            }
                        }
                        PredfinedMenuItemType::None => unreachable!(),
                    },
                    _ => {}
                }
            }

            if dispatch {
                let _ = crate::MENU_CHANNEL.0.send(crate::MenuEvent { id });
            }
        }
    }

    if ret == -1 {
        DefSubclassProc(hwnd, msg, wparam, lparam)
    } else {
        ret
    }
}

fn update_haccel(haccel: &mut HMENU, accels: Vec<Accel>) {
    unsafe {
        DestroyAcceleratorTable(*haccel);
        *haccel = CreateAcceleratorTableW(
            accels.iter().map(|i| i.0).collect::<Vec<_>>().as_ptr(),
            accels.len() as _,
        );
    }
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
