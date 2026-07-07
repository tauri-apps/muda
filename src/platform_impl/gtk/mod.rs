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
use gtk4::{gdk::Rectangle, gio, prelude::*};
pub(crate) use icon::PlatformIcon;
use mnemonic::to_gtk_mnemonic;

use crate::{
    accelerator::KeyAccelerator,
    util::{AddOp, Counter},
    Icon, IsMenuItem, MenuEvent, MenuId, MenuItemKind, MenuItemType, NativeIcon,
    PredefinedMenuItemType,
};

static COUNTER: Counter = Counter::new();

const DEFAULT_ACTION_GROUP: &str = "muda";
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

    pub fn remove_for_gtk_window<W>(&mut self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
        W: gtk4::prelude::IsA<gtk4::Widget>,
    {
        let id = window.as_ptr() as u32;

        let Some(_menu_bar) = self.instances.remove(&id) else {
            return Err(crate::Error::NotInitialized);
        };

        window.insert_action_group(DEFAULT_ACTION_GROUP, None::<&gio::SimpleActionGroup>);

        // TODO: destroy the menu bar

        Ok(())
    }

    pub fn hide_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        let id = window.as_ptr() as u32;
        let Some(menu_bar) = self.instances.get(&id) else {
            return Err(crate::Error::NotInitialized);
        };
        menu_bar.menu_bar().set_visible(false);
        Ok(())
    }

    pub fn show_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        let id = window.as_ptr() as u32;
        let Some(menu_bar) = self.instances.get(&id) else {
            return Err(crate::Error::NotInitialized);
        };
        menu_bar.menu_bar().set_visible(true);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn is_visible_on_gtk_window<W>(&self, window: &W) -> bool
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        let id = window.as_ptr() as u32;
        self.instances
            .get(&id)
            .map(|m| m.menu_bar().is_visible())
            .unwrap_or(false)
    }

    pub fn gtk_menubar_for_gtk_window<W>(&self, window: &W) -> Option<gtk4::PopoverMenuBar>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        let id = window.as_ptr() as u32;
        self.instances.get(&id).map(|m| m.menu_bar().clone())
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
    Item {
        item: gio::MenuItem,
        app: gtk4::Application,
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
            GtkMenuChild::Item { app, .. } => app,
        }
    }

    fn item(&self) -> &gio::MenuItem {
        match self {
            GtkMenuChild::Submenu { item, .. } => item,
            GtkMenuChild::Item { item, .. } => item,
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
    key_accelerator: Option<KeyAccelerator>,

    checked: bool,

    icon: Option<Icon>,

    type_: MenuItemType,

    instances: HashMap<u32, Vec<GtkMenuChild>>,
    ctx_menu_id: u32,
    children: Vec<Rc<RefCell<MenuChild>>>,

    action: Option<gio::SimpleAction>,
}

impl MenuChild {
    pub fn new_submenu(text: &str, enabled: bool, id: Option<MenuId>) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            text: text.to_string(),
            enabled,
            checked: false,
            icon: None,
            key_accelerator: None,
            type_: MenuItemType::Submenu,
            ctx_menu_id: COUNTER.next(),
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn create_gtk_item_for_submenu(
        &mut self,
        app: &gtk4::Application,
        menu_id: u32,
    ) -> crate::Result<gio::MenuItem> {
        let menu = gio::Menu::new();
        let item = gio::MenuItem::new_submenu(Some(&to_gtk_mnemonic(&self.text)), &menu);
        item.set_detailed_action(&self.detailed_action());

        if self.action.is_none() {
            let action_group = action_group_from_app(&app);

            let action = gio::SimpleAction::new(self.id.as_ref(), None);
            action.connect_activate(|_, _| ());
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

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
    pub fn new(
        text: &str,
        enabled: bool,
        key_accelerator: Option<KeyAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            text: text.to_string(),
            enabled,
            key_accelerator,
            icon: None,
            checked: false,
            type_: MenuItemType::MenuItem,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn create_gtk_item_for_menu_item(
        &mut self,
        app: &gtk4::Application,
        menu_id: u32,
    ) -> crate::Result<gio::MenuItem> {
        let detailed_action = self.detailed_action();
        let item = gio::MenuItem::new(Some(&to_gtk_mnemonic(&self.text)), Some(&detailed_action));

        if let Some(accelerator) = &self.key_accelerator {
            app.set_accels_for_action(&detailed_action, &[&accelerator.to_gtk()]);
        }

        if self.action.is_none() {
            let action_group = action_group_from_app(&app);

            let action = gio::SimpleAction::new(self.id.as_ref(), None);
            let id = self.id.clone();
            action.connect_activate(move |_, _| MenuEvent::send(MenuEvent { id: id.clone() }));
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        let child = GtkMenuChild::Item {
            item: item.clone(),
            app: app.clone(),
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(item)
    }

    pub fn id(&self) -> &MenuId {
        &self.id
    }

    fn detailed_action(&self) -> String {
        format!("{DEFAULT_ACTION_GROUP}.{}", self.id.as_ref())
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

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;

        if let Some(action) = self.action.as_ref() {
            action.set_enabled(enabled);
        }
    }

    pub fn set_key_accelerator(
        &mut self,
        key_accelerator: Option<KeyAccelerator>,
    ) -> crate::Result<()> {
        let detailed_action = self.detailed_action();
        let accelerator = key_accelerator.as_ref().map(|a| a.to_gtk());
        let accelerator = accelerator.as_deref().map(|a| [a]).unwrap_or_default();
        for item in self.instances.values().flat_map(|v| v.iter()) {
            let app = item.application();
            app.set_accels_for_action(&detailed_action, accelerator.as_slice());
        }

        self.key_accelerator = key_accelerator;

        Ok(())
    }
}

impl MenuChild {
    pub fn new_predefined(item_type: PredefinedMenuItemType, text: Option<String>) -> Self {
        Self {
            id: MenuId(COUNTER.next().to_string()),
            text: text.unwrap_or_else(|| item_type.text().to_string()),
            enabled: true,
            key_accelerator: None,
            icon: None,
            checked: false,
            type_: MenuItemType::Predefined,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }
}

impl MenuChild {
    pub fn new_check(
        text: &str,
        enabled: bool,
        checked: bool,
        key_accelerator: Option<KeyAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            text: text.to_string(),
            enabled,
            key_accelerator,
            icon: None,
            checked,
            type_: MenuItemType::Check,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn create_gtk_item_for_check_menu_item(
        &mut self,
        app: &gtk4::Application,
        menu_id: u32,
    ) -> crate::Result<gio::MenuItem> {
        let detailed_action = self.detailed_action();
        let item = gio::MenuItem::new(Some(&to_gtk_mnemonic(&self.text)), Some(&detailed_action));

        if let Some(accelerator) = &self.key_accelerator {
            app.set_accels_for_action(&detailed_action, &[&accelerator.to_gtk()]);
        }

        if self.action.is_none() {
            let action_group = action_group_from_app(&app);

            let state = &self.checked.to_variant();
            let action = gio::SimpleAction::new_stateful(self.id.as_ref(), None, state);
            let id = self.id.clone();
            action.connect_state_notify(move |_| MenuEvent::send(MenuEvent { id: id.clone() }));
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        let child = GtkMenuChild::Item {
            item: item.clone(),
            app: app.clone(),
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(item)
    }

    pub fn is_checked(&self) -> bool {
        self.action
            .as_ref()
            .and_then(|action| action.state())
            .and_then(|s| s.get())
            .unwrap_or(self.checked)
    }

    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;

        if let Some(action) = self.action.as_ref() {
            action.set_state(&checked.to_variant());
        }
    }
}

impl MenuChild {
    pub fn new_icon(
        text: &str,
        enabled: bool,
        icon: Option<Icon>,
        key_accelerator: Option<KeyAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            text: text.to_string(),
            enabled,
            key_accelerator,
            icon,
            checked: false,
            type_: MenuItemType::Icon,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    pub fn new_native_icon(
        text: &str,
        enabled: bool,
        icon: Option<NativeIcon>,
        key_accelerator: Option<KeyAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            text: text.to_string(),
            enabled,
            key_accelerator,
            icon: None,
            checked: false,
            type_: MenuItemType::Submenu,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn create_gtk_item_for_icon_menu_item(
        &mut self,
        app: &gtk4::Application,
        menu_id: u32,
    ) -> crate::Result<gio::MenuItem> {
        let detailed_action = self.detailed_action();
        let item = gio::MenuItem::new(Some(&to_gtk_mnemonic(&self.text)), Some(&detailed_action));

        if let Some(accelerator) = &self.key_accelerator {
            app.set_accels_for_action(&detailed_action, &[&accelerator.to_gtk()]);
        }

        if let Some(icon) = &self.icon {
            item.set_icon(icon.inner.bytes_icon());
        }

        if self.action.is_none() {
            let action_group = action_group_from_app(&app);

            let action = gio::SimpleAction::new(self.id.as_ref(), None);
            let id = self.id.clone();
            action.connect_activate(move |_, _| MenuEvent::send(MenuEvent { id: id.clone() }));
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        let child = GtkMenuChild::Item {
            item: item.clone(),
            app: app.clone(),
        };
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
