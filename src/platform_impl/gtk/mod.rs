// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod accelerator;
mod icon;
mod mnemonic;

use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    rc::Rc,
};

use dpi::Position;
use gtk4::{gdk::Rectangle, gio, glib::VariantTy, prelude::*};
pub(crate) use icon::PlatformIcon;
use mnemonic::to_gtk_mnemonic;

use crate::{
    accelerator::Accelerator,
    util::{AddOp, Counter},
    Icon, IsMenuItem, MenuEvent, MenuId, MenuItemKind, MenuItemType, NativeIcon,
    PredefinedMenuItemType,
};

static COUNTER: Counter = Counter::new();

const DEFAULT_ACTION: &str = "_internal_sendEvent";
const DEFAULT_ACTION_GROUP: &str = "muda";
const DEFAULT_DETAILED_ACTION: &str = "muda._internal_sendEvent";
const ACTION_GROUP_DATA_KEY: &str = "mudaActionGroup";

enum GtkMenuBar {
    MenuBar {
        widget: gtk4::PopoverMenuBar,
        menu: gio::Menu,
        app: gtk4::Application,
    },
    ContextMenu {
        widget: gtk4::PopoverMenu,
        menu: gio::Menu,
        app: gtk4::Application,
    },
}

impl GtkMenuBar {
    fn new(app: gtk4::Application) -> Self {
        let menu = gio::Menu::new();
        let widget = gtk4::PopoverMenuBar::from_model(Some(&menu));
        Self::MenuBar { widget, menu, app }
    }

    fn new_context(app: gtk4::Application) -> Self {
        let menu = gio::Menu::new();
        let widget = gtk4::PopoverMenu::from_model(Some(&menu));
        Self::ContextMenu { widget, menu, app }
    }

    fn applicaiton(&self) -> &gtk4::Application {
        match self {
            GtkMenuBar::MenuBar { app, .. } => app,
            GtkMenuBar::ContextMenu { app, .. } => app,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn menu_bar(&self) -> &gtk4::PopoverMenuBar {
        match self {
            GtkMenuBar::MenuBar { widget, .. } => widget,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn context_menu(&self) -> &gtk4::PopoverMenu {
        match self {
            GtkMenuBar::ContextMenu { widget, .. } => widget,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn menu(&self) -> &gio::Menu {
        match self {
            GtkMenuBar::MenuBar { menu, .. } => menu,
            GtkMenuBar::ContextMenu { menu, .. } => menu,
        }
    }
}

pub struct Menu {
    id: MenuId,
    instances: HashMap<u32, GtkMenuBar>,
    ctx_menu_id: u32,
    children: Vec<Rc<RefCell<MenuChild>>>,
}

impl Menu {
    pub fn new(id: Option<MenuId>) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            instances: HashMap::new(),
            ctx_menu_id: COUNTER.next(),
            children: Vec::new(),
        }
    }

    pub fn id(&self) -> &MenuId {
        &self.id
    }

    pub fn add_menu_item(&mut self, item: &dyn IsMenuItem, op: AddOp) -> crate::Result<()> {
        match op {
            AddOp::Append => self.children.push(item.child()),
            AddOp::Insert(i) => self.children.insert(i, item.child()),
        }

        for (menu_id, menu_bar) in &self.instances {
            let gtk_item = item.make_gtk_menu_item(menu_bar.applicaiton(), *menu_id)?;
            match op {
                AddOp::Append => menu_bar.menu().append_item(&gtk_item),
                AddOp::Insert(position) => menu_bar.menu().insert_item(position as i32, &gtk_item),
            }
        }

        Ok(())
    }

    pub fn add_menu_item_with_id(&mut self, item: &dyn IsMenuItem, id: u32) -> crate::Result<()> {
        for (menu_id, menu_bar) in self.instances.iter().filter(|m| *m.0 == id) {
            let gtk_item = item.make_gtk_menu_item(menu_bar.applicaiton(), *menu_id)?;
            menu_bar.menu().append_item(&gtk_item);
        }

        Ok(())
    }

    pub fn remove(&self, item: &dyn IsMenuItem) -> crate::Result<()> {
        todo!()
    }

    pub fn items(&self) -> Vec<MenuItemKind> {
        self.children
            .iter()
            .map(|c| c.borrow().kind(c.clone()))
            .collect()
    }

    pub fn init_for_gtk_window<W, C>(
        &mut self,
        window: &W,
        container: Option<&C>,
    ) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
        W: gtk4::prelude::IsA<gtk4::Widget>,
        C: gtk4::prelude::IsA<gtk4::Widget>,
    {
        let id = window.as_ptr() as u32;

        let Some(app) = window.application() else {
            return Err(crate::Error::GtkWindowWithoutApplication);
        };

        // This is the first time this method has been called on this window
        // so we need to create the menubar
        if let Entry::Vacant(e) = self.instances.entry(id) {
            e.insert(GtkMenuBar::new(app.clone()));
        } else {
            return Err(crate::Error::AlreadyInitialized);
        }

        let action_group = action_group_from_app(&app);
        window.insert_action_group(DEFAULT_ACTION_GROUP, Some(&action_group));

        for item in self.items() {
            self.add_menu_item_with_id(item.as_ref(), id)?;
        }

        let menu_bar = self.instances[&id].menu_bar();

        // add the menubar to the specified widget, otherwise to the window
        if let Some(container) = container {
            if container.type_().name() == "GtkBox" {
                let gtk_box = container.dynamic_cast_ref::<gtk4::Box>().unwrap();
                gtk_box.prepend(menu_bar);
            } else if container.type_().name() == "GtkFixed" {
                let gtk_box = container.dynamic_cast_ref::<gtk4::Fixed>().unwrap();
                gtk_box.put(menu_bar, 0., 0.);
            } else if container.type_().name() == "GtkStack" {
                let gtk_box = container.dynamic_cast_ref::<gtk4::Stack>().unwrap();
                gtk_box.add_child(menu_bar);
            }
        } else {
            window.set_child(Some(menu_bar));
        }

        // show the menu bar
        menu_bar.set_visible(true);

        Ok(())
    }

    pub fn remove_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        todo!()
    }

    pub fn hide_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        todo!()
    }

    pub fn show_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        todo!()
    }

    #[cfg(target_os = "linux")]
    pub fn is_visible_on_gtk_window<W>(&self, window: &W) -> bool
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        todo!()
    }

    pub fn gtk_menubar_for_gtk_window<W>(&self, window: &W) -> Option<gtk4::PopoverMenuBar>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        todo!()
    }

    pub fn show_context_menu_for_gtk_window(
        &mut self,
        window: &gtk4::Window,
        position: Option<Position>,
    ) -> bool {
        let Some(app) = window.application() else {
            return false; // TODO: better error
        };

        if self.instances.get(&self.ctx_menu_id).is_none() {
            let action_group = action_group_from_app(&app);
            window.insert_action_group(DEFAULT_ACTION_GROUP, Some(&action_group));

            let menu = GtkMenuBar::new_context(app);

            self.instances.insert(self.ctx_menu_id, menu);

            for item in self.items() {
                let _ = self.add_menu_item_with_id(item.as_ref(), self.ctx_menu_id);
            }
        }

        let (x, y) = match position {
            Some(p) => p.to_logical::<i32>(window.scale_factor() as _).into(),
            None => get_cursor_pos(window),
        };

        // SAFETY: it is guaranteed to exist due to the check above
        let menu = self.instances.get(&self.ctx_menu_id).unwrap();
        let context_menu = menu.context_menu();

        if context_menu.parent().is_some() {
            context_menu.unparent();
        }
        context_menu.set_parent(window);

        context_menu.popup();
        context_menu.set_pointing_to(Some(&Rectangle::new(x, y, 0, 0)));

        true
    }
}

#[derive(Clone)]
enum GtkMenuChild {
    Item(gio::MenuItem),
    CheckItem {
        item: gio::MenuItem,
        action: gio::SimpleAction,
    },
    Submenu {
        id: u32,
        item: gio::MenuItem,
        menu: gio::Menu,
        app: gtk4::Application,
    },
    ContextMenu {
        id: u32,
        widget: gtk4::PopoverMenu,
        menu: gio::Menu,
        app: gtk4::Application,
    },
}

impl GtkMenuChild {
    fn id(&self) -> u32 {
        match self {
            GtkMenuChild::Submenu { id, .. } => *id,
            GtkMenuChild::ContextMenu { id, .. } => *id,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn application(&self) -> &gtk4::Application {
        match self {
            GtkMenuChild::Submenu { app, .. } => app,
            GtkMenuChild::ContextMenu { app, .. } => app,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn item(&self) -> &gio::MenuItem {
        match self {
            GtkMenuChild::Submenu { item, .. } => item,
            GtkMenuChild::Item(item) => item,
            GtkMenuChild::CheckItem { item, .. } => item,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn action(&self) -> &gio::SimpleAction {
        match self {
            GtkMenuChild::CheckItem { action, .. } => action,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn menu(&self) -> &gio::Menu {
        match self {
            GtkMenuChild::Submenu { menu, .. } => menu,
            GtkMenuChild::ContextMenu { menu, .. } => menu,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn context_menu(&self) -> &gtk4::PopoverMenu {
        match self {
            GtkMenuChild::ContextMenu { widget, .. } => widget,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }
}

pub struct MenuChild {
    id: MenuId,
    text: String,
    enabled: bool,
    accelerator: Option<Accelerator>,

    checked: bool,

    icon: Option<Icon>,

    type_: MenuItemType,

    instances: HashMap<u32, Vec<GtkMenuChild>>,
    ctx_menu_id: u32,
    children: Vec<Rc<RefCell<MenuChild>>>,
}

impl MenuChild {
    pub fn new_submenu(text: &str, enabled: bool, id: Option<MenuId>) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            text: text.to_string(),
            enabled,
            checked: false,
            icon: None,
            accelerator: None,
            type_: MenuItemType::Submenu,
            ctx_menu_id: COUNTER.next(),
            instances: HashMap::new(),
            children: Vec::new(),
        }
    }

    fn create_gtk_item_for_submenu(
        &mut self,
        app: &gtk4::Application,
        menu_id: u32,
    ) -> crate::Result<gio::MenuItem> {
        let menu = gio::Menu::new();
        let item = gio::MenuItem::new_submenu(Some(&to_gtk_mnemonic(&self.text)), &menu);

        let id = COUNTER.next();

        let child = GtkMenuChild::Submenu {
            item: item.clone(),
            menu,
            id,
            app: app.clone(),
        };

        self.instances.entry(menu_id).or_default().push(child);

        for item in self.items() {
            self.add_menu_item_with_id(item.as_ref(), id)?;
        }

        Ok(item)
    }

    pub fn add_menu_item(&mut self, item: &dyn IsMenuItem, op: AddOp) -> crate::Result<()> {
        match op {
            AddOp::Append => self.children.push(item.child()),
            AddOp::Insert(i) => self.children.insert(i, item.child()),
        }

        for menus in self.instances.values() {
            for gtk_child in menus {
                let gtk_item = item.make_gtk_menu_item(gtk_child.application(), gtk_child.id())?;

                match op {
                    AddOp::Append => gtk_child.menu().append_item(&gtk_item),
                    AddOp::Insert(position) => {
                        gtk_child.menu().insert_item(position as i32, &gtk_item)
                    }
                }
            }
        }

        Ok(())
    }

    pub fn add_menu_item_with_id(&self, item: &dyn IsMenuItem, id: u32) -> crate::Result<()> {
        for menus in self.instances.values() {
            for gtk_child in menus.iter().filter(|m| m.id() == id) {
                let gtk_item = item.make_gtk_menu_item(gtk_child.application(), gtk_child.id())?;
                gtk_child.menu().append_item(&gtk_item);
            }
        }

        Ok(())
    }

    pub fn remove(&self, item: &dyn IsMenuItem) -> crate::Result<()> {
        todo!()
    }

    pub fn items(&self) -> Vec<MenuItemKind> {
        self.children
            .iter()
            .map(|c| c.borrow().kind(c.clone()))
            .collect()
    }

    pub fn show_context_menu_for_gtk_window(
        &mut self,
        window: &gtk4::Window,
        position: Option<Position>,
    ) -> bool {
        let Some(app) = window.application() else {
            return false; // TODO: better error
        };

        if self.instances.get(&self.ctx_menu_id).is_none() {
            let menu = gio::Menu::new();
            let widget = gtk4::PopoverMenu::from_model(Some(&menu));

            let action_group = action_group_from_app(&app);
            window.insert_action_group(DEFAULT_ACTION_GROUP, Some(&action_group));

            let menu = GtkMenuChild::ContextMenu {
                id: self.ctx_menu_id,
                widget,
                menu,
                app,
            };

            self.instances.insert(self.ctx_menu_id, vec![menu]);

            for item in self.items() {
                let _ = self.add_menu_item_with_id(item.as_ref(), self.ctx_menu_id);
            }
        }

        // SAFETY: it is guaranteed to exist due to the check above
        let menus = self.instances.get(&self.ctx_menu_id).unwrap();
        let menu = menus.first().unwrap();

        let (x, y) = match position {
            Some(p) => p.to_logical::<i32>(window.scale_factor() as _).into(),
            None => get_cursor_pos(window),
        };

        let context_menu = menu.context_menu();

        if context_menu.parent().is_some() {
            context_menu.unparent();
        }
        context_menu.set_parent(window);

        context_menu.popup();
        context_menu.set_pointing_to(Some(&Rectangle::new(x, y, 0, 0)));

        true
    }
}

impl MenuChild {
    pub fn new_menu_item(
        text: &str,
        enabled: bool,
        accelerator: Option<Accelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            text: text.to_string(),
            enabled,
            accelerator,
            icon: None,
            checked: false,
            type_: MenuItemType::MenuItem,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
        }
    }

    fn create_gtk_item_for_menu_item(
        &mut self,
        app: &gtk4::Application,
        menu_id: u32,
    ) -> crate::Result<gio::MenuItem> {
        let detailed_action = format!("{DEFAULT_DETAILED_ACTION}::{}", self.id.as_ref());
        let item = gio::MenuItem::new(Some(&to_gtk_mnemonic(&self.text)), Some(&detailed_action));

        if let Some(accelerator) = &self.accelerator {
            app.set_accels_for_action(&detailed_action, &[&accelerator.to_gtk()]);
        }

        let child = GtkMenuChild::Item(item.clone());
        self.instances.entry(menu_id).or_default().push(child);

        Ok(item)
    }

    pub fn id(&self) -> &MenuId {
        &self.id
    }

    pub fn item_type(&self) -> &MenuItemType {
        &self.type_
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn set_text(&self, text: &str) {
        todo!()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&self, enabled: bool) {
        todo!()
    }

    pub fn set_accelerator(&self, accelerator: Option<Accelerator>) -> crate::Result<()> {
        todo!()
    }
}

impl MenuChild {
    pub fn new_predefined(item_type: PredefinedMenuItemType, text: Option<String>) -> Self {
        Self {
            id: MenuId(COUNTER.next().to_string()),
            text: text.unwrap_or_else(|| item_type.text().to_string()),
            enabled: true,
            accelerator: None,
            icon: None,
            checked: false,
            type_: MenuItemType::Predefined,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
        }
    }
}

impl MenuChild {
    pub fn new_check(
        text: &str,
        enabled: bool,
        checked: bool,
        accelerator: Option<Accelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            text: text.to_string(),
            enabled,
            accelerator,
            icon: None,
            checked,
            type_: MenuItemType::Check,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
        }
    }

    fn create_gtk_item_for_check_menu_item(
        &mut self,
        app: &gtk4::Application,
        menu_id: u32,
    ) -> crate::Result<gio::MenuItem> {
        let detailed_action = format!("{DEFAULT_ACTION_GROUP}.{}", self.id.as_ref());
        let item = gio::MenuItem::new(Some(&to_gtk_mnemonic(&self.text)), Some(&detailed_action));

        if let Some(accelerator) = &self.accelerator {
            app.set_accels_for_action(&detailed_action, &[&accelerator.to_gtk()]);
        }

        let action_group = action_group_from_app(&app);

        let state = &self.checked.to_variant();
        let action = gio::SimpleAction::new_stateful(self.id.as_ref(), None, state);
        let id = self.id.clone();
        action.connect_state_notify(move |_| {
            MenuEvent::send(MenuEvent { id: id.clone() });
        });
        action_group.add_action(&action);

        let child = GtkMenuChild::CheckItem {
            item: item.clone(),
            action,
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(item)
    }

    pub fn is_checked(&self) -> bool {
        self.instances
            .values()
            .find_map(|i| i.first())
            .and_then(|i| i.action().state())
            .and_then(|s| s.get())
            .unwrap_or(self.checked)
    }

    pub fn set_checked(&self, checked: bool) {
        todo!()
    }
}

impl MenuChild {
    pub fn new_icon(
        text: &str,
        enabled: bool,
        icon: Option<Icon>,
        accelerator: Option<Accelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            text: text.to_string(),
            enabled,
            accelerator,
            icon,
            checked: false,
            type_: MenuItemType::Icon,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
        }
    }

    pub fn new_native_icon(
        text: &str,
        enabled: bool,
        icon: Option<NativeIcon>,
        accelerator: Option<Accelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            text: text.to_string(),
            enabled,
            accelerator,
            icon: None,
            checked: false,
            type_: MenuItemType::Submenu,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
        }
    }

    fn create_gtk_item_for_icon_menu_item(
        &mut self,
        app: &gtk4::Application,
        menu_id: u32,
    ) -> crate::Result<gio::MenuItem> {
        let detailed_action = format!("{DEFAULT_ACTION_GROUP}.{}", self.id.as_ref());
        let item = gio::MenuItem::new(Some(&to_gtk_mnemonic(&self.text)), Some(&detailed_action));

        if let Some(accelerator) = &self.accelerator {
            app.set_accels_for_action(&detailed_action, &[&accelerator.to_gtk()]);
        }

        if let Some(icon) = &self.icon {
            item.set_icon(icon.inner.bytes_icon());
        }

        let child = GtkMenuChild::Item(item.clone());
        self.instances.entry(menu_id).or_default().push(child);

        Ok(item)
    }

    pub fn set_icon(&self, icon: Option<Icon>) {}
}

impl dyn IsMenuItem + '_ {
    fn make_gtk_menu_item(
        &self,
        app: &gtk4::Application,
        menu_id: u32,
    ) -> crate::Result<gio::MenuItem> {
        let kind = self.kind();
        let mut child = kind.child_mut();
        match child.item_type() {
            MenuItemType::Submenu => child.create_gtk_item_for_submenu(app, menu_id),
            MenuItemType::MenuItem => child.create_gtk_item_for_menu_item(app, menu_id),
            MenuItemType::Check => child.create_gtk_item_for_check_menu_item(app, menu_id),
            MenuItemType::Icon => child.create_gtk_item_for_icon_menu_item(app, menu_id),
            _ => todo!(),
            // MenuItemType::Predefined => {
            //     child.create_gtk_item_for_predefined_menu_item(menu_id, action_group)
            // }
        }
    }
}

/// Returns and creates the action group on this applicaiton if necessary.
fn action_group_from_app(app: &gtk4::Application) -> gio::SimpleActionGroup {
    let action_group = unsafe { app.data::<gio::SimpleActionGroup>(ACTION_GROUP_DATA_KEY) };

    let action_group = if let Some(action_group) = action_group {
        unsafe { action_group.as_ref() }.clone()
    } else {
        let action_group = gio::SimpleActionGroup::new();

        let action = gtk4::gio::SimpleAction::new(DEFAULT_ACTION, Some(&VariantTy::STRING));
        action.connect_activate(|_, v| {
            if let Some(v) = v {
                MenuEvent::send(MenuEvent {
                    id: MenuId(v.as_ref().to_string()),
                });
            }
        });
        action_group.add_action(&action);

        unsafe { app.set_data(ACTION_GROUP_DATA_KEY, action_group.clone()) };
        action_group
    };

    action_group
}

fn get_cursor_pos(window: &gtk4::Window) -> (i32, i32) {
    WidgetExt::display(window)
        .default_seat()
        .and_then(|s| s.pointer())
        .map(|p| {
            let (_, x, y) = p.surface_at_position();
            (x as _, y as _)
        })
        .unwrap_or_default()
}
