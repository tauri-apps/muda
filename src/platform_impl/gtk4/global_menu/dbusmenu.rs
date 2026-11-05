// Copyright 2026 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Adapts a live GMenuModel to the desktop-independent DBusMenu interface.
use gtk::{gio, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};

const INTERFACE: &str = "com.canonical.dbusmenu";
const XML: &str = r#"<node><interface name="com.canonical.dbusmenu">
<method name="GetLayout"><arg type="i" direction="in"/><arg type="i" direction="in"/><arg type="as" direction="in"/><arg type="u" direction="out"/><arg type="(ia{sv}av)" direction="out"/></method>
<method name="GetGroupProperties"><arg type="ai" direction="in"/><arg type="as" direction="in"/><arg type="a(ia{sv})" direction="out"/></method>
<method name="GetProperty"><arg type="i" direction="in"/><arg type="s" direction="in"/><arg type="v" direction="out"/></method>
<method name="Event"><arg type="i" direction="in"/><arg type="s" direction="in"/><arg type="v" direction="in"/><arg type="u" direction="in"/></method>
<method name="EventGroup"><arg type="a(isvu)" direction="in"/><arg type="ai" direction="out"/></method>
<method name="AboutToShow"><arg type="i" direction="in"/><arg type="b" direction="out"/></method>
<method name="AboutToShowGroup"><arg type="ai" direction="in"/><arg type="ai" direction="out"/><arg type="ai" direction="out"/></method>
<property name="Version" type="u" access="read"/>
<property name="TextDirection" type="s" access="read"/>
<property name="Status" type="s" access="read"/>
<property name="IconThemePath" type="as" access="read"/>
<signal name="LayoutUpdated"><arg type="u"/><arg type="i"/></signal>
<signal name="ItemsPropertiesUpdated"><arg type="a(ia{sv})"/><arg type="a(ias)"/></signal>
</interface></node>"#;

#[derive(Clone, Default)]
struct Node {
    properties: HashMap<String, glib::Variant>,
    action: Option<String>,
    target: Option<glib::Variant>,
    children: Vec<i32>,
}

#[derive(Default)]
struct Tree {
    nodes: HashMap<i32, Node>,
    ids: HashMap<(usize, u64), i32>,
    next_id: i32,
}

#[derive(Default)]
struct Changes {
    layouts: Vec<i32>,
    updated: Vec<(i32, HashMap<String, glib::Variant>)>,
    removed: Vec<(i32, Vec<String>)>,
}

impl Changes {
    fn between(previous: &HashMap<i32, Node>, current: &HashMap<i32, Node>) -> Self {
        let mut changes = Self::default();
        for (&id, node) in current {
            let Some(old) = previous.get(&id) else {
                // New items are included in their parent's layout update.
                if id == 0 {
                    changes.layouts.push(id);
                }
                continue;
            };
            if old.children != node.children {
                changes.layouts.push(id);
            }
            let updated: HashMap<_, _> = node
                .properties
                .iter()
                .filter(|(name, value)| old.properties.get(*name) != Some(*value))
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect();
            if !updated.is_empty() {
                changes.updated.push((id, updated));
            }
            let mut removed: Vec<_> = old
                .properties
                .keys()
                .filter(|name| !node.properties.contains_key(*name))
                .cloned()
                .collect();
            if !removed.is_empty() {
                removed.sort();
                changes.removed.push((id, removed));
            }
        }
        changes.layouts.sort_unstable();
        changes.updated.sort_by_key(|(id, _)| *id);
        changes.removed.sort_by_key(|(id, _)| *id);
        changes
    }
}

impl Tree {
    fn id(&mut self, model: &gio::MenuModel, index: i32) -> i32 {
        let key = model
            .item_attribute_value(index, super::super::INTERNAL_ID_ATTRIBUTE, None)
            .and_then(|v| v.get::<u64>())
            .map(|id| (0, id))
            .unwrap_or((model.as_ptr() as usize, index as u64));
        *self.ids.entry(key).or_insert_with(|| {
            self.next_id += 1;
            self.next_id
        })
    }

    fn rebuild(
        &mut self,
        model: &gio::MenuModel,
        actions: &gio::ActionGroup,
    ) -> Vec<gio::MenuModel> {
        self.nodes.clear();
        let mut models = Vec::new();
        let children = self.collect(model, actions, &mut models, &mut HashSet::new());
        self.nodes.insert(
            0,
            Node {
                children,
                ..Default::default()
            },
        );
        models
    }

    fn collect(
        &mut self,
        model: &gio::MenuModel,
        actions: &gio::ActionGroup,
        models: &mut Vec<gio::MenuModel>,
        ancestors: &mut HashSet<usize>,
    ) -> Vec<i32> {
        let address = model.as_ptr() as usize;
        if !ancestors.insert(address) {
            return Vec::new();
        }
        if !models.iter().any(|m| m == model) {
            models.push(model.clone());
        }
        let mut children = Vec::new();
        for index in 0..model.n_items() {
            let id = self.id(model, index);
            let mut node = Node::default();
            if let Some(section) = model.item_link(index, gio::MENU_LINK_SECTION) {
                let section_children = self.collect(&section, actions, models, ancestors);
                if !children.is_empty() && !section_children.is_empty() {
                    node.properties
                        .insert("type".into(), "separator".to_variant());
                    self.nodes.insert(id, node);
                    children.push(id);
                }
                children.extend(section_children);
                continue;
            }
            if let Some(label) = model.item_attribute_value(index, gio::MENU_ATTRIBUTE_LABEL, None)
            {
                node.properties.insert("label".into(), label);
            }
            node.action = model
                .item_attribute_value(index, gio::MENU_ATTRIBUTE_ACTION, None)
                .and_then(|v| v.get::<String>())
                .and_then(|s| s.strip_prefix("app.").map(str::to_owned));
            node.target = model.item_attribute_value(index, gio::MENU_ATTRIBUTE_TARGET, None);
            if let Some(action) = &node.action {
                node.properties.insert(
                    "enabled".into(),
                    actions.is_action_enabled(action).to_variant(),
                );
                if let Some(state) = actions.action_state(action) {
                    if let Some(checked) = state.get::<bool>() {
                        node.properties
                            .insert("toggle-type".into(), "checkmark".to_variant());
                        node.properties
                            .insert("toggle-state".into(), i32::from(checked).to_variant());
                    } else if let Some(target) = &node.target {
                        node.properties
                            .insert("toggle-type".into(), "radio".to_variant());
                        node.properties.insert(
                            "toggle-state".into(),
                            i32::from(&state == target).to_variant(),
                        );
                    }
                }
            }
            if let Some(submenu) = model.item_link(index, gio::MENU_LINK_SUBMENU) {
                node.properties
                    .insert("children-display".into(), "submenu".to_variant());
                node.children = self.collect(&submenu, actions, models, ancestors);
            } else if node.action.is_none() && !node.properties.contains_key("label") {
                node.properties
                    .insert("type".into(), "separator".to_variant());
            }
            if let Some(icon) = model
                .item_attribute_value(index, gio::MENU_ATTRIBUTE_ICON, None)
                .and_then(|v| gio::Icon::deserialize(&v))
            {
                if let Some(icon) = icon.downcast_ref::<gio::ThemedIcon>() {
                    if let Some(name) = icon.names().first() {
                        node.properties
                            .insert("icon-name".into(), name.to_variant());
                    }
                } else if let Some(icon) = icon.downcast_ref::<gio::BytesIcon>() {
                    node.properties
                        .insert("icon-data".into(), icon.bytes().as_ref().to_variant());
                }
            }
            if let Some(accel) = model
                .item_attribute_value(index, "accel", None)
                .and_then(|v| v.get::<String>())
            {
                let mut keys = Vec::new();
                if let Some((key, mods)) = gtk::accelerator_parse(&accel) {
                    for (mask, name) in [
                        (gtk::gdk::ModifierType::CONTROL_MASK, "Control"),
                        (gtk::gdk::ModifierType::ALT_MASK, "Alt"),
                        (gtk::gdk::ModifierType::SHIFT_MASK, "Shift"),
                        (gtk::gdk::ModifierType::SUPER_MASK, "Super"),
                    ] {
                        if mods.contains(mask) {
                            keys.push(name.to_owned());
                        }
                    }
                    if let Some(name) = key.name() {
                        keys.push(name.to_string());
                    }
                    node.properties
                        .insert("shortcut".into(), vec![keys].to_variant());
                }
            }
            self.nodes.insert(id, node);
            children.push(id);
        }
        ancestors.remove(&address);
        children
    }

    fn properties(&self, id: i32, names: &[String]) -> Option<HashMap<String, glib::Variant>> {
        Some(
            self.nodes
                .get(&id)?
                .properties
                .iter()
                .filter(|(key, _)| names.is_empty() || names.contains(key))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        )
    }

    fn layout(&self, id: i32, depth: i32, names: &[String]) -> Option<glib::Variant> {
        let node = self.nodes.get(&id)?;
        let children: Vec<_> = if depth == 0 {
            Vec::new()
        } else {
            node.children
                .iter()
                .filter_map(|id| self.layout(*id, if depth < 0 { -1 } else { depth - 1 }, names))
                .collect()
        };
        Some((id, self.properties(id, names)?, children).to_variant())
    }
}

struct State {
    model: gio::MenuModel,
    actions: gio::ActionGroup,
    connection: gio::DBusConnection,
    path: String,
    tree: RefCell<Tree>,
    revision: Cell<u32>,
    models: RefCell<Vec<(gio::MenuModel, glib::SignalHandlerId)>>,
}

impl State {
    fn refresh(self: &Rc<Self>) {
        for (model, handler) in self.models.borrow_mut().drain(..) {
            model.disconnect(handler);
        }
        let (models, changes) = {
            let mut tree = self.tree.borrow_mut();
            let previous = std::mem::take(&mut tree.nodes);
            let models = tree.rebuild(&self.model, &self.actions);
            (models, Changes::between(&previous, &tree.nodes))
        };
        for model in models {
            let weak = Rc::downgrade(self);
            let handler = model.connect_items_changed(move |_, _, _, _| {
                if let Some(state) = weak.upgrade() {
                    state.refresh();
                }
            });
            self.models.borrow_mut().push((model, handler));
        }
        if !changes.updated.is_empty() || !changes.removed.is_empty() {
            let _ = self.connection.emit_signal(
                None,
                &self.path,
                INTERFACE,
                "ItemsPropertiesUpdated",
                Some(&(changes.updated, changes.removed).to_variant()),
            );
        }
        if !changes.layouts.is_empty() {
            // Importers cache submenu layouts independently. Invalidating only
            // the root leaves an unchanged top-level item's submenu stale.
            self.notify_layout(&changes.layouts);
        }
    }

    fn notify_layout(&self, parents: &[i32]) {
        if parents.is_empty() {
            return;
        }
        self.revision.set(self.revision.get().wrapping_add(1));
        for &parent in parents {
            let _ = self.connection.emit_signal(
                None,
                &self.path,
                INTERFACE,
                "LayoutUpdated",
                Some(&(self.revision.get(), parent).to_variant()),
            );
        }
    }

    fn event(&self, id: i32, event: &str) -> bool {
        let node = self.tree.borrow().nodes.get(&id).cloned();
        let Some(node) = node else {
            return false;
        };
        if event == "clicked" {
            if let Some(action) = node.action {
                if self.actions.is_action_enabled(&action) {
                    self.actions.activate_action(&action, node.target.as_ref());
                }
            }
        }
        true
    }

    fn call(&self, method: &str, params: &glib::Variant) -> Option<glib::Variant> {
        match method {
            "GetLayout" => {
                let (id, depth, names) = params.get::<(i32, i32, Vec<String>)>()?;
                let layout = self.tree.borrow().layout(id, depth, &names)?;
                Some(glib::Variant::tuple_from_iter([
                    self.revision.get().to_variant(),
                    layout,
                ]))
            }
            "GetGroupProperties" => {
                let (mut ids, names) = params.get::<(Vec<i32>, Vec<String>)>()?;
                let tree = self.tree.borrow();
                if ids.is_empty() {
                    ids.extend(tree.nodes.keys().copied());
                    ids.sort_unstable();
                }
                let properties: Vec<_> = ids
                    .into_iter()
                    .filter_map(|id| Some((id, tree.properties(id, &names)?)))
                    .collect();
                Some((properties,).to_variant())
            }
            "GetProperty" => {
                let (id, name) = params.get::<(i32, String)>()?;
                let tree = self.tree.borrow();
                let node = tree.nodes.get(&id)?;
                let value =
                    node.properties
                        .get(&name)
                        .cloned()
                        .or_else(|| match name.as_str() {
                            "type" => Some("standard".to_variant()),
                            "enabled" | "visible" => Some(true.to_variant()),
                            "label" | "icon-name" | "toggle-type" | "children-display" => {
                                Some("".to_variant())
                            }
                            "toggle-state" => Some((-1i32).to_variant()),
                            "icon-data" => Some(Vec::<u8>::new().to_variant()),
                            "shortcut" => Some(Vec::<Vec<String>>::new().to_variant()),
                            _ => None,
                        })?;
                Some((value,).to_variant())
            }
            "Event" => {
                let (id, event, _, _) = params.get::<(i32, String, glib::Variant, u32)>()?;
                self.event(id, &event).then(|| ().to_variant())
            }
            "EventGroup" => {
                let (events,) = params.get::<(Vec<(i32, String, glib::Variant, u32)>,)>()?;
                let errors: Vec<_> = events
                    .into_iter()
                    .filter_map(|(id, event, _, _)| (!self.event(id, &event)).then_some(id))
                    .collect();
                Some((errors,).to_variant())
            }
            "AboutToShow" => {
                let (id,) = params.get::<(i32,)>()?;
                self.tree
                    .borrow()
                    .nodes
                    .contains_key(&id)
                    .then(|| (true,).to_variant())
            }
            "AboutToShowGroup" => {
                let (ids,) = params.get::<(Vec<i32>,)>()?;
                let tree = self.tree.borrow();
                let (updates, errors): (Vec<_>, Vec<_>) =
                    ids.into_iter().partition(|id| tree.nodes.contains_key(id));
                Some((updates, errors).to_variant())
            }
            _ => None,
        }
    }
}

pub(super) struct Export {
    state: Rc<State>,
    registration: Option<gio::RegistrationId>,
    handlers: Vec<glib::SignalHandlerId>,
}

impl Export {
    pub(super) fn new(
        connection: &gio::DBusConnection,
        path: &str,
        model: &gio::Menu,
        actions: &gtk::Application,
    ) -> Option<Self> {
        let state = Rc::new(State {
            model: model.clone().upcast(),
            actions: actions.clone().upcast(),
            connection: connection.clone(),
            path: path.to_owned(),
            tree: RefCell::new(Tree::default()),
            revision: Cell::new(0),
            models: RefCell::new(Vec::new()),
        });
        let weak = Rc::downgrade(&state);
        let info = gio::DBusNodeInfo::for_xml(XML)
            .ok()?
            .lookup_interface(INTERFACE)?;
        let registration = connection
            .register_object(path, &info)
            .method_call(move |_, _, _, _, method, params, invocation| {
                if let Some((state, value)) = weak
                    .upgrade()
                    .and_then(|state| state.call(method, &params).map(|value| (state, value)))
                {
                    let refresh = match method {
                        "AboutToShow" => params.get::<(i32,)>().map(|(id,)| vec![id]),
                        "AboutToShowGroup" => value
                            .get::<(Vec<i32>, Vec<i32>)>()
                            .map(|(updates, _)| updates),
                        _ => None,
                    };
                    invocation.return_value(Some(&value));
                    if let Some(parents) = refresh {
                        // Pair the requested refresh with a layout notification.
                        // Send the reply first so importers can coalesce the two
                        // without dropping the next real application update.
                        state.notify_layout(&parents);
                    }
                } else {
                    invocation.return_dbus_error(
                        "com.canonical.dbusmenu.Error.InvalidMenuItem",
                        "Unknown menu item or method arguments",
                    );
                }
            })
            .property(|_, _, _, _, property| match property {
                "Version" => 4u32.to_variant(),
                "TextDirection" => if gtk::Widget::default_direction() == gtk::TextDirection::Rtl {
                    "rtl"
                } else {
                    "ltr"
                }
                .to_variant(),
                "Status" => "normal".to_variant(),
                _ => Vec::<String>::new().to_variant(),
            })
            .build()
            .ok()?;
        state.refresh();
        let weak = Rc::downgrade(&state);
        let enabled = actions.connect_action_enabled_changed(None, move |_, _, _| {
            if let Some(state) = weak.upgrade() {
                state.refresh();
            }
        });
        let weak = Rc::downgrade(&state);
        let changed = actions.connect_action_state_changed(None, move |_, _, _| {
            if let Some(state) = weak.upgrade() {
                state.refresh();
            }
        });
        let weak = Rc::downgrade(&state);
        let added = actions.connect_action_added(None, move |_, _| {
            if let Some(state) = weak.upgrade() {
                state.refresh();
            }
        });
        let weak = Rc::downgrade(&state);
        let removed = actions.connect_action_removed(None, move |_, _| {
            if let Some(state) = weak.upgrade() {
                state.refresh();
            }
        });
        Some(Self {
            state,
            registration: Some(registration),
            handlers: vec![enabled, changed, added, removed],
        })
    }
}

impl Drop for Export {
    fn drop(&mut self) {
        for handler in self.handlers.drain(..) {
            self.state.actions.disconnect(handler);
        }
        for (model, handler) in self.state.models.borrow_mut().drain(..) {
            model.disconnect(handler);
        }
        if let Some(id) = self.registration.take() {
            let _ = self.state.connection.unregister_object(id);
        }
    }
}
