// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod icon;
mod icon_menu_item;
mod mnemonic;

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use dpi::Position;
use gtk::{gdk::Rectangle, gio, glib, prelude::*};
pub(crate) use icon::PlatformIcon;
use icon_menu_item::IconMenuItem;
use mnemonic::to_gtk_mnemonic;

use crate::{
    accelerator::MenuAccelerator,
    util::{AddOp, Counter},
    AboutMetadata, Icon, IsMenuItem, MenuEvent, MenuId, MenuItemKind, MenuItemType, NativeIcon,
    PredefinedMenuItemType, TextStyle,
};

static COUNTER: Counter = Counter::new();

const DEFAULT_ACTION_GROUP: &str = "muda";
const ACTION_GROUP_DATA_KEY: &str = "mudaActionGroup";
const INTERNAL_ID_ATTRIBUTE: &str = "muda-internal-id";

type GtkId = usize;

enum GtkMenuBar {
    MenuBar {
        widget: gtk::PopoverMenuBar,
        menu: gio::Menu,
        app: gtk::Application,
    },
    ContextMenu {
        widget: gtk::PopoverMenu,
        menu: gio::Menu,
        app: gtk::Application,
    },
}

impl GtkMenuBar {
    fn new(app: gtk::Application) -> Self {
        let menu = gio::Menu::new();
        let widget = gtk::PopoverMenuBar::from_model(Some(&menu));
        Self::MenuBar { widget, menu, app }
    }

    fn new_context(app: gtk::Application) -> Self {
        let menu = gio::Menu::new();
        let widget = gtk::PopoverMenu::from_model_full(&menu, gtk::PopoverMenuFlags::NESTED);
        Self::ContextMenu { widget, menu, app }
    }

    fn applicaiton(&self) -> &gtk::Application {
        match self {
            GtkMenuBar::MenuBar { app, .. } => app,
            GtkMenuBar::ContextMenu { app, .. } => app,
        }
    }

    fn menu_bar(&self) -> &gtk::PopoverMenuBar {
        match self {
            GtkMenuBar::MenuBar { widget, .. } => widget,
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn context_menu(&self) -> &gtk::PopoverMenu {
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

    fn widget(&self) -> &gtk::Widget {
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

        for (menu_id, menu_bar) in &self.instances {
            let host = menu_bar.widget();
            let app = menu_bar.applicaiton();

            item.insert_gtk_menu_item(app, *menu_id, menu_bar.menu(), host, op)?;
        }

        Ok(())
    }

    pub fn add_existing_item_to_instance(
        &mut self,
        item: &dyn IsMenuItem,
        id: GtkId,
    ) -> crate::Result<()> {
        if let Some(menu_bar) = self.instances.get(&id) {
            let host = menu_bar.widget();
            let app = menu_bar.applicaiton();

            item.insert_gtk_menu_item(app, id, menu_bar.menu(), host, AddOp::Append)?;
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
        W: gtk::prelude::IsA<gtk::Window>,
        W: gtk::prelude::IsA<gtk::Widget>,
        C: gtk::prelude::IsA<gtk::Widget>,
    {
        let id = window.as_ptr() as GtkId;

        let Some(app) = window.application() else {
            return Err(crate::Error::GtkWindowWithoutApplication);
        };

        if self.instances.contains_key(&id) {
            return Err(crate::Error::AlreadyInitialized);
        }

        let menu_bar = GtkMenuBar::new(app.clone());
        Self::attach_menubar_to_window(window, container, menu_bar.menu_bar())?;
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

    fn attach_menubar_to_window<W, C>(
        window: &W,
        container: Option<&C>,
        menu_bar: &gtk::PopoverMenuBar,
    ) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window>,
        C: gtk::prelude::IsA<gtk::Widget>,
    {
        if let Some(container) = container {
            if let Some(gtk_box) = container.dynamic_cast_ref::<gtk::Box>() {
                gtk_box.prepend(menu_bar);
            } else if let Some(gtk_fixed) = container.dynamic_cast_ref::<gtk::Fixed>() {
                gtk_fixed.put(menu_bar, 0., 0.);
            } else if let Some(gtk_stack) = container.dynamic_cast_ref::<gtk::Stack>() {
                gtk_stack.add_child(menu_bar);
            } else {
                return Err(crate::Error::UnsupportedGtkContainer);
            }
        } else {
            window.set_child(Some(menu_bar));
        }

        Ok(())
    }

    pub fn remove_for_gtk_window<W>(&mut self, window: &W) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window>,
        W: gtk::prelude::IsA<gtk::Widget>,
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
        W: gtk::prelude::IsA<gtk::Window>,
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
        W: gtk::prelude::IsA<gtk::Window>,
    {
        let id = window.as_ptr() as GtkId;
        let Some(menu_bar) = self.instances.get(&id) else {
            return Err(crate::Error::NotInitialized);
        };
        menu_bar.menu_bar().set_visible(true);
        Ok(())
    }

    pub fn is_visible_on_gtk_window<W>(&self, window: &W) -> bool
    where
        W: gtk::prelude::IsA<gtk::Window>,
    {
        let id = window.as_ptr() as GtkId;
        self.instances
            .get(&id)
            .map(|m| m.menu_bar().is_visible())
            .unwrap_or(false)
    }

    pub fn gtk_menubar_for_gtk_window<W>(&self, window: &W) -> Option<gtk::PopoverMenuBar>
    where
        W: gtk::prelude::IsA<gtk::Window>,
    {
        let id = window.as_ptr() as GtkId;
        self.instances.get(&id).map(|m| m.menu_bar().clone())
    }

    pub fn show_context_menu_for_gtk_window(
        &mut self,
        window: &gtk::Window,
        position: Option<Position>,
    ) -> bool {
        let Some(app) = window.application() else {
            return false;
        };

        self.ensure_context_menu(app);

        let (x, y) = match position {
            Some(p) => p.to_logical::<i32>(scale_factor(window)).into(),
            None => get_cursor_pos(window),
        };

        // SAFETY: it is guaranteed to exist due to ensure_context_menu.
        let menu = self.instances.get(&self.ctx_menu_id).unwrap();
        let context_menu = menu.context_menu();

        context_menu.set_parent(window);

        let items = self.items();
        run_context_menu(context_menu, &items, x, y)
    }

    pub fn gtk_context_menu(&mut self) -> gtk::PopoverMenu {
        self.ensure_context_menu(self.context_menu_application());

        // SAFETY: it is guaranteed to exist due to ensure_context_menu.
        self.instances[&self.ctx_menu_id].context_menu().clone()
    }

    fn ensure_context_menu(&mut self, app: gtk::Application) {
        if self.instances.contains_key(&self.ctx_menu_id) {
            return;
        }

        let action_group = action_group_from_app(&app);

        let menu = GtkMenuBar::new_context(app);

        let widget = menu.context_menu();
        widget.insert_action_group(DEFAULT_ACTION_GROUP, Some(&action_group));

        self.instances.insert(self.ctx_menu_id, menu);

        for item in self.items() {
            let _ = self.add_existing_item_to_instance(item.as_ref(), self.ctx_menu_id);
        }
    }

    fn context_menu_application(&self) -> gtk::Application {
        self.instances
            .values()
            .next()
            .map(|menu| menu.applicaiton().clone())
            .unwrap_or_else(default_gtk_application)
    }
}

#[derive(Clone)]
struct GtkCustomWidget {
    widget: gtk::Widget,
    host: gtk::Widget,
}

impl GtkCustomWidget {
    fn new(widget: impl IsA<gtk::Widget>, host: &gtk::Widget) -> Self {
        Self {
            widget: widget.upcast(),
            host: host.clone(),
        }
    }
}

#[derive(Clone)]
enum GtkMenuChild {
    Item {
        id: GtkId,
        parent_menu: gio::Menu,
        widget: Option<GtkCustomWidget>,
        app: gtk::Application,
    },
    Submenu {
        id: GtkId,
        parent_menu: gio::Menu,
        menu: gio::Menu,
        widget: gtk::PopoverMenu,
        app: gtk::Application,
    },
    ContextMenu {
        id: GtkId,
        widget: gtk::PopoverMenu,
        menu: gio::Menu,
        app: gtk::Application,
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

    fn application(&self) -> &gtk::Application {
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

    fn widget(&self) -> &gtk::Widget {
        match self {
            GtkMenuChild::Submenu { widget, .. } => widget.upcast_ref(),
            GtkMenuChild::ContextMenu { widget, .. } => widget.upcast_ref(),
            _ => unreachable!("This is a bug report to https://github.com/tauri-apps/muda"),
        }
    }

    fn context_menu(&self) -> &gtk::PopoverMenu {
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
            widget: Some(widget),
            ..
        } = self
        {
            remove_custom_child(&widget.host, &widget.widget);
        }
    }
}

pub struct MenuChild {
    id: MenuId,
    action_name: String,
    text: String,
    enabled: bool,
    accelerator: Option<MenuAccelerator>,

    checked: bool,

    icon: Option<Icon>,
    native_icon: Option<NativeIcon>,

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
            native_icon: None,
            accelerator: None,
            type_: MenuItemType::Submenu,
            predefined_item_type: None,
            ctx_menu_id: COUNTER.next() as GtkId,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn insert_gtk_item_for_submenu(
        &mut self,
        app: &gtk::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
        parent_widget: &gtk::Widget,
        op: AddOp,
    ) -> crate::Result<()> {
        let menu = gio::Menu::new();
        let detailed_action = self.detailed_action();
        let id = COUNTER.next() as GtkId;
        let item = gio_submenu(&self.text, &detailed_action, &menu, id);

        if self.action.is_none() {
            let action_group = action_group_from_app(app);

            let action = gio::SimpleAction::new(&self.action_name, None);
            action.connect_activate(|_, _| ());
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        match op {
            AddOp::Append => parent_menu.append_item(&item),
            AddOp::Insert(position) => parent_menu.insert_item(position as i32, &item),
        }

        let widget = find_submenu_widget(parent_widget, &menu)
            .expect("GTK did not create a PopoverMenu for submenu");

        let child = GtkMenuChild::Submenu {
            parent_menu: parent_menu.clone(),
            menu,
            widget,
            id,
            app: app.clone(),
        };

        self.instances.entry(menu_id).or_default().push(child);

        for item in self.items() {
            self.add_existing_item_to_instance(item.as_ref(), id)?;
        }

        Ok(())
    }

    pub fn add_menu_item(&mut self, item: &dyn IsMenuItem, op: AddOp) -> crate::Result<()> {
        match op {
            AddOp::Append => self.children.push(item.child()),
            AddOp::Insert(i) => self.children.insert(i, item.child()),
        }

        for menus in self.instances.values() {
            for gtk_child in menus {
                let app = gtk_child.application();
                let menu_id = gtk_child.id();
                let parent_menu = gtk_child.menu();
                let host = gtk_child.widget();

                item.insert_gtk_menu_item(app, menu_id, parent_menu, host, op)?;
            }
        }

        Ok(())
    }

    pub fn add_existing_item_to_instance(
        &self,
        item: &dyn IsMenuItem,
        id: GtkId,
    ) -> crate::Result<()> {
        for menus in self.instances.values() {
            for gtk_child in menus.iter().filter(|m| m.id() == id) {
                let app = gtk_child.application();
                let menu_id = gtk_child.id();
                let parent_menu = gtk_child.menu();
                let host = gtk_child.widget();

                item.insert_gtk_menu_item(app, menu_id, parent_menu, host, AddOp::Append)?;
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
        window: &gtk::Window,
        position: Option<Position>,
    ) -> bool {
        let Some(app) = window.application() else {
            return false;
        };

        self.ensure_context_menu(app);

        // SAFETY: it is guaranteed to exist due to ensure_context_menu.
        let menus = self.instances.get(&self.ctx_menu_id).unwrap();
        let menu = menus.first().unwrap();

        let (x, y) = match position {
            Some(p) => p.to_logical::<i32>(scale_factor(window)).into(),
            None => get_cursor_pos(window),
        };

        let context_menu = menu.context_menu();

        context_menu.set_parent(window);

        let items = self.items();
        run_context_menu(context_menu, &items, x, y)
    }

    pub fn gtk_context_menu(&mut self) -> gtk::PopoverMenu {
        self.ensure_context_menu(self.context_menu_application());

        // SAFETY: it is guaranteed to exist due to ensure_context_menu.
        self.instances[&self.ctx_menu_id]
            .first()
            .unwrap()
            .context_menu()
            .clone()
    }

    fn ensure_context_menu(&mut self, app: gtk::Application) {
        if self.instances.contains_key(&self.ctx_menu_id) {
            return;
        }

        let menu = gio::Menu::new();
        let widget = gtk::PopoverMenu::from_model_full(&menu, gtk::PopoverMenuFlags::NESTED);

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

    fn context_menu_application(&self) -> gtk::Application {
        self.instances
            .values()
            .flatten()
            .next()
            .map(|menu| menu.application().clone())
            .unwrap_or_else(default_gtk_application)
    }
}

impl MenuChild {
    pub fn new(
        text: &str,
        enabled: bool,
        accelerator: Option<MenuAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            action_name: format!("item-{}", COUNTER.next()),
            text: text.to_string(),
            enabled,
            accelerator,
            icon: None,
            native_icon: None,
            checked: false,
            type_: MenuItemType::MenuItem,
            predefined_item_type: None,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn insert_gtk_item_for_menu_item(
        &mut self,
        app: &gtk::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
        op: AddOp,
    ) -> crate::Result<()> {
        let detailed_action = self.detailed_action();
        let id = COUNTER.next() as GtkId;
        let item = gio_item(&self.text, &detailed_action, id);

        if let Some(accelerator) = self.accelerator.as_ref().and_then(|a| a.to_gtk()) {
            app.set_accels_for_action(&detailed_action, &[&accelerator]);
        }

        if self.action.is_none() {
            let action_group = action_group_from_app(app);

            let action = gio::SimpleAction::new(&self.action_name, None);
            let id = self.id.clone();
            action.connect_activate(move |_, _| MenuEvent::send(MenuEvent { id: id.clone() }));
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        match op {
            AddOp::Append => parent_menu.append_item(&item),
            AddOp::Insert(position) => parent_menu.insert_item(position as i32, &item),
        }

        let child = GtkMenuChild::Item {
            id,
            parent_menu: parent_menu.clone(),
            widget: None,
            app: app.clone(),
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(())
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

        self.for_each_icon_item(|widget| widget.set_label(text));
    }

    pub fn set_styled_text<S: AsRef<str>>(
        &mut self,
        parts: impl IntoIterator<Item = (S, TextStyle)>,
    ) {
        let text = parts
            .into_iter()
            .map(|(s, _style)| s.as_ref().to_string())
            .collect::<String>();

        self.set_text(&text);
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

    pub fn set_accelerator(&mut self, accelerator: Option<MenuAccelerator>) -> crate::Result<()> {
        let detailed_action = self.detailed_action();
        let gtk_accelerator = accelerator.as_ref().and_then(|a| a.to_gtk());

        for item in self.instances.values().flat_map(|v| v.iter()) {
            if let Some(accelerator) = gtk_accelerator.as_ref() {
                let app = item.application();
                app.set_accels_for_action(&detailed_action, &[accelerator]);
            }
        }

        self.accelerator = accelerator;

        if self.type_ == MenuItemType::Icon {
            let accelerator = self.accelerator.as_ref();
            self.for_each_icon_item(|widget| widget.set_accelerator(accelerator));
        }

        Ok(())
    }

    fn replace_gtk_items(&self) {
        for instance in self.instances.values().flatten() {
            if let Some(item) = self.gtk_item_for_instance(instance, self.type_) {
                instance.replace_parent_row(&item);
            }
        }
    }

    fn gtk_item_for_instance(
        &self,
        instance: &GtkMenuChild,
        type_: MenuItemType,
    ) -> Option<gio::MenuItem> {
        let detailed_action = self.detailed_action();

        match instance {
            GtkMenuChild::Item {
                widget: Some(_), ..
            } if type_ == MenuItemType::Icon => None,
            GtkMenuChild::Item {
                id, widget: None, ..
            } => Some(gio_item(&self.text, &detailed_action, *id)),
            GtkMenuChild::Submenu { id, menu, .. } => {
                Some(gio_submenu(&self.text, &detailed_action, menu, *id))
            }
            _ => None,
        }
    }

    /// A logical menu item is alive while at least one GTK menu row still
    /// represents it. Context popovers are containers, not item rows.
    fn is_alive(&self) -> bool {
        self.instances
            .values()
            .flatten()
            .any(|child| !matches!(child, GtkMenuChild::ContextMenu { .. }))
    }

    fn cleanup_unused_action(&mut self, app: &gtk::Application) {
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

        // Drop the tracked GTK occurrence that belonged to the removed row.
        let Some(instance_index) = instances.iter().position(|instance| instance.id() == id) else {
            return;
        };
        let instance = instances.remove(instance_index);
        if instances.is_empty() {
            self.instances.remove(&parent_id);
        }

        instance.remove_custom_widget();
        parent_menu.remove(position as i32);

        if let GtkMenuChild::Submenu { id, .. } = instance {
            for child in &mut self.children {
                child.borrow_mut().remove_instances_for_parent(id);
            }
        }

        let app = instance.application();
        self.cleanup_unused_action(app);
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
            instance.remove_custom_widget();
            if let Some(index) = find_row_index(parent_menu, instance.id()) {
                parent_menu.remove(index);
            }

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
        let accelerator = item_type.accelerator();
        let enabled = item_type.is_supported_on_gtk4();

        Self {
            id: MenuId(COUNTER.next().to_string()),
            action_name: format!("item-{}", COUNTER.next()),
            text: text.unwrap_or_else(|| item_type.text().to_string()),
            enabled,
            accelerator,
            icon: None,
            native_icon: None,
            checked: false,
            type_: MenuItemType::Predefined,
            predefined_item_type: Some(item_type),
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn insert_gtk_item_for_predefined_menu_item(
        &mut self,
        app: &gtk::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
        parent_widget: &gtk::Widget,
        op: AddOp,
    ) -> crate::Result<()> {
        let predefined_item_type = self.predefined_item_type.as_ref().unwrap().clone();

        // Separator is a special case, that requires custom widget
        if matches!(predefined_item_type, PredefinedMenuItemType::Separator) {
            return self.insert_gtk_item_for_separator(
                app,
                menu_id,
                parent_menu,
                parent_widget,
                op,
            );
        }

        let detailed_action = self.detailed_action();
        let id = COUNTER.next() as GtkId;
        let item = gio_item(&self.text, &detailed_action, id);

        if let Some(accelerator) = self.accelerator.as_ref().and_then(|a| a.to_gtk()) {
            app.set_accels_for_action(&detailed_action, &[&accelerator]);
        }

        if self.action.is_none() {
            let action_group = action_group_from_app(app);

            let action = gio::SimpleAction::new(&self.action_name, None);
            if predefined_item_type.is_supported_on_gtk4() {
                let app = app.clone();
                action.connect_activate(move |_, _| {
                    activate_predefined_action(&app, &predefined_item_type)
                });
            }

            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        match op {
            AddOp::Append => parent_menu.append_item(&item),
            AddOp::Insert(position) => parent_menu.insert_item(position as i32, &item),
        }

        let child = GtkMenuChild::Item {
            id,
            parent_menu: parent_menu.clone(),
            widget: None,
            app: app.clone(),
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(())
    }

    fn insert_gtk_item_for_separator(
        &mut self,
        app: &gtk::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
        parent_widget: &gtk::Widget,
        op: AddOp,
    ) -> crate::Result<()> {
        let id = COUNTER.next() as GtkId;

        let item = gio_custom_item(None, None, id);
        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);

        let widget = GtkCustomWidget::new(separator, parent_widget);

        match op {
            AddOp::Append => parent_menu.append_item(&item),
            AddOp::Insert(position) => parent_menu.insert_item(position as i32, &item),
        }

        add_custom_child(&widget.host, &widget.widget, &id.to_string());

        let child = GtkMenuChild::Item {
            id,
            parent_menu: parent_menu.clone(),
            widget: Some(widget),
            app: app.clone(),
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(())
    }
}

impl MenuChild {
    pub fn new_check(
        text: &str,
        enabled: bool,
        checked: bool,
        accelerator: Option<MenuAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            action_name: format!("item-{}", COUNTER.next()),
            text: text.to_string(),
            enabled,
            accelerator,
            icon: None,
            native_icon: None,
            checked,
            type_: MenuItemType::Check,
            predefined_item_type: None,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn insert_gtk_item_for_check_menu_item(
        &mut self,
        app: &gtk::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
        op: AddOp,
    ) -> crate::Result<()> {
        let detailed_action = self.detailed_action();
        let id = COUNTER.next() as GtkId;
        let item = gio_item(&self.text, &detailed_action, id);

        if let Some(accelerator) = self.accelerator.as_ref().and_then(|a| a.to_gtk()) {
            app.set_accels_for_action(&detailed_action, &[&accelerator]);
        }

        if self.action.is_none() {
            let action_group = action_group_from_app(app);

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

        match op {
            AddOp::Append => parent_menu.append_item(&item),
            AddOp::Insert(position) => parent_menu.insert_item(position as i32, &item),
        }

        let child = GtkMenuChild::Item {
            id,
            parent_menu: parent_menu.clone(),
            widget: None,
            app: app.clone(),
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(())
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
        accelerator: Option<MenuAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            action_name: format!("item-{}", COUNTER.next()),
            text: text.to_string(),
            enabled,
            accelerator,
            icon,
            native_icon: None,
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
        native_icon: Option<NativeIcon>,
        accelerator: Option<MenuAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            action_name: format!("item-{}", COUNTER.next()),
            text: text.to_string(),
            enabled,
            accelerator,
            icon: None,
            native_icon,
            checked: false,
            type_: MenuItemType::Icon,
            predefined_item_type: None,
            ctx_menu_id: 0,
            instances: HashMap::new(),
            children: Vec::new(),
            action: None,
        }
    }

    fn insert_gtk_item_for_icon_menu_item(
        &mut self,
        app: &gtk::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
        parent_widget: &gtk::Widget,
        op: AddOp,
    ) -> crate::Result<()> {
        let detailed_action = self.detailed_action();
        let id = COUNTER.next() as GtkId;
        let item = gio_custom_item(Some(&self.text), Some(&detailed_action), id);

        if let Some(accelerator) = self.accelerator.as_ref().and_then(|a| a.to_gtk()) {
            app.set_accels_for_action(&detailed_action, &[&accelerator]);
        }

        if self.action.is_none() {
            let action_group = action_group_from_app(app);

            let action = gio::SimpleAction::new(&self.action_name, None);
            let id = self.id.clone();
            action.connect_activate(move |_, _| MenuEvent::send(MenuEvent { id: id.clone() }));
            action.set_enabled(self.enabled);
            action_group.add_action(&action);

            self.action = Some(action);
        }

        let widget = IconMenuItem::new(
            &self.text,
            &detailed_action,
            self.icon.as_ref(),
            self.native_icon.as_ref().map(NativeIcon::gtk_icon_name),
            self.accelerator.as_ref(),
        );

        let widget = GtkCustomWidget::new(widget.clone(), parent_widget);
        match op {
            AddOp::Append => parent_menu.append_item(&item),
            AddOp::Insert(position) => parent_menu.insert_item(position as i32, &item),
        }

        add_custom_child(&widget.host, &widget.widget, &id.to_string());

        let child = GtkMenuChild::Item {
            id,
            parent_menu: parent_menu.clone(),
            widget: Some(widget),
            app: app.clone(),
        };
        self.instances.entry(menu_id).or_default().push(child);

        Ok(())
    }

    fn for_each_icon_item(&self, mut f: impl FnMut(&IconMenuItem)) {
        for instance in self.instances.values().flatten() {
            if let GtkMenuChild::Item {
                widget: Some(widget),
                ..
            } = instance
            {
                if let Some(widget) = widget.widget.downcast_ref() {
                    f(widget);
                }
            }
        }
    }

    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon = icon;
        self.native_icon = None;
        self.for_each_icon_item(|widget| widget.set_icon(self.icon.as_ref()));
    }

    pub fn set_native_icon(&mut self, icon: Option<NativeIcon>) {
        self.native_icon = icon;
        self.icon = None;
        let icon = self.native_icon.as_ref().map(NativeIcon::gtk_icon_name);
        self.for_each_icon_item(|widget| widget.set_native_icon(icon));
    }
}

impl dyn IsMenuItem + '_ {
    fn insert_gtk_menu_item(
        &self,
        app: &gtk::Application,
        menu_id: GtkId,
        parent_menu: &gio::Menu,
        parent_widget: &gtk::Widget,
        op: AddOp,
    ) -> crate::Result<()> {
        let kind = self.kind();
        let mut child = kind.child_mut();
        match child.item_type() {
            MenuItemType::Submenu => {
                child.insert_gtk_item_for_submenu(app, menu_id, parent_menu, parent_widget, op)
            }
            MenuItemType::MenuItem => {
                child.insert_gtk_item_for_menu_item(app, menu_id, parent_menu, op)
            }
            MenuItemType::Check => {
                child.insert_gtk_item_for_check_menu_item(app, menu_id, parent_menu, op)
            }
            MenuItemType::Icon => child.insert_gtk_item_for_icon_menu_item(
                app,
                menu_id,
                parent_menu,
                parent_widget,
                op,
            ),
            MenuItemType::Predefined => child.insert_gtk_item_for_predefined_menu_item(
                app,
                menu_id,
                parent_menu,
                parent_widget,
                op,
            ),
        }
    }
}

fn gio_item(text: &str, detailed_action: &str, id: GtkId) -> gio::MenuItem {
    let item = gio::MenuItem::new(Some(&to_gtk_mnemonic(text)), Some(detailed_action));
    item.set_attribute_value(INTERNAL_ID_ATTRIBUTE, Some(&(id as u64).to_variant()));
    item
}

fn gio_custom_item(text: Option<&str>, detailed_action: Option<&str>, id: GtkId) -> gio::MenuItem {
    let label = text.map(to_gtk_mnemonic);
    let item = gio::MenuItem::new(label.as_deref(), detailed_action);
    item.set_attribute_value(INTERNAL_ID_ATTRIBUTE, Some(&(id as u64).to_variant()));
    // GTK matches add_child() widgets to menu rows through this "custom" attribute.
    item.set_attribute_value("custom", Some(&id.to_string().to_variant()));
    item
}

fn gio_submenu(text: &str, detailed_action: &str, menu: &gio::Menu, id: GtkId) -> gio::MenuItem {
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

fn find_row_index(menu: &gio::Menu, id: GtkId) -> Option<i32> {
    (0..menu.n_items()).find(|index| internal_id_at(menu, *index) == Some(id))
}

/// Gtk creates a new PopoverMenu for each submenu,
/// so we need to find the correct PopoverMenu that matches the given gio::Menu.
/// so we can add the custom item to the correct PopoverMenu.
fn find_submenu_widget(host: &gtk::Widget, menu: &gio::Menu) -> Option<gtk::PopoverMenu> {
    if let Some(popover_menu) = host.downcast_ref::<gtk::PopoverMenu>() {
        if popover_menu
            .menu_model()
            .as_ref()
            .is_some_and(|model| is_same_menu_model(model, menu))
        {
            return Some(popover_menu.clone());
        }
    }

    let mut child = host.first_child();
    while let Some(current) = child {
        if let Some(host) = find_submenu_widget(&current, menu) {
            return Some(host);
        }

        child = current.next_sibling();
    }

    None
}

fn is_same_menu_model(model: &gio::MenuModel, menu: &gio::Menu) -> bool {
    model.as_ptr().cast::<()>() == menu.as_ptr().cast::<()>()
}

fn add_custom_child(host: &gtk::Widget, child: &impl IsA<gtk::Widget>, id: &str) {
    if let Some(menu_bar) = host.downcast_ref::<gtk::PopoverMenuBar>() {
        let _ = menu_bar.add_child(child, id);
    } else if let Some(menu) = host.downcast_ref::<gtk::PopoverMenu>() {
        let _ = menu.add_child(child, id);
    }
}

fn remove_custom_child(host: &gtk::Widget, child: &impl IsA<gtk::Widget>) {
    if let Some(menu_bar) = host.downcast_ref::<gtk::PopoverMenuBar>() {
        menu_bar.remove_child(child);
    } else if let Some(menu) = host.downcast_ref::<gtk::PopoverMenu>() {
        menu.remove_child(child);
    }
}

impl PredefinedMenuItemType {
    fn is_supported_on_gtk4(&self) -> bool {
        matches!(
            self,
            PredefinedMenuItemType::Separator
                | PredefinedMenuItemType::Minimize
                | PredefinedMenuItemType::Maximize
                | PredefinedMenuItemType::Fullscreen
                | PredefinedMenuItemType::Hide
                | PredefinedMenuItemType::CloseWindow
                | PredefinedMenuItemType::Quit
                | PredefinedMenuItemType::About(_)
        )
    }
}

fn activate_predefined_action(
    app: &gtk::Application,
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
    app: &gtk::Application,
    window: &gtk::Window,
    metadata: Option<&AboutMetadata>,
) {
    let title = metadata
        .and_then(|m| m.name.as_deref())
        .unwrap_or_default()
        .to_string();
    let title = format!("About {}", title);

    let mut builder = gtk::AboutDialog::builder()
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
            builder = builder.logo(&icon.inner.texture());
        }
    }

    let dialog = builder.build();

    if let Some(titlebar) = dialog
        .titlebar()
        .and_then(|titlebar| titlebar.downcast::<gtk::HeaderBar>().ok())
    {
        let title_label = gtk::Label::new(Some(&title));
        titlebar.set_title_widget(Some(&title_label));
    }

    dialog.present();
}

fn default_gtk_application() -> gtk::Application {
    gio::Application::default()
        .and_then(|app| app.downcast::<gtk::Application>().ok())
        .unwrap_or_default()
}

/// Returns and creates the action group on this application if necessary.
fn action_group_from_app(app: &gtk::Application) -> gio::SimpleActionGroup {
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

fn get_cursor_pos(window: &gtk::Window) -> (i32, i32) {
    WidgetExt::display(window)
        .default_seat()
        .and_then(|s| s.pointer())
        .map(|p| {
            let (_, x, y) = p.surface_at_position();
            (x as _, y as _)
        })
        .unwrap_or_default()
}

fn scale_factor(window: &gtk::Window) -> f64 {
    window
        .surface()
        .map(|surface| surface.scale())
        .unwrap_or_else(|| window.scale_factor() as f64)
}

fn run_context_menu(
    context_menu: &gtk::PopoverMenu,
    items: &[MenuItemKind],
    x: i32,
    y: i32,
) -> bool {
    let main_loop = glib::MainLoop::new(None, false);
    let selected = Rc::new(Cell::new(false));

    // Connect handlers on each menu item action so we can detect when the user selects
    // vs the menu closed aka user canceled selection.
    // Either way we quit this main loop from each item handler or the closed handler.
    let handlers = connect_context_menu_action_handlers(items, &main_loop, &selected);
    let closed_handler = context_menu.connect_closed({
        let main_loop = main_loop.clone();
        move |_| {
            let main_loop = main_loop.clone();
            glib::idle_add_local_once(move || main_loop.quit());
        }
    });

    // Show the context menu.
    context_menu.set_pointing_to(Some(&Rectangle::new(x, y, 0, 0)));
    context_menu.popup();

    // Run a nested main loop and wait for the user to select an item or close the menu.
    main_loop.run();

    // Unparent and disconnect handlers.
    context_menu.unparent();
    context_menu.disconnect(closed_handler);
    for (action, handler) in handlers {
        action.disconnect(handler);
    }

    selected.get()
}

fn connect_context_menu_action_handlers(
    items: &[MenuItemKind],
    main_loop: &glib::MainLoop,
    selected: &Rc<Cell<bool>>,
) -> Vec<(gio::SimpleAction, glib::SignalHandlerId)> {
    let mut handlers = Vec::new();

    for item in items {
        connect_context_menu_action_handler(item.as_ref(), main_loop, selected, &mut handlers);
    }

    handlers
}

/// Recursively connect handlers for each menu item action so we can detect when the user selects an item.
fn connect_context_menu_action_handler(
    item: &dyn IsMenuItem,
    main_loop: &glib::MainLoop,
    selected: &Rc<Cell<bool>>,
    handlers: &mut Vec<(gio::SimpleAction, glib::SignalHandlerId)>,
) {
    let kind = item.kind();
    let child = kind.child();

    if matches!(child.item_type(), MenuItemType::Submenu) {
        for item in child.items() {
            connect_context_menu_action_handler(item.as_ref(), main_loop, selected, handlers);
        }
    } else {
        let Some(action) = child.action.as_ref().cloned() else {
            return;
        };

        let selected = selected.clone();
        let main_loop = main_loop.clone();
        let handler = action.connect_activate(move |_, _| {
            selected.set(true);
            main_loop.quit();
        });
        handlers.push((action, handler));
    }
}
