// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod dark_menu_bar;
mod icon;
mod util;

use self::dark_menu_bar::{WM_UAHDRAWMENU, WM_UAHDRAWMENUITEM};
pub(crate) use self::icon::WinIcon as PlatformIcon;

use crate::{
    accelerator::MenuAccelerator,
    dpi::Position,
    items::{ClickAction, IconType, PredefinedMenuItemType},
    platform_impl::PlatformAttachArgs,
    util::{AddOp, Counter},
    AboutMetadata, MenuEvent, MenuTheme, NativeIcon,
};

use std::{cell::RefCell, collections::HashMap, rc::Rc};
use windows_sys::Win32::{
    Foundation::{FALSE, HWND, LPARAM, LRESULT, POINT, WPARAM},
    Graphics::Gdi::*,
    UI::{
        Input::KeyboardAndMouse::*,
        Shell::{self as shell, *},
        WindowsAndMessaging::*,
    },
};

/// Type alias for a window handle (HWND) in Windows.
type Hwnd = isize;

/// Internal command ids. Used for the `WM_COMMAND` message to identify which menu item was clicked.
static COUNTER: Counter = Counter::new_with_start(1000);

/// The accelerator table for a menu, which is shared by all windows that have the menu attached.
/// and also by all submenus and items of the menu so that they can add and remove their own accelerators.
struct AcceleratorTable {
    handle: HACCEL,
    entries: HashMap<u32, ACCEL>,
}

impl AcceleratorTable {
    fn add(&mut self, id: u32, accelerator: &MenuAccelerator) -> crate::Result<()> {
        let accel = accelerator.to_accel(id as _)?;
        self.entries.insert(id, accel);
        self.rebuild();
        Ok(())
    }

    fn remove(&mut self, id: u32) {
        if self.entries.remove(&id).is_some() {
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        unsafe {
            DestroyAcceleratorTable(self.handle);
            let len = self.entries.len();
            let accels = self.entries.values().collect::<Vec<_>>();
            self.handle = CreateAcceleratorTableW(*accels.as_ptr(), len as _);
        }
    }
}

/// The state of a window that has a menu attached.
struct WindowState {
    theme: MenuTheme,
}

/// The parent menu of a menu item. A menu item can have multiple parents if it is attached to multiple menus.
struct ParentMenu {
    hmenu: HMENU,
    /// The windows that have this menu attached. Only items that are directly attached to a root menu have this field set.
    /// This is used to redraw the menu bar when the item is updated.
    windows: Option<Rc<RefCell<HashMap<Hwnd, WindowState>>>>,
}

/// A root menu bar that can be attached to a window.
pub(crate) struct PlatformMenu {
    id: u32,
    hmenu: HMENU,
    hpopupmenu: HMENU,
    windows: Rc<RefCell<HashMap<Hwnd, WindowState>>>,
    accelerator_table: Rc<RefCell<AcceleratorTable>>,
    children: Vec<Rc<RefCell<PlatformMenuItem>>>,
}

impl Drop for PlatformMenu {
    fn drop(&mut self) {
        // 1. Remove the menu from all windows that have it.
        let windows = self.windows.borrow_mut().drain().collect::<Vec<_>>();
        for (hwnd, _) in windows {
            unsafe { self.remove_from_hwnd(hwnd) };
        }

        // 3. Remove the menu from children's accelerator tables, recursively.
        fn remove_accelerator_table_from_children(
            id: u32,
            children: &[Rc<RefCell<PlatformMenuItem>>],
        ) {
            for child in children {
                let Ok(mut child) = child.try_borrow_mut() else {
                    continue;
                };
                child.accelerator_tables.remove(&id);
                if let Some(children) = &child.children {
                    remove_accelerator_table_from_children(id, children);
                }
            }
        }
        remove_accelerator_table_from_children(self.id, &self.children);

        // 4. Remove the menu items from the menu and popup menu.
        for child in &self.children {
            let Ok(child) = child.try_borrow() else {
                continue;
            };
            let id = child.id();
            unsafe {
                RemoveMenu(self.hpopupmenu, id, MF_BYCOMMAND);
                RemoveMenu(self.hmenu, id, MF_BYCOMMAND);
            }
        }

        // 7. Forget the menu and popup menu handles from the children's parent lists.
        forget_container(self.hmenu, &self.children);
        forget_container(self.hpopupmenu, &self.children);

        // 6. Destroy the menu and popup menu handles.
        unsafe {
            DestroyMenu(self.hmenu);
            DestroyMenu(self.hpopupmenu);
        }
    }
}

/// A menu item that can be attached to a menu or submenu.
pub(crate) struct PlatformMenuItem {
    id: u32,
    /// What a click does.
    click: ClickAction,
    parents: Vec<ParentMenu>,
    accelerator_tables: HashMap<u32, Rc<RefCell<AcceleratorTable>>>,
    // submenu fields
    hmenu: HMENU,
    hpopupmenu: HMENU,
    children: Option<Vec<Rc<RefCell<PlatformMenuItem>>>>,
}

impl Drop for PlatformMenuItem {
    fn drop(&mut self) {
        if let Some(children) = &self.children {
            // 1. Forget the menu and popup menu handles from the children's parent lists.
            forget_container(self.hmenu, children);
            forget_container(self.hpopupmenu, children);

            // 2. Destroy the menu and popup menu handles.
            unsafe {
                DestroyMenu(self.hmenu);
                DestroyMenu(self.hpopupmenu);
            }
        }

        // 3. Remove the item from all accelerator tables it is in.
        for store in self.accelerator_tables.values() {
            if let Ok(mut store) = store.try_borrow_mut() {
                store.remove(self.id())
            }
        }
    }
}

impl PlatformMenu {
    pub fn new() -> Self {
        let id = COUNTER.next();
        Self {
            id,
            hmenu: unsafe { CreateMenu() },
            hpopupmenu: unsafe { CreatePopupMenu() },
            accelerator_table: Rc::new(RefCell::new(AcceleratorTable {
                handle: std::ptr::null_mut(),
                entries: HashMap::new(),
            })),
            children: Vec::new(),
            windows: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn haccel(&self) -> isize {
        self.accelerator_table.borrow().handle as _
    }

    pub fn hpopupmenu(&self) -> isize {
        self.hpopupmenu as _
    }

    pub unsafe fn init_for_hwnd_with_theme(
        &mut self,
        hwnd: isize,
        theme: MenuTheme,
    ) -> crate::Result<()> {
        if self.windows.borrow().contains_key(&hwnd) {
            return Err(crate::Error::AlreadyInitialized);
        }

        self.windows
            .borrow_mut()
            .insert(hwnd, WindowState { theme });

        self.attach_menu_subclass_for_hwnd(hwnd);

        SetMenu(hwnd as _, self.hmenu);
        DrawMenuBar(hwnd as _);

        Ok(())
    }

    pub unsafe fn init_for_hwnd(&mut self, hwnd: isize) -> crate::Result<()> {
        self.init_for_hwnd_with_theme(hwnd, MenuTheme::Auto)
    }

    pub unsafe fn remove_for_hwnd(&mut self, hwnd: isize) -> crate::Result<()> {
        self.windows
            .borrow_mut()
            .remove(&hwnd)
            .ok_or(crate::Error::NotInitialized)?;
        self.remove_from_hwnd(hwnd);
        Ok(())
    }

    unsafe fn remove_from_hwnd(&self, hwnd: isize) {
        self.detach_menu_subclass_from_hwnd(hwnd);
        SetMenu(hwnd as _, std::ptr::null_mut());
        DrawMenuBar(hwnd as _);
    }

    pub unsafe fn attach_menu_subclass_for_hwnd(&self, hwnd: isize) {
        // SAFETY: HWND validity is upheld by caller
        SetWindowSubclass(
            hwnd as _,
            Some(menu_subclass_proc),
            MENU_SUBCLASS_ID,
            self as *const Self as usize,
        );
    }

    pub unsafe fn detach_menu_subclass_from_hwnd(&self, hwnd: isize) {
        // SAFETY: HWND validity is upheld by caller
        RemoveWindowSubclass(hwnd as _, Some(menu_subclass_proc), MENU_SUBCLASS_ID);
    }

    pub unsafe fn hide_for_hwnd(&self, hwnd: isize) -> crate::Result<()> {
        if !self.windows.borrow().contains_key(&hwnd) {
            return Err(crate::Error::NotInitialized);
        }

        // SAFETY: HWND validity is upheld by caller
        SetMenu(hwnd as _, std::ptr::null_mut());
        DrawMenuBar(hwnd as _);

        Ok(())
    }

    pub unsafe fn show_for_hwnd(&self, hwnd: isize) -> crate::Result<()> {
        if !self.windows.borrow().contains_key(&hwnd) {
            return Err(crate::Error::NotInitialized);
        }

        // SAFETY: HWND validity is upheld by caller
        SetMenu(hwnd as _, self.hmenu);
        DrawMenuBar(hwnd as _);

        Ok(())
    }

    pub unsafe fn is_visible_on_hwnd(&self, hwnd: isize) -> bool {
        self.windows.borrow().contains_key(&hwnd)
            // SAFETY: HWND validity is upheld by caller
            && !unsafe { GetMenu(hwnd as _) }.is_null()
    }

    pub unsafe fn show_context_menu(
        &self,
        hwnd: isize,
        position: Option<Position>,
    ) -> Option<Rc<RefCell<PlatformMenuItem>>> {
        show_context_menu(hwnd as _, self.hpopupmenu, position).and_then(|id| self.find_by_id(id))
    }

    pub unsafe fn set_theme_for_hwnd(&self, hwnd: isize, theme: MenuTheme) -> crate::Result<()> {
        if !self.windows.borrow().contains_key(&hwnd) {
            return Err(crate::Error::NotInitialized);
        }

        // SAFETY: HWND validity is upheld by caller
        SendMessageW(hwnd as _, MENU_UPDATE_THEME, 0, theme as _);

        Ok(())
    }

    pub fn attach(
        &mut self,
        args: &PlatformAttachArgs,
        child: Rc<RefCell<PlatformMenuItem>>,
        op: AddOp,
    ) -> crate::Result<()> {
        attach_item(
            self.hmenu,
            self.hpopupmenu,
            Some(self.windows.clone()),
            [(self.id, self.accelerator_table.clone())],
            args,
            &child,
            op,
        )?;

        match op {
            AddOp::Append => self.children.push(child),
            AddOp::Insert(position) => self.children.insert(position, child),
        }

        Ok(())
    }

    pub fn remove_at(&mut self, position: usize) {
        if position >= self.children.len() {
            return;
        }

        let child = self.children.remove(position);
        let mut child = child.borrow_mut();
        let id = child.id();

        unsafe {
            RemoveMenu(self.hmenu, id, MF_BYCOMMAND);
            RemoveMenu(self.hpopupmenu, id, MF_BYCOMMAND);
        }

        self.redraw_menu_bars();

        child
            .parents
            .retain(|parent| parent.hmenu != self.hmenu && parent.hmenu != self.hpopupmenu);
    }

    fn find_by_id(&self, id: u32) -> Option<Rc<RefCell<PlatformMenuItem>>> {
        find_by_id(id, &self.children)
    }

    fn redraw_menu_bars(&self) {
        for hwnd in self.windows.borrow().keys() {
            unsafe { DrawMenuBar(*hwnd as _) };
        }
    }
}

impl PlatformMenuItem {
    pub fn new(click: ClickAction) -> Self {
        Self {
            id: COUNTER.next(),
            click,
            parents: Vec::new(),
            accelerator_tables: HashMap::new(),
            hmenu: std::ptr::null_mut(),
            hpopupmenu: std::ptr::null_mut(),
            children: None,
        }
    }

    pub fn new_submenu(click: ClickAction) -> Self {
        Self {
            id: COUNTER.next(),
            click,
            parents: Vec::new(),
            accelerator_tables: HashMap::new(),
            hmenu: unsafe { CreateMenu() },
            hpopupmenu: unsafe { CreatePopupMenu() },
            children: Some(Vec::new()),
        }
    }

    /// How Win32 addresses this item inside a container.
    ///
    /// A submenu is addressed by the container it owns rather than by its command id: that is
    /// what `MF_POPUP` inserts, so it is what `MF_BYCOMMAND` matches.
    fn id(&self) -> u32 {
        if self.hmenu.is_null() {
            self.id
        } else {
            self.hmenu as u32
        }
    }
}

/// Shared item properties
impl PlatformMenuItem {
    pub fn text(&self) -> Option<String> {
        let parent = self.parents.first()?;
        let id = self.id();

        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        info.fMask = MIIM_STRING;

        if unsafe { GetMenuItemInfoW(parent.hmenu, id, FALSE, &mut info) } == 0 {
            return None;
        }

        info.cch += 1;
        let mut dw_type_data = Vec::with_capacity(info.cch as usize);
        info.dwTypeData = dw_type_data.as_mut_ptr();

        if unsafe { GetMenuItemInfoW(parent.hmenu, id, FALSE, &mut info) } == 0 {
            return None;
        }

        let text = util::decode_wide(info.dwTypeData);

        // The label is the text before the tab character, if any, which is where the accelerator is appended.
        Some(text.split('\t').next().unwrap().to_string())
    }

    pub fn set_text(&mut self, text: &str, accelerator: Option<&MenuAccelerator>) {
        let mut text = match accelerator {
            Some(accelerator) => util::encode_wide(format!("{text}\t{accelerator}")),
            None => util::encode_wide(text),
        };

        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        info.fMask = MIIM_STRING;
        info.dwTypeData = text.as_mut_ptr();

        for parent in &self.parents {
            unsafe { SetMenuItemInfoW(parent.hmenu, self.id(), FALSE, &info) };
        }

        self.redraw_menu_bars();
    }

    fn state(&self) -> Option<u32> {
        let parent = self.parents.first()?;

        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
        info.fMask = MIIM_STATE;

        if unsafe { GetMenuItemInfoW(parent.hmenu, self.id(), FALSE, &mut info) } == 0 {
            return None;
        }

        Some(info.fState)
    }

    pub fn is_enabled(&self) -> Option<bool> {
        let state = self.state()?;
        Some((state & MFS_DISABLED) == 0)
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        let flag = if enabled { MF_ENABLED } else { MF_DISABLED };

        for parent in &self.parents {
            unsafe { EnableMenuItem(parent.hmenu, self.id(), flag) };
        }

        self.redraw_menu_bars();
    }

    pub fn is_checked(&self) -> Option<bool> {
        let state = self.state()?;
        Some((state & MFS_CHECKED) != 0)
    }

    pub fn set_checked(&mut self, checked: bool) {
        use windows_sys::Win32::UI::WindowsAndMessaging;

        let flag = if checked { MF_CHECKED } else { MF_UNCHECKED };

        for parent in &self.parents {
            unsafe { WindowsAndMessaging::CheckMenuItem(parent.hmenu, self.id(), flag) };
        }

        self.redraw_menu_bars();
    }

    pub fn set_accelerator(
        &mut self,
        text: &str,
        accelerator: Option<&MenuAccelerator>,
    ) -> crate::Result<()> {
        self.set_text(text, accelerator);

        for store in self.accelerator_tables.values() {
            let mut store = store.borrow_mut();

            match accelerator {
                Some(accelerator) => store.add(self.id(), accelerator)?,
                None => store.remove(self.id()),
            }
        }

        Ok(())
    }

    fn redraw_menu_bars(&self) {
        for parent in &self.parents {
            if let Some(windows) = &parent.windows {
                for hwnd in windows.borrow().keys() {
                    unsafe { DrawMenuBar(*hwnd as _) };
                }
            }
        }
    }
}

/// Icons
impl PlatformMenuItem {
    pub fn set_icon(&mut self, icon: Option<&IconType>) {
        let hbitmap = self.hbitmap(icon);
        let info = create_icon_item_info(hbitmap);

        for parent in &self.parents {
            unsafe { SetMenuItemInfoW(parent.hmenu, self.id(), FALSE, &info) };
        }

        self.redraw_menu_bars();
    }

    fn hbitmap(&self, icon: Option<&IconType>) -> HBITMAP {
        match icon {
            Some(IconType::Custom(icon)) => unsafe { icon.inner.to_hbitmap() },
            Some(IconType::Native(icon)) => native_icon_hbitmap(icon),
            None => std::ptr::null_mut(),
        }
    }
}

impl PlatformMenuItem {
    pub fn attach(
        &mut self,
        args: &PlatformAttachArgs,
        child: Rc<RefCell<PlatformMenuItem>>,
        op: AddOp,
    ) -> crate::Result<()> {
        attach_item(
            self.hmenu,
            self.hpopupmenu,
            None,
            self.accelerator_tables
                .iter()
                .map(|(&id, table)| (id, table.clone())),
            args,
            &child,
            op,
        )?;

        // SAFETY: this method is only called from Submenu item
        let children = self.children.as_mut().unwrap();
        match op {
            AddOp::Append => children.push(child),
            AddOp::Insert(position) => children.insert(position, child),
        }

        Ok(())
    }

    pub fn remove_at(&mut self, position: usize) {
        let Some(children) = self.children.as_mut() else {
            return;
        };
        if position >= children.len() {
            return;
        }

        let child = children.remove(position);
        let mut child = child.borrow_mut();
        let id = child.id();

        unsafe {
            RemoveMenu(self.hmenu, id, MF_BYCOMMAND);
            RemoveMenu(self.hpopupmenu, id, MF_BYCOMMAND);
        }

        child
            .parents
            .retain(|parent| parent.hmenu != self.hmenu && parent.hmenu != self.hpopupmenu);
    }

    fn find_by_id(&self, id: u32) -> Option<Rc<RefCell<PlatformMenuItem>>> {
        find_by_id(id, self.children.as_deref().unwrap_or_default())
    }

    pub fn hpopupmenu(&self) -> isize {
        self.hpopupmenu as _
    }

    pub unsafe fn show_context_menu(
        &self,
        hwnd: isize,
        position: Option<Position>,
    ) -> Option<Rc<RefCell<PlatformMenuItem>>> {
        show_context_menu(hwnd as _, self.hpopupmenu, position).and_then(|id| self.find_by_id(id))
    }

    pub unsafe fn attach_menu_subclass_for_hwnd(&self, hwnd: isize) {
        // SAFETY: HWND validity is upheld by caller
        SetWindowSubclass(
            hwnd as _,
            Some(menu_subclass_proc),
            SUBMENU_SUBCLASS_ID,
            self as *const Self as usize,
        );
    }

    pub unsafe fn detach_menu_subclass_from_hwnd(&self, hwnd: isize) {
        // SAFETY: HWND validity is upheld by caller
        RemoveWindowSubclass(hwnd as _, Some(menu_subclass_proc), SUBMENU_SUBCLASS_ID);
    }
}

fn attach_item(
    hmenu: HMENU,
    hpopupmenu: HMENU,
    windows: Option<Rc<RefCell<HashMap<Hwnd, WindowState>>>>,
    accelerator_tables: impl IntoIterator<Item = (u32, Rc<RefCell<AcceleratorTable>>)>,
    args: &PlatformAttachArgs,
    child: &Rc<RefCell<PlatformMenuItem>>,
    op: AddOp,
) -> crate::Result<()> {
    let accelerator_tables = accelerator_tables.into_iter().collect::<Vec<_>>();
    let mut child = child.borrow_mut();

    child
        .accelerator_tables
        .extend(accelerator_tables.iter().cloned());

    let id = child.id();
    let mut flags = if args.submenu {
        MF_POPUP
    } else if args.separator {
        MF_SEPARATOR
    } else {
        MF_STRING
    };
    if !args.enabled {
        flags |= MF_GRAYED;
    }
    if args.checked {
        flags |= MF_CHECKED;
    }

    let text = match &args.accelerator {
        Some(accelerator) => util::encode_wide(format!("{}\t{accelerator}", args.text)),
        None => util::encode_wide(&args.text),
    };

    if let Some(accelerator) = &args.accelerator {
        for (_, table) in &accelerator_tables {
            table.borrow_mut().add(id, accelerator)?;
        }
    }

    unsafe {
        insert_into(hmenu, op, flags, id, &text);
        insert_into(hpopupmenu, op, flags, id, &text);
    }

    if args.icon.is_some() {
        let info = create_icon_item_info(child.hbitmap(args.icon.as_ref()));
        unsafe {
            SetMenuItemInfoW(hmenu, id, FALSE, &info);
            SetMenuItemInfoW(hpopupmenu, id, FALSE, &info);
        }
    }

    if let Some(windows) = &windows {
        for hwnd in windows.borrow().keys() {
            unsafe { DrawMenuBar(*hwnd as _) };
        }
    }

    child.parents.push(ParentMenu { hmenu, windows });
    child.parents.push(ParentMenu {
        hmenu: hpopupmenu,
        windows: None,
    });

    Ok(())
}

/// Inserts a menu item into a menu at the specified position, with the specified flags and ID.
unsafe fn insert_into(hmenu: HMENU, op: AddOp, flags: u32, id: u32, text: &[u16]) {
    match op {
        AddOp::Append => {
            AppendMenuW(hmenu, flags, id as usize, text.as_ptr());
        }
        AddOp::Insert(position) => {
            InsertMenuW(
                hmenu,
                position as _,
                flags | MF_BYPOSITION,
                id as usize,
                text.as_ptr(),
            );
        }
    }
}

/// Forgets the specified menu handle from the parent lists of the specified children.
fn forget_container(hmenu: HMENU, children: &[Rc<RefCell<PlatformMenuItem>>]) {
    for child in children {
        if let Ok(mut child) = child.try_borrow_mut() {
            child.parents.retain(|parent| parent.hmenu != hmenu);
        }
    }
}

/// Finds a menu item by its ID in the specified children and their descendants.
fn find_by_id(
    id: u32,
    children: &[Rc<RefCell<PlatformMenuItem>>],
) -> Option<Rc<RefCell<PlatformMenuItem>>> {
    for child in children {
        let item = child.borrow();
        if item.id == id {
            return Some(child.clone());
        }

        if item.children.is_some() {
            if let Some(child) = item.find_by_id(id) {
                return Some(child);
            }
        }
    }
    None
}

/// Shows a context menu at the specified position, or at the current cursor position if no position is specified.
unsafe fn show_context_menu(hwnd: HWND, hmenu: HMENU, position: Option<Position>) -> Option<u32> {
    let pt = if let Some(pos) = position {
        let dpi = util::hwnd_dpi(hwnd);
        let scale_factor = util::dpi_to_scale_factor(dpi);
        let pos = pos.to_physical::<i32>(scale_factor);
        let mut pt = POINT {
            x: pos.x as _,
            y: pos.y as _,
        };
        ClientToScreen(hwnd, &mut pt);
        pt
    } else {
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        pt
    };

    SetForegroundWindow(hwnd);

    let result = TrackPopupMenu(
        hmenu,
        TPM_LEFTALIGN | TPM_RETURNCMD,
        pt.x,
        pt.y,
        0,
        hwnd,
        std::ptr::null(),
    );

    (result > 0).then_some(result.try_into().ok()).flatten()
}

fn create_icon_item_info(hbitmap: HBITMAP) -> MENUITEMINFOW {
    let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
    info.fMask = MIIM_BITMAP;
    info.hbmpItem = hbitmap;
    info
}

fn native_icon_hbitmap(icon: &NativeIcon) -> HBITMAP {
    // Translate the public native icon to the shell stock icon ID expected by
    // SHGetStockIconInfo. Unsupported variants deliberately render as no icon.
    let Some(icon_id) = stock_icon_id(icon) else {
        return std::ptr::null_mut();
    };

    let mut info = SHSTOCKICONINFO {
        cbSize: std::mem::size_of::<SHSTOCKICONINFO>() as _,
        ..Default::default()
    };

    let result = unsafe { SHGetStockIconInfo(icon_id, SHGSI_ICON | SHGSI_SMALLICON, &mut info) };

    if result < 0 || info.hIcon.is_null() {
        return std::ptr::null_mut();
    }

    let icon = PlatformIcon::from_handle(info.hIcon);
    unsafe { icon.to_hbitmap() }
}

fn stock_icon_id(icon: &NativeIcon) -> Option<SHSTOCKICONID> {
    let id = match icon {
        NativeIcon::Advanced | NativeIcon::PreferencesGeneral => shell::SIID_SETTINGS,
        NativeIcon::Caution => shell::SIID_WARNING,
        NativeIcon::Computer => shell::SIID_DESKTOPPC,
        NativeIcon::Everyone
        | NativeIcon::User
        | NativeIcon::UserAccounts
        | NativeIcon::UserGroup
        | NativeIcon::UserGuest => shell::SIID_USERS,
        NativeIcon::Folder => shell::SIID_FOLDER,
        NativeIcon::FolderBurnable => shell::SIID_STUFFEDFOLDER,
        NativeIcon::FolderSmart => shell::SIID_FOLDER,
        NativeIcon::FollowLinkFreestanding => shell::SIID_LINK,
        NativeIcon::Home => shell::SIID_FOLDER,
        NativeIcon::Info => shell::SIID_INFO,
        NativeIcon::InvalidDataFreestanding => shell::SIID_ERROR,
        NativeIcon::LockLocked => shell::SIID_LOCK,
        NativeIcon::LockUnlocked => shell::SIID_KEY,
        NativeIcon::MobileMe => shell::SIID_WORLD,
        NativeIcon::MultipleDocuments => shell::SIID_MIXEDFILES,
        NativeIcon::Network => shell::SIID_MYNETWORK,
        NativeIcon::QuickLook => shell::SIID_FIND,
        NativeIcon::Remove => shell::SIID_DELETE,
        NativeIcon::RevealFreestanding => shell::SIID_FOLDEROPEN,
        NativeIcon::Share => shell::SIID_SHARE,
        NativeIcon::TrashEmpty => shell::SIID_RECYCLER,
        NativeIcon::TrashFull => shell::SIID_RECYCLERFULL,
        NativeIcon::Raw(id) => *id,
        _ => return None,
    };

    (0..shell::SIID_MAX_ICONS).contains(&id).then_some(id)
}

const MENU_SUBCLASS_ID: usize = 200;
const MENU_UPDATE_THEME: u32 = 201;
const SUBMENU_SUBCLASS_ID: usize = 202;

unsafe extern "system" fn menu_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    uidsubclass: usize,
    dwrefdata: usize,
) -> LRESULT {
    match msg {
        MENU_UPDATE_THEME if uidsubclass == MENU_SUBCLASS_ID => {
            let menu = util::cast_mut::<PlatformMenu>(dwrefdata);
            let theme: MenuTheme = std::mem::transmute(lparam);

            let mut windows = menu.windows.borrow_mut();
            windows.insert(hwnd as _, WindowState { theme });

            // Simulate a loss and regain of activation to force the menu bar to redraw with the new theme.
            if GetActiveWindow() == hwnd {
                PostMessageW(hwnd, WM_NCACTIVATE, 0, 0);
                PostMessageW(hwnd, WM_NCACTIVATE, true.into(), 0);
            } else {
                PostMessageW(hwnd, WM_NCACTIVATE, true.into(), 0);
                PostMessageW(hwnd, WM_NCACTIVATE, 0, 0);
            }

            0
        }

        WM_COMMAND => {
            let id = util::LOWORD(wparam as _) as u32;

            let item = match uidsubclass {
                MENU_SUBCLASS_ID => {
                    let menu = util::cast_mut::<PlatformMenu>(dwrefdata);
                    menu.find_by_id(id)
                }
                SUBMENU_SUBCLASS_ID => {
                    let menu = util::cast_mut::<PlatformMenuItem>(dwrefdata);
                    menu.find_by_id(id)
                }
                _ => unreachable!(),
            };

            if let Some(item) = item {
                handle_item_activate(hwnd, &item);
                0
            } else {
                DefSubclassProc(hwnd as _, msg, wparam, lparam)
            }
        }

        WM_UAHDRAWMENUITEM | WM_UAHDRAWMENU if uidsubclass == MENU_SUBCLASS_ID => {
            let menu = util::cast_mut::<PlatformMenu>(dwrefdata);
            let windows = menu.windows.borrow();

            let theme = windows.get(&(hwnd as _)).map(|state| state.theme);
            let theme = theme.unwrap_or(MenuTheme::Auto);

            if theme.should_use_dark(hwnd as _) {
                dark_menu_bar::draw(hwnd as _, msg, wparam, lparam);
                0
            } else {
                DefSubclassProc(hwnd as _, msg, wparam, lparam)
            }
        }
        WM_NCACTIVATE | WM_NCPAINT => {
            // DefSubclassProc needs to be called before calling the
            // custom dark menu redraw
            let res = DefSubclassProc(hwnd as _, msg, wparam, lparam);

            let menu = util::cast_mut::<PlatformMenu>(dwrefdata);
            let windows = menu.windows.borrow();

            let theme = windows.get(&(hwnd as _)).map(|state| state.theme);
            let theme = theme.unwrap_or(MenuTheme::Auto);

            if theme.should_use_dark(hwnd as _) {
                dark_menu_bar::draw(hwnd as _, msg, wparam, lparam);
            }

            res
        }
        _ => DefSubclassProc(hwnd as _, msg, wparam, lparam),
    }
}

/// Handle a selection returned by [`PlatformMenu::show_context_menu`].
///
/// Split out so that the caller can release its borrow of the container first.
pub(crate) unsafe fn dispatch_selection(
    hwnd: isize,
    item: Option<Rc<RefCell<PlatformMenuItem>>>,
) -> bool {
    match item {
        Some(item) => {
            unsafe { handle_item_activate(hwnd as _, &item) };
            true
        }
        None => false,
    }
}

unsafe fn handle_item_activate(hwnd: HWND, item: &Rc<RefCell<PlatformMenuItem>>) {
    let click = item.borrow().click.clone();

    match click {
        ClickAction::Emit(id) => MenuEvent::send(MenuEvent { id }),
        ClickAction::Toggle(id, state) => {
            if let Some(state) = state.upgrade() {
                let checked = {
                    let mut state = state.borrow_mut();
                    state.checked = !state.checked;
                    state.checked
                };
                item.borrow_mut().set_checked(checked);
            }
            MenuEvent::send(MenuEvent { id });
        }
        ClickAction::Predefined(state) => {
            if let Some(state) = state.upgrade() {
                let item_type = state.borrow().predefined_item_type.clone();
                run_predefined(hwnd, &item_type);
            }
        }
    }
}

/// Carry out what a predefined item does. Predefined items emit no [`MenuEvent`].
unsafe fn run_predefined(hwnd: HWND, item_type: &PredefinedMenuItemType) {
    match item_type {
        PredefinedMenuItemType::Copy => EditCommand::Copy.run(),
        PredefinedMenuItemType::Cut => EditCommand::Cut.run(),
        PredefinedMenuItemType::Paste => EditCommand::Paste.run(),
        PredefinedMenuItemType::SelectAll => EditCommand::SelectAll.run(),
        PredefinedMenuItemType::Undo => EditCommand::Undo.run(),
        PredefinedMenuItemType::Redo => EditCommand::Redo.run(),
        PredefinedMenuItemType::Separator => {}
        PredefinedMenuItemType::Minimize => {
            ShowWindow(hwnd, SW_MINIMIZE);
        }
        PredefinedMenuItemType::Maximize => {
            ShowWindow(hwnd, SW_MAXIMIZE);
        }
        PredefinedMenuItemType::Hide => {
            ShowWindow(hwnd, SW_HIDE);
        }
        PredefinedMenuItemType::CloseWindow => {
            SendMessageW(hwnd, WM_CLOSE, 0, 0);
        }
        PredefinedMenuItemType::Quit => {
            PostQuitMessage(0);
        }
        PredefinedMenuItemType::About(Some(metadata)) => show_about_dialog(hwnd as _, metadata),

        _ => {}
    }
}

impl MenuTheme {
    fn should_use_dark(&self, hwnd: isize) -> bool {
        match self {
            MenuTheme::Dark => true,
            MenuTheme::Auto if dark_menu_bar::should_use_dark_mode(hwnd as _) => true,
            _ => false,
        }
    }
}

enum EditCommand {
    Copy,
    Cut,
    Paste,
    SelectAll,
    Undo,
    Redo,
}

impl EditCommand {
    fn run(&self) {
        let key = match self {
            EditCommand::Copy => VK_C,
            EditCommand::Cut => VK_X,
            EditCommand::Paste => VK_V,
            EditCommand::SelectAll => VK_A,
            EditCommand::Undo => VK_Z,
            EditCommand::Redo => VK_Y,
        };

        unsafe {
            let mut inputs: [INPUT; 4] = std::mem::zeroed();
            inputs[0].r#type = INPUT_KEYBOARD;
            inputs[0].Anonymous.ki.wVk = VK_CONTROL;
            inputs[0].Anonymous.ki.dwFlags = 0;

            inputs[1].r#type = INPUT_KEYBOARD;
            inputs[1].Anonymous.ki.wVk = key;
            inputs[1].Anonymous.ki.dwFlags = 0;

            inputs[2].r#type = INPUT_KEYBOARD;
            inputs[2].Anonymous.ki.wVk = key;
            inputs[2].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

            inputs[3].r#type = INPUT_KEYBOARD;
            inputs[3].Anonymous.ki.wVk = VK_CONTROL;
            inputs[3].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

            SendInput(4, &inputs as *const _, std::mem::size_of::<INPUT>() as _);
        }
    }
}

fn show_about_dialog(hwnd: Hwnd, metadata: &AboutMetadata) {
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

    let message = util::encode_wide(message);
    let title = util::encode_wide(format!(
        "About {}",
        metadata.name.as_deref().unwrap_or_default()
    ));

    #[cfg(not(feature = "common-controls-v6"))]
    std::thread::spawn(move || unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION};
        MessageBoxW(
            hwnd as _,
            message.as_ptr(),
            title.as_ptr(),
            MB_ICONINFORMATION,
        );
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
                hwndParent: hwnd as _,
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
                hInstance: std::ptr::null_mut(),
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
