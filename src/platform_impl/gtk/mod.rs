// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod accelerator;
mod icon;
mod mnemonic;

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use dpi::Position;
use gtk4::{gdk::Rectangle, gio, prelude::*};
pub(crate) use icon::PlatformIcon;
use mnemonic::to_gtk_mnemonic;

use crate::{
    accelerator::KeyAccelerator,
    util::{AddOp, Counter},
    AboutMetadata, Icon, IsMenuItem, MenuEvent, MenuId, MenuItemKind, MenuItemType, NativeIcon,
    PredefinedMenuItemType,
};

static COUNTER: Counter = Counter::new();

const DEFAULT_ACTION_GROUP: &str = "muda";
const ACTION_GROUP_DATA_KEY: &str = "mudaActionGroup";
const INTERNAL_ID_ATTRIBUTE: &str = "muda-internal-id";

type GtkId = usize;

macro_rules! is_item_supported {
    ($item:expr) => {{
        let child = $item.child();
        let child = child.borrow();
        if let Some(predefined_item_type) = &child.predefined_item_type {
            matches!(
                predefined_item_type,
                PredefinedMenuItemType::Separator
                    | PredefinedMenuItemType::Minimize
                    | PredefinedMenuItemType::Maximize
                    | PredefinedMenuItemType::Fullscreen
                    | PredefinedMenuItemType::Hide
                    | PredefinedMenuItemType::CloseWindow
                    | PredefinedMenuItemType::Quit
                    | PredefinedMenuItemType::About(_)
            )
        } else {
            true
        }
    }};
}

macro_rules! return_if_item_not_supported {
    ($item:expr) => {
        if !is_item_supported!($item) {
            return Ok(());
        }
    };
}

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

    fn custom_widget_host(&self) -> &gtk4::Widget {
        match self {
            GtkMenuBar::MenuBar { widget, .. } => widget.upcast_ref(),
            GtkMenuBar::ContextMenu { widget, .. } => widget.upcast_ref(),
        }
    }
}

pub struct Menu {
    id: MenuId,
    instances: HashMap<GtkId, GtkMenuBar>,
    ctx_menu_id: GtkId,
    children: Vec<Rc<RefCell<MenuChild>>>,
}

impl Menu {
    pub fn new(id: Option<MenuId>) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            instances: HashMap::new(),
            ctx_menu_id: COUNTER.next() as GtkId,
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

        return_if_item_not_supported!(item);

        for (menu_id, menu_bar) in &self.instances {
            let host = menu_bar.custom_widget_host();
            let app = menu_bar.applicaiton();

            let gtk_item = item.make_gtk_menu_item(app, *menu_id, menu_bar.menu(), host)?;

            match op {
                AddOp::Append => menu_bar.menu().append_item(&gtk_item),
                AddOp::Insert(position) => menu_bar.menu().insert_item(position as i32, &gtk_item),
            }

            if let Some(instance_id) = internal_id(&gtk_item) {
                item.add_custom_widget_to_host(*menu_id, instance_id);
            }
        }

        Ok(())
    }

    pub fn add_existing_item_to_instance(
        &mut self,
        item: &dyn IsMenuItem,
        id: GtkId,
    ) -> crate::Result<()> {
        return_if_item_not_supported!(item);

        if let Some(menu_bar) = self.instances.get(&id) {
            let host = menu_bar.custom_widget_host();
            let app = menu_bar.applicaiton();

            let gtk_item = item.make_gtk_menu_item(app, id, menu_bar.menu(), host)?;
            menu_bar.menu().append_item(&gtk_item);

            if let Some(instance_id) = internal_id(&gtk_item) {
                item.add_custom_widget_to_host(id, instance_id);
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn IsMenuItem) -> crate::Result<()> {
        let child = item.child();
        let positions = self
            .children
            .iter()
            .enumerate()
            .filter_map(|(index, current)| Rc::ptr_eq(current, &child).then_some(index))
            .collect::<Vec<_>>();

        if positions.is_empty() {
            return Err(crate::Error::NotAChildOfThisMenu);
        }

        for position in positions.into_iter().rev() {
            self.remove_at(position);
        }

        Ok(())
    }

    pub fn remove_at(&mut self, position: usize) -> Option<MenuItemKind> {
        if position >= self.children.len() {
            return None;
        }

        let child = self.children.remove(position);
        let item = child.borrow().kind(child.clone());

        for menu_id in self.instances.keys().copied().collect::<Vec<_>>() {
            child
                .borrow_mut()
                .remove_instance_for_parent_at_position(menu_id, position);
        }

        Some(item)
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
        let id = window.as_ptr() as GtkId;

        let Some(app) = window.application() else {
            return Err(crate::Error::GtkWindowWithoutApplication);
        };

        if self.instances.contains_key(&id) {
            return Err(crate::Error::AlreadyInitialized);
        }

        let menu_bar = GtkMenuBar::new(app.clone());
        attach_menubar_to_window(window, container, menu_bar.menu_bar())?;
        self.instances.insert(id, menu_bar);

        let action_group = action_group_from_app(&app);
        window.insert_action_group(DEFAULT_ACTION_GROUP, Some(&action_group));

        for item in self.items() {
            self.add_existing_item_to_instance(item.as_ref(), id)?;
        }

        let menu_bar = self.instances[&id].menu_bar();
        menu_bar.set_visible(true);

        Ok(())
    }

    pub fn remove_for_gtk_window<W>(&mut self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
        W: gtk4::prelude::IsA<gtk4::Widget>,
    {
        let id = window.as_ptr() as GtkId;

        let Some(menu_bar) = self.instances.remove(&id) else {
            return Err(crate::Error::NotInitialized);
        };

        for child in &self.children {
            child.borrow_mut().remove_instances_for_parent(id);
        }

        menu_bar.menu_bar().unparent();

        window.insert_action_group(DEFAULT_ACTION_GROUP, None::<&gio::SimpleActionGroup>);

        Ok(())
    }

    pub fn hide_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        let id = window.as_ptr() as GtkId;
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
        let id = window.as_ptr() as GtkId;
        let Some(menu_bar) = self.instances.get(&id) else {
            return Err(crate::Error::NotInitialized);
        };
        menu_bar.menu_bar().set_visible(true);
        Ok(())
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn is_visible_on_gtk_window<W>(&self, window: &W) -> bool
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        let id = window.as_ptr() as GtkId;
        self.instances
            .get(&id)
            .map(|m| m.menu_bar().is_visible())
            .unwrap_or(false)
    }

    pub fn gtk_menubar_for_gtk_window<W>(&self, window: &W) -> Option<gtk4::PopoverMenuBar>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        let id = window.as_ptr() as GtkId;
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

            let menu = GtkMenuBar::new_context(app);

            let widget = menu.context_menu();
            widget.insert_action_group(DEFAULT_ACTION_GROUP, Some(&action_group));

            self.instances.insert(self.ctx_menu_id, menu);

            for item in self.items() {
                let _ = self.add_existing_item_to_instance(item.as_ref(), self.ctx_menu_id);
            }
        }

        let (x, y) = match position {
            Some(p) => p.to_logical::<i32>(window.scale_factor() as _).into(),
            None => get_cursor_pos(window),
        };

        // SAFETY: it is guaranteed to exist due to the check above
        let menu = self.instances.get(&self.ctx_menu_id).unwrap();
        let context_menu = menu.context_menu();

        context_menu.unparent();
        context_menu.set_parent(window);

        context_menu.popup();
        context_menu.set_pointing_to(Some(&Rectangle::new(x, y, 0, 0)));

        true
    }
}

#[derive(Clone)]
struct GtkCustomWidget {
    widget: gtk4::Widget,
    host: gtk4::Widget,
}

#[derive(Clone)]
enum GtkMenuChild {
    Item {
        id: GtkId,
        parent_menu: gio::Menu,
        custom_widget: Option<GtkCustomWidget>,
        app: gtk4::Application,
    },
    Submenu {
        id: GtkId,
        parent_menu: gio::Menu,
        menu: gio::Menu,
        custom_widget_host: gtk4::Widget,
        app: gtk4::Application,
    },
    ContextMenu {
        id: GtkId,
        widget: gtk4::PopoverMenu,
        menu: gio::Menu,
        app: gtk4::Application,
    },
}

impl GtkMenuChild {
    fn id(&self) -> GtkId {
        match self {
            GtkMenuChild::Item { id, .. } => *id,
            GtkMenuChild::Submenu { id, .. } => *id,
            GtkMenuChild::ContextMenu { id, .. } => *id,
        }
    }

    fn application(&self) -> &gtk4::Application {
        match self {
            GtkMenuChild::Submenu { app, .. } => app,
            GtkMenuChild::ContextMenu { app, .. } => app,
            GtkMenuChild::Item { app, .. } => app,
        }
    }

    fn menu(&self) -> &gio::Menu {
        match self {
            GtkMenuChild::Submenu { menu, .. } => menu,
            GtkMenuChild::ContextMenu { menu, .. } => menu,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn custom_widget_host(&self) -> &gtk4::Widget {
        match self {
            GtkMenuChild::Submenu {
                custom_widget_host, ..
            } => custom_widget_host,
            GtkMenuChild::ContextMenu { widget, .. } => widget.upcast_ref(),
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn context_menu(&self) -> &gtk4::PopoverMenu {
        match self {
            GtkMenuChild::ContextMenu { widget, .. } => widget,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn parent_menu(&self) -> &gio::Menu {
        match self {
            GtkMenuChild::Item { parent_menu, .. } => parent_menu,
            GtkMenuChild::Submenu { parent_menu, .. } => parent_menu,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn replace_parent_row(&self, item: &gio::MenuItem) {
        let parent_menu = self.parent_menu();
        if let Some(index) = find_row_index(parent_menu, self.id()) {
            parent_menu.remove(index);
            parent_menu.insert_item(index, item);
        }
    }

    fn remove_custom_widget(&self) {
        if let GtkMenuChild::Item {
            custom_widget: Some(custom_widget),
            ..
        } = self
        {
            let GtkCustomWidget { widget, host } = custom_widget;

            if let Some(menu_bar) = host.downcast_ref::<gtk4::PopoverMenuBar>() {
                let _ = menu_bar.remove_child(widget);
            } else if let Some(menu) = host.downcast_ref::<gtk4::PopoverMenu>() {
                let _ = menu.remove_child(widget);
            }
        }
    }
}

fn gtk_action_item(
    text: &str,
    detailed_action: &str,
    icon: Option<&Icon>,
    id: GtkId,
) -> gio::MenuItem {
    let item = gio::MenuItem::new(Some(&to_gtk_mnemonic(text)), Some(detailed_action));
    if let Some(icon) = icon {
        item.set_icon(icon.inner.bytes_icon());
    }
    item.set_attribute_value(INTERNAL_ID_ATTRIBUTE, Some(&(id as u64).to_variant()));
    item
}

fn gtk_submenu_item(
    text: &str,
    detailed_action: &str,
    menu: &gio::Menu,
    id: GtkId,
) -> gio::MenuItem {
    let item = gio::MenuItem::new_submenu(Some(&to_gtk_mnemonic(text)), menu);
    item.set_detailed_action(detailed_action);
    item.set_attribute_value(INTERNAL_ID_ATTRIBUTE, Some(&(id as u64).to_variant()));
    item
}

fn internal_id_at(menu: &gio::Menu, index: i32) -> Option<GtkId> {
    menu.item_attribute_value(index, INTERNAL_ID_ATTRIBUTE, None)
        .and_then(|value| value.get::<u64>())
        .map(|id| id as GtkId)
}

fn internal_id(item: &gio::MenuItem) -> Option<GtkId> {
    item.attribute_value(INTERNAL_ID_ATTRIBUTE, None)
        .and_then(|value| value.get::<u64>())
        .map(|id| id as GtkId)
}

fn find_row_index(menu: &gio::Menu, id: GtkId) -> Option<i32> {
    (0..menu.n_items()).find(|index| internal_id_at(menu, *index) == Some(id))
}

fn attach_menubar_to_window<W, C>(
    window: &W,
    container: Option<&C>,
    menu_bar: &gtk4::PopoverMenuBar,
) -> crate::Result<()>
where
    W: gtk4::prelude::IsA<gtk4::Window>,
    C: gtk4::prelude::IsA<gtk4::Widget>,
{
    if let Some(container) = container {
        if let Some(gtk_box) = container.dynamic_cast_ref::<gtk4::Box>() {
            gtk_box.prepend(menu_bar);
        } else if let Some(gtk_fixed) = container.dynamic_cast_ref::<gtk4::Fixed>() {
            gtk_fixed.put(menu_bar, 0., 0.);
        } else if let Some(gtk_stack) = container.dynamic_cast_ref::<gtk4::Stack>() {
            gtk_stack.add_child(menu_bar);
        } else {
            return Err(crate::Error::UnsupportedGtkContainer);
        }
    } else {
        window.set_child(Some(menu_bar));
    }

    Ok(())
}

pub struct MenuChild {
    id: MenuId,
    action_name: String,
    text: String,
    enabled: bool,
    key_accelerator: Option<KeyAccelerator>,

    checked: bool,

    icon: Option<Icon>,

    type_: MenuItemType,
    predefined_item_type: Option<PredefinedMenuItemType>,

    instances: HashMap<GtkId, Vec<GtkMenuChild>>,
    ctx_menu_id: GtkId,
    children: Vec<Rc<RefCell<MenuChild>>>,

    action: Option<gio::SimpleAction>,
}

impl MenuChild {
    pub fn new_submenu(text: &str, enabled: bool, id: Option<MenuId>) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            action_name: format!("item-{}", COUNTER.next()),
            text: text.to_string(),
            enabled,
            checked: false,
            icon: None,
            key_accelerator: None,
            type_: MenuItemType::Submenu,
            predefined_item_type: None,
            ctx_menu_id: COUNTER.next() as GtkId,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn create_gtk_item_for_submenu(
        &mut self,
        app: &gtk4::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
        custom_widget_host: &gtk4::Widget,
    ) -> crate::Result<gio::MenuItem> {
        let menu = gio::Menu::new();
        let detailed_action = self.detailed_action();
        let id = COUNTER.next() as GtkId;
        let item = gtk_submenu_item(&self.text, &detailed_action, &menu, id);

        if self.action.is_none() {
            let action_group = action_group_from_app(&app);

            let action = gio::SimpleAction::new(&self.action_name, None);
            action.connect_activate(|_, _| ());
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        let child = GtkMenuChild::Submenu {
            parent_menu: parent_menu.clone(),
            menu,
            custom_widget_host: custom_widget_host.clone(),
            id,
            app: app.clone(),
        };

        self.instances.entry(menu_id).or_default().push(child);

        for item in self.items() {
            self.add_existing_item_to_instance(item.as_ref(), id)?;
        }

        Ok(item)
    }

    pub fn add_menu_item(&mut self, item: &dyn IsMenuItem, op: AddOp) -> crate::Result<()> {
        match op {
            AddOp::Append => self.children.push(item.child()),
            AddOp::Insert(i) => self.children.insert(i, item.child()),
        }

        return_if_item_not_supported!(item);

        for menus in self.instances.values() {
            for gtk_child in menus {
                let app = gtk_child.application();
                let menu_id = gtk_child.id();
                let parent_menu = gtk_child.menu();
                let host = gtk_child.custom_widget_host();

                let gtk_item = item.make_gtk_menu_item(app, menu_id, parent_menu, &host)?;

                match op {
                    AddOp::Append => gtk_child.menu().append_item(&gtk_item),
                    AddOp::Insert(position) => {
                        gtk_child.menu().insert_item(position as i32, &gtk_item)
                    }
                }

                if let Some(instance_id) = internal_id(&gtk_item) {
                    item.add_custom_widget_to_host(menu_id, instance_id);
                }
            }
        }

        Ok(())
    }

    pub fn add_existing_item_to_instance(
        &self,
        item: &dyn IsMenuItem,
        id: GtkId,
    ) -> crate::Result<()> {
        return_if_item_not_supported!(item);

        for menus in self.instances.values() {
            for gtk_child in menus.iter().filter(|m| m.id() == id) {
                let app = gtk_child.application();
                let menu_id = gtk_child.id();
                let parent_menu = gtk_child.menu();
                let host = gtk_child.custom_widget_host();

                let gtk_item = item.make_gtk_menu_item(app, menu_id, parent_menu, &host)?;
                gtk_child.menu().append_item(&gtk_item);

                if let Some(instance_id) = internal_id(&gtk_item) {
                    item.add_custom_widget_to_host(menu_id, instance_id);
                }
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn IsMenuItem) -> crate::Result<()> {
        let child = item.child();
        let positions = self
            .children
            .iter()
            .enumerate()
            .filter_map(|(index, current)| Rc::ptr_eq(current, &child).then_some(index))
            .collect::<Vec<_>>();

        if positions.is_empty() {
            return Err(crate::Error::NotAChildOfThisMenu);
        }

        for position in positions.into_iter().rev() {
            self.remove_at(position);
        }

        Ok(())
    }

    pub fn remove_at(&mut self, position: usize) -> Option<MenuItemKind> {
        if position >= self.children.len() {
            return None;
        }

        let child = self.children.remove(position);
        let item = child.borrow().kind(child.clone());

        for parent_id in self
            .instances
            .values()
            .flat_map(|menus| menus.iter().map(GtkMenuChild::id))
            .collect::<Vec<_>>()
        {
            child
                .borrow_mut()
                .remove_instance_for_parent_at_position(parent_id, position);
        }

        Some(item)
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
            widget.insert_action_group(DEFAULT_ACTION_GROUP, Some(&action_group));

            let menu = GtkMenuChild::ContextMenu {
                id: self.ctx_menu_id,
                widget,
                menu,
                app,
            };

            self.instances.insert(self.ctx_menu_id, vec![menu]);

            for item in self.items() {
                let _ = self.add_existing_item_to_instance(item.as_ref(), self.ctx_menu_id);
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

        context_menu.unparent();
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
            action_name: format!("item-{}", COUNTER.next()),
            text: text.to_string(),
            enabled,
            key_accelerator,
            icon: None,
            checked: false,
            type_: MenuItemType::MenuItem,
            predefined_item_type: None,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn create_gtk_item_for_menu_item(
        &mut self,
        app: &gtk4::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
    ) -> crate::Result<gio::MenuItem> {
        let detailed_action = self.detailed_action();
        let id = COUNTER.next() as GtkId;
        let item = gtk_action_item(&self.text, &detailed_action, None, id);

        if let Some(accelerator) = &self.key_accelerator {
            app.set_accels_for_action(&detailed_action, &[&accelerator.to_gtk()]);
        }

        if self.action.is_none() {
            let action_group = action_group_from_app(&app);

            let action = gio::SimpleAction::new(&self.action_name, None);
            let id = self.id.clone();
            action.connect_activate(move |_, _| MenuEvent::send(MenuEvent { id: id.clone() }));
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        let child = GtkMenuChild::Item {
            id,
            parent_menu: parent_menu.clone(),
            custom_widget: None,
            app: app.clone(),
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(item)
    }

    pub fn id(&self) -> &MenuId {
        &self.id
    }

    fn detailed_action(&self) -> String {
        format!("{DEFAULT_ACTION_GROUP}.{}", self.action_name)
    }

    pub fn item_type(&self) -> &MenuItemType {
        &self.type_
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.replace_gtk_items();
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

    fn replace_gtk_items(&self) {
        for instance in self.instances.values().flatten() {
            if let Some(item) = self.gtk_item_for_instance(instance) {
                instance.replace_parent_row(&item);
            }
        }
    }

    fn gtk_item_for_instance(&self, instance: &GtkMenuChild) -> Option<gio::MenuItem> {
        let detailed_action = self.detailed_action();

        match instance {
            GtkMenuChild::Item {
                id,
                custom_widget: None,
                ..
            } => {
                let icon = self.icon.as_ref();
                Some(gtk_action_item(&self.text, &detailed_action, icon, *id))
            }
            GtkMenuChild::Submenu { id, menu, .. } => {
                Some(gtk_submenu_item(&self.text, &detailed_action, menu, *id))
            }
            _ => None,
        }
    }

    // A logical menu item is alive while at least one GTK menu row still
    // represents it. Context popovers are containers, not item rows.
    fn is_alive(&self) -> bool {
        self.instances
            .values()
            .flatten()
            .any(|child| !matches!(child, GtkMenuChild::ContextMenu { .. }))
    }

    fn cleanup_unused_action(&mut self, app: &gtk4::Application) {
        if self.is_alive() {
            return;
        }

        app.set_accels_for_action(&self.detailed_action(), &[]);

        let Some(action) = self.action.take() else {
            return;
        };

        if self.type_ == MenuItemType::Check {
            // Any check activation other than set_checked() updates the GTK
            // action state, so self.checked may be stale, and
            // we need to synchronize it from the GTK state so later
            // calls to is_checked() or readdition of the item to a menu
            // will have the correct checked state.
            if let Some(checked) = action.state().and_then(|state| state.get()) {
                self.checked = checked;
            }
        }

        let action_group = action_group_from_app(app);
        action_group.remove_action(&self.action_name);
    }

    fn remove_instance_for_parent_at_position(&mut self, parent_id: GtkId, position: usize) {
        let Some(instances) = self.instances.get_mut(&parent_id) else {
            return;
        };
        let Some(parent_menu) = instances.first().map(GtkMenuChild::parent_menu).cloned() else {
            return;
        };

        // Remove the visible row at this parent position, capturing its
        // internal id first because the GMenuModel row disappears after remove.
        let Some(id) = internal_id_at(&parent_menu, position as i32) else {
            return;
        };
        parent_menu.remove(position as i32);

        // Drop the tracked GTK occurrence that belonged to the removed row.
        let Some(instance_index) = instances.iter().position(|instance| instance.id() == id) else {
            return;
        };
        let instance = instances.remove(instance_index);
        if instances.is_empty() {
            self.instances.remove(&parent_id);
        }

        instance.remove_custom_widget();

        if let GtkMenuChild::Submenu { id, .. } = instance {
            for child in &mut self.children {
                child.borrow_mut().remove_instances_for_parent(id);
            }
        }

        let app = instance.application();
        self.cleanup_unused_action(&app);
    }

    fn remove_instances_for_parent(&mut self, parent_id: GtkId) {
        let Some(instances) = self.instances.remove(&parent_id) else {
            return;
        };

        let app = instances
            .first()
            .map(|instance| instance.application().clone());

        for instance in instances {
            let parent_menu = instance.parent_menu();
            if let Some(index) = find_row_index(&parent_menu, instance.id()) {
                parent_menu.remove(index);
            }

            instance.remove_custom_widget();

            if let GtkMenuChild::Submenu { id, .. } = instance {
                for child in &mut self.children {
                    child.borrow_mut().remove_instances_for_parent(id);
                }
            }
        }

        if let Some(app) = app {
            self.cleanup_unused_action(&app);
        }
    }
}

impl MenuChild {
    pub fn new_predefined(item_type: PredefinedMenuItemType, text: Option<String>) -> Self {
        let key_accelerator = item_type.accelerator().map(Into::into);

        Self {
            id: MenuId(COUNTER.next().to_string()),
            action_name: format!("item-{}", COUNTER.next()),
            text: text.unwrap_or_else(|| item_type.text().to_string()),
            enabled: true,
            key_accelerator,
            icon: None,
            checked: false,
            type_: MenuItemType::Predefined,
            predefined_item_type: Some(item_type),
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn create_gtk_item_for_predefined_menu_item(
        &mut self,
        app: &gtk4::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
        custom_widget_host: &gtk4::Widget,
    ) -> crate::Result<gio::MenuItem> {
        let predefined_item_type = self.predefined_item_type.as_ref().unwrap().clone();

        // Separator is a special case, that requires custom widget
        if matches!(predefined_item_type, PredefinedMenuItemType::Separator) {
            return self.create_gtk_item_for_separator(
                app,
                menu_id,
                parent_menu,
                custom_widget_host,
            );
        }

        let detailed_action = self.detailed_action();
        let id = COUNTER.next() as GtkId;
        let item = gtk_action_item(&self.text, &detailed_action, None, id);

        if let Some(accelerator) = &self.key_accelerator {
            app.set_accels_for_action(&detailed_action, &[&accelerator.to_gtk()]);
        }

        if self.action.is_none() {
            let action_group = action_group_from_app(&app);

            let action = gio::SimpleAction::new(&self.action_name, None);
            let app = app.clone();
            action.connect_activate(move |_, _| {
                activate_predefined_action(&app, &predefined_item_type)
            });
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        let child = GtkMenuChild::Item {
            id,
            parent_menu: parent_menu.clone(),
            custom_widget: None,
            app: app.clone(),
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(item)
    }

    fn create_gtk_item_for_separator(
        &mut self,
        app: &gtk4::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
        custom_widget_host: &gtk4::Widget,
    ) -> crate::Result<gio::MenuItem> {
        let id = COUNTER.next() as GtkId;

        let item = gio::MenuItem::new(None, None);
        // We need to set "custom" attribute to mark it as a custom widget,
        // so later .add_child() will associate the widget with the menu item.
        item.set_attribute_value("custom", Some(&id.to_string().to_variant()));
        item.set_attribute_value(INTERNAL_ID_ATTRIBUTE, Some(&(id as u64).to_variant()));

        // GTK can only attach a custom child after the menu model contains
        // the row with the matching "custom" attribute.
        let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal).upcast();

        let child = GtkMenuChild::Item {
            id,
            parent_menu: parent_menu.clone(),
            custom_widget: Some(GtkCustomWidget {
                widget: separator,
                host: custom_widget_host.clone(),
            }),
            app: app.clone(),
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(item)
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
            action_name: format!("item-{}", COUNTER.next()),
            text: text.to_string(),
            enabled,
            key_accelerator,
            icon: None,
            checked,
            type_: MenuItemType::Check,
            predefined_item_type: None,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn create_gtk_item_for_check_menu_item(
        &mut self,
        app: &gtk4::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
    ) -> crate::Result<gio::MenuItem> {
        let detailed_action = self.detailed_action();
        let id = COUNTER.next() as GtkId;
        let item = gtk_action_item(&self.text, &detailed_action, None, id);

        if let Some(accelerator) = &self.key_accelerator {
            app.set_accels_for_action(&detailed_action, &[&accelerator.to_gtk()]);
        }

        if self.action.is_none() {
            let action_group = action_group_from_app(&app);

            let state = &self.checked.to_variant();
            let action = gio::SimpleAction::new_stateful(&self.action_name, None, state);
            let id = self.id.clone();
            // Dispatch from activation, not state notification, so set_checked()
            // can synchronize GTK state without emitting a user menu event.
            action.connect_activate(move |action, _| {
                let checked = action
                    .state()
                    .and_then(|state| state.get())
                    .unwrap_or(false);
                action.set_state(&(!checked).to_variant());
                MenuEvent::send(MenuEvent { id: id.clone() });
            });
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        let child = GtkMenuChild::Item {
            id,
            parent_menu: parent_menu.clone(),
            custom_widget: None,
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
            action_name: format!("item-{}", COUNTER.next()),
            text: text.to_string(),
            enabled,
            key_accelerator,
            icon,
            checked: false,
            type_: MenuItemType::Icon,
            predefined_item_type: None,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    pub fn new_native_icon(
        text: &str,
        enabled: bool,
        _icon: Option<NativeIcon>,
        key_accelerator: Option<KeyAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            action_name: format!("item-{}", COUNTER.next()),
            text: text.to_string(),
            enabled,
            key_accelerator,
            icon: None,
            checked: false,
            type_: MenuItemType::Icon,
            predefined_item_type: None,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn create_gtk_item_for_icon_menu_item(
        &mut self,
        app: &gtk4::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
    ) -> crate::Result<gio::MenuItem> {
        let detailed_action = self.detailed_action();
        let id = COUNTER.next() as GtkId;
        let item = gtk_action_item(&self.text, &detailed_action, self.icon.as_ref(), id);

        if let Some(accelerator) = &self.key_accelerator {
            app.set_accels_for_action(&detailed_action, &[&accelerator.to_gtk()]);
        }

        if self.action.is_none() {
            let action_group = action_group_from_app(&app);

            let action = gio::SimpleAction::new(&self.action_name, None);
            let id = self.id.clone();
            action.connect_activate(move |_, _| MenuEvent::send(MenuEvent { id: id.clone() }));
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        let child = GtkMenuChild::Item {
            id,
            parent_menu: parent_menu.clone(),
            custom_widget: None,
            app: app.clone(),
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(item)
    }

    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon = icon;
        self.replace_gtk_items();
    }
}

impl dyn IsMenuItem + '_ {
    fn make_gtk_menu_item(
        &self,
        app: &gtk4::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
        custom_widget_host: &gtk4::Widget,
    ) -> crate::Result<gio::MenuItem> {
        let kind = self.kind();
        let mut child = kind.child_mut();
        match child.item_type() {
            MenuItemType::Submenu => {
                child.create_gtk_item_for_submenu(app, menu_id, parent_menu, custom_widget_host)
            }
            MenuItemType::MenuItem => {
                child.create_gtk_item_for_menu_item(app, menu_id, parent_menu)
            }
            MenuItemType::Check => {
                child.create_gtk_item_for_check_menu_item(app, menu_id, parent_menu)
            }
            MenuItemType::Icon => {
                child.create_gtk_item_for_icon_menu_item(app, menu_id, parent_menu)
            }
            MenuItemType::Predefined => child.create_gtk_item_for_predefined_menu_item(
                app,
                menu_id,
                parent_menu,
                custom_widget_host,
            ),
        }
    }

    // GTK can only attach a custom child after the menu model contains
    // the row with the matching "custom" attribute.
    fn add_custom_widget_to_host(&self, menu_id: GtkId, instance_id: GtkId) {
        let kind = self.kind();
        let child = kind.child();
        let instances = child.instances.get(&menu_id);
        let Some(instance) = instances.and_then(|is| is.iter().find(|i| i.id() == instance_id))
        else {
            return;
        };

        if let GtkMenuChild::Item {
            custom_widget: Some(custom_widget),
            ..
        } = instance
        {
            let custom_id = instance_id.to_string();
            add_custom_child(&custom_widget.host, &custom_widget.widget, &custom_id);
        }

        if let GtkMenuChild::Submenu { id, menu, .. } = instance {
            let id = *id;
            let menu = menu.clone();
            let items = child
                .items()
                .into_iter()
                .filter(|item| is_item_supported!(item.as_ref()))
                .collect::<Vec<_>>();

            // Release the borrow before recursing into submenu children.
            drop(child);

            for (index, item) in items.iter().enumerate() {
                let Some(instance_id) = internal_id_at(&menu, index as i32) else {
                    continue;
                };

                item.as_ref().add_custom_widget_to_host(id, instance_id);
            }
        }
    }
}

fn activate_predefined_action(
    app: &gtk4::Application,
    predefined_item_type: &PredefinedMenuItemType,
) {
    let Some(window) = app.active_window() else {
        return;
    };

    match predefined_item_type {
        PredefinedMenuItemType::Minimize => window.minimize(),
        PredefinedMenuItemType::Maximize => window.maximize(),
        PredefinedMenuItemType::Fullscreen => {
            if window.is_fullscreen() {
                window.unfullscreen();
            } else {
                window.fullscreen();
            }
        }
        PredefinedMenuItemType::Hide => window.set_visible(false),
        PredefinedMenuItemType::CloseWindow => window.close(),
        PredefinedMenuItemType::Quit => {
            for window in app.windows() {
                window.close();
            }
            app.quit();
        }
        PredefinedMenuItemType::About(metadata) => {
            show_about_dialog(app, &window, metadata.as_ref());
        }
        _ => {}
    }
}

fn show_about_dialog(
    app: &gtk4::Application,
    window: &gtk4::Window,
    metadata: Option<&AboutMetadata>,
) {
    let title = metadata
        .and_then(|m| m.name.as_deref())
        .unwrap_or_default()
        .to_string();
    let title = format!("About {}", title);

    let mut builder = gtk4::AboutDialog::builder()
        .application(app)
        .modal(true)
        .transient_for(window)
        .title(&title);

    if let Some(metadata) = metadata {
        if let Some(name) = &metadata.name {
            builder = builder.program_name(name);
        }
        if let Some(version) = &metadata.full_version() {
            builder = builder.version(version);
        }
        if let Some(authors) = &metadata.authors {
            let authors = authors.iter().map(String::as_str).collect::<Vec<_>>();
            builder = builder.authors(authors);
        }
        if let Some(comments) = &metadata.comments {
            builder = builder.comments(comments);
        }
        if let Some(copyright) = &metadata.copyright {
            builder = builder.copyright(copyright);
        }
        if let Some(license) = &metadata.license {
            builder = builder.license(license).wrap_license(true);
        }
        if let Some(website) = &metadata.website {
            builder = builder.website(website);
        }
        if let Some(website_label) = &metadata.website_label {
            builder = builder.website_label(website_label);
        }
        if let Some(icon) = &metadata.icon {
            if let Ok(texture) = gtk4::gdk::Texture::from_bytes(&icon.inner.bytes_icon().bytes()) {
                builder = builder.logo(&texture);
            }
        }
    }

    let dialog = builder.build();

    if let Some(titlebar) = dialog
        .titlebar()
        .and_then(|titlebar| titlebar.downcast::<gtk4::HeaderBar>().ok())
    {
        let title_label = gtk4::Label::new(Some(&title));
        titlebar.set_title_widget(Some(&title_label));
    }

    dialog.present();
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

fn add_custom_child(host: &gtk4::Widget, child: &impl IsA<gtk4::Widget>, id: &str) {
    if let Some(menu_bar) = host.downcast_ref::<gtk4::PopoverMenuBar>() {
        let _ = menu_bar.add_child(child, id);
    } else if let Some(menu) = host.downcast_ref::<gtk4::PopoverMenu>() {
        let _ = menu.add_child(child, id);
    }
}
