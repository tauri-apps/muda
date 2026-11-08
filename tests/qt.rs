// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Tests of the Qt backend (`qt` feature) against a real Qt application, offscreen, one process
//! per case (Qt allows one `QApplication` per process, on its main thread; see
//! `qt-test-harness`).
//!
//! ```sh
//! RUSTFLAGS="--cfg muda_qt_tests" cargo test --no-default-features --features qt --test qt
//! ```
//!
//! The `muda_qt_tests` cfg pulls in the Qt dev-dependencies (`qtshim`, `qt-test-harness`) and
//! leaves out the GTK 4 one, so other backends' tests don't need Qt and these don't need GTK 4.

#![allow(unexpected_cfgs)]

#[cfg(not(muda_qt_tests))]
compile_error!("the Qt tests need `RUSTFLAGS=\"--cfg muda_qt_tests\"` (see tests/qt.rs)");

use std::{
    cell::RefCell,
    ffi::c_void,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use dpi::{LogicalPosition, PhysicalPosition, Position};
use muda::{
    accelerator::{Accelerator, Code, Key, KeyAccelerator, Modifiers, NamedKey},
    AboutMetadata, CheckMenuItem, ContextMenu, Error, Icon, IconMenuItem, Menu, MenuEvent,
    MenuItem, NativeIcon, PredefinedMenuItem, Submenu,
};
use qt_test_harness::Case;
use qtshim::{
    app,
    menu::{debug, Widget},
    webengine::Window,
};

fn main() {
    let mut cases = Vec::new();
    cases.extend(build_cases());
    cases.extend(update_cases());
    cases.extend(click_cases());
    cases.extend(lifecycle_cases());
    cases.extend(invalid_cases());
    cases.extend(context_cases());
    qt_test_harness::main(cases, teardown);
}

// -------------------------------------------------------------------------------------------
// helpers (from qtshim's `tests/qt/common.rs`; the harness crate knows nothing about Qt)

/// How long [`wait_for`] waits.
const WAIT: Duration = Duration::from_secs(20);

fn init() {
    app::init(Some("muda-qt-test"));
}

fn teardown() {
    if app::is_running() {
        app::shutdown();
    }
}

thread_local! {
    static COND: RefCell<Option<Box<dyn FnMut() -> bool>>> = const { RefCell::new(None) };
}

/// Run the event loop until `cond` returns `true` or `timeout` passes. Returns whether `cond`
/// became true.
fn run_until(timeout: Duration, cond: impl FnMut() -> bool + 'static) -> bool {
    let deadline = Instant::now() + timeout;
    let met = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    COND.with(|c| *c.borrow_mut() = Some(Box::new(cond)));
    let ticker = {
        let (met, done) = (met.clone(), done.clone());
        thread::spawn(move || {
            while !done.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(5));
                let (met, done) = (met.clone(), done.clone());
                let queued = app::dispatch(move || {
                    if done.load(Ordering::Acquire) {
                        return;
                    }
                    let ok = COND.with(|c| c.borrow_mut().as_mut().is_some_and(|f| f()));
                    if ok || Instant::now() >= deadline {
                        met.store(ok, Ordering::Release);
                        done.store(true, Ordering::Release);
                        app::quit(0);
                    }
                });
                if !queued {
                    break;
                }
            }
        })
    };
    app::exec();
    done.store(true, Ordering::Release);
    ticker.join().unwrap();
    COND.with(|c| c.borrow_mut().take());
    met.load(Ordering::Acquire)
}

/// [`run_until`] with [`WAIT`], panicking with `what` on timeout.
#[track_caller]
fn wait_for(what: &str, cond: impl FnMut() -> bool + 'static) {
    assert!(run_until(WAIT, cond), "timed out waiting for {what}");
}

/// Run the event loop for `duration`.
fn pump(duration: Duration) {
    run_until(duration, || false);
}

/// Let posted rebuilds run (they're queued with `app::post`, so a short pump is enough).
fn settle() {
    pump(Duration::from_millis(50));
}

/// A window and its widget handle, with the raw pointer muda takes.
struct Win {
    window: Window,
    widget: Widget,
}

impl Win {
    fn new(id: u64) -> Self {
        let window = Window::new(id, false);
        let widget = window.widget();
        Self { window, widget }
    }

    fn shown(id: u64) -> Self {
        let w = Self::new(id);
        w.window.set_visible(true);
        w
    }

    fn ptr(&self) -> *mut c_void {
        self.window.widget_ptr().as_ptr()
    }

    fn tree(&self) -> String {
        debug::menubar_tree(&self.widget)
    }

    /// Wait until the menubar tree satisfies `pred`.
    #[track_caller]
    fn wait_tree(&self, what: &str, pred: impl Fn(&str) -> bool + 'static) {
        let widget = self.widget.clone();
        let ok = run_until(WAIT, move || pred(&debug::menubar_tree(&widget)));
        assert!(
            ok,
            "timed out waiting for {what}; menubar:\n{}",
            self.tree()
        );
    }
}

fn ctrl(code: Code) -> Option<Accelerator> {
    Some(Accelerator::new(Modifiers::CONTROL, code))
}

fn icon() -> Icon {
    Icon::from_rgba(vec![255; 16 * 16 * 4], 16, 16).unwrap()
}

/// Drain the menu events received so far.
fn take_events() -> Vec<String> {
    MenuEvent::receiver()
        .try_iter()
        .map(|e| e.id().0.clone())
        .collect()
}

/// A pointer that is certainly not a widget (a Rust heap allocation).
fn not_a_widget() -> *mut c_void {
    Box::leak(Box::new([0u64; 16])).as_mut_ptr().cast()
}

// -------------------------------------------------------------------------------------------
// building

fn build_cases() -> Vec<Case> {
    vec![
        Case::pass(
            "build: init_for_qt_window builds every item kind",
            build_all_kinds,
        ),
        Case::pass("build: nested submenus", build_nested),
        Case::pass(
            "build: every predefined item (macOS-only ones disabled)",
            build_predefined,
        ),
        Case::pass(
            "build: menubar registers shortcuts, editing items only display theirs",
            build_shortcuts,
        ),
        Case::pass(
            "build: key accelerators and unmappable keys",
            build_key_accelerators,
        ),
        Case::pass(
            "build: every mapped key is a valid Qt shortcut",
            build_every_key,
        ),
        Case::pass("build: empty menu gives an empty menubar", build_empty),
        Case::pass("build: one menu on two windows", build_two_windows),
        Case::pass("build: menu built before app::init", build_before_init),
        Case::pass(
            "build: attach to a non-window widget",
            build_non_window_widget,
        ),
    ]
}

fn build_all_kinds() {
    init();
    let w = Win::new(1);
    let menu = Menu::new();
    let file = Submenu::with_items(
        "&File",
        true,
        &[
            &MenuItem::with_id("open", "&Open", true, ctrl(Code::KeyO)),
            &MenuItem::with_id("off", "Disabled", false, None),
            &CheckMenuItem::with_id("chk", "Check", true, true, None),
            &CheckMenuItem::with_id("unchk", "Uncheck", false, false, None),
            &IconMenuItem::with_id("ico", "Icon", true, Some(icon()), None),
            &IconMenuItem::with_id_and_native_icon(
                "native",
                "Native",
                true,
                Some(NativeIcon::Add),
                None,
            ),
            &PredefinedMenuItem::separator(),
            &MenuItem::new("A && B", true, None),
        ],
    )
    .unwrap();
    menu.append(&file).unwrap();
    menu.append(&Submenu::new("&Disabled", false)).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    // Built synchronously by init.
    assert_eq!(
        w.tree(),
        "File\n  Open <Ctrl+O>\n  Disabled {disabled}\n  [x] Check\n  [ ] Uncheck {disabled}\n  \
         Icon\n  Native\n  ---\n  A & B\nDisabled {disabled}\n"
    );
}

fn build_nested() {
    init();
    let w = Win::new(1);
    let inner = Submenu::with_items(
        "Inner",
        true,
        &[&MenuItem::with_id("deep", "Deep", true, ctrl(Code::KeyD))],
    )
    .unwrap();
    let mid = Submenu::with_items("Mid", true, &[&inner]).unwrap();
    let icon_sub = Submenu::new("WithIcon", true);
    icon_sub.set_icon(Some(icon()));
    icon_sub.append(&MenuItem::new("x", true, None)).unwrap();
    let native_sub = Submenu::new("Native", true);
    native_sub.set_native_icon(Some(NativeIcon::Folder));
    let top = Submenu::with_items("Top", true, &[&mid, &icon_sub, &native_sub]).unwrap();
    let menu = Menu::with_items(&[&top]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    assert_eq!(
        w.tree(),
        "Top\n  Mid\n    Inner\n      Deep <Ctrl+D>\n  WithIcon\n    x\n  Native\n"
    );
    assert!(debug::trigger_menu_item(&w.widget, "Top/Mid/Inner/Deep"));
    assert_eq!(take_events(), ["deep"]);
}

fn build_predefined() {
    init();
    let w = Win::new(1);
    let about = AboutMetadata {
        name: Some("App".into()),
        ..Default::default()
    };
    let edit = Submenu::with_items(
        "Edit",
        true,
        &[
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::paste_and_match_style(None),
            &PredefinedMenuItem::delete(None),
            &PredefinedMenuItem::select_all(None),
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(None),
            &PredefinedMenuItem::zoom(None),
            &PredefinedMenuItem::actual_size(None),
            &PredefinedMenuItem::zoom_in(None),
            &PredefinedMenuItem::zoom_out(None),
            &PredefinedMenuItem::fullscreen(None),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::close_window(None),
            &PredefinedMenuItem::quit(None),
            &PredefinedMenuItem::about(None, Some(about)),
            &PredefinedMenuItem::about(Some("About None"), None),
            &PredefinedMenuItem::services(None),
            &PredefinedMenuItem::bring_all_to_front(None),
            &PredefinedMenuItem::start_speaking(None),
            &PredefinedMenuItem::stop_speaking(None),
            &PredefinedMenuItem::start_dictation(None),
            &PredefinedMenuItem::emoji_and_symbols(None),
            &PredefinedMenuItem::section_header("Header"),
            &PredefinedMenuItem::copy(Some("My Copy")),
        ],
    )
    .unwrap();
    let menu = Menu::with_items(&[&edit]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    let tree = w.tree();
    let lines: Vec<&str> = tree.lines().collect();
    let expected = [
        "Edit",
        "  ---",
        "  Copy (Ctrl+C)",
        "  Cut (Ctrl+X)",
        "  Paste (Ctrl+V)",
        "  Paste and Match Style {disabled}",
        "  Delete",
        "  Select All (Ctrl+A)",
        "  Undo (Ctrl+Z)",
        "  Redo (Ctrl+Y)",
        "  Minimize <Ctrl+M>",
        "  Maximize",
        "  Maximize",
        "  Actual Size {disabled}",
        "  Zoom In {disabled}",
        "  Zoom Out {disabled}",
        "  Toggle Full Screen",
        "  Hide <Ctrl+H>",
        "  Hide Others {disabled}",
        "  Show All {disabled}",
        "  Close Window <Alt+F4>",
        "  Quit",
        "  About",
        "  About None",
        "  Services {disabled}",
        "  Bring All to Front {disabled}",
        "  Start Speaking {disabled}",
        "  Stop Speaking {disabled}",
        "  Start Dictation… {disabled}",
    ];
    assert_eq!(&lines[..expected.len()], &expected, "{tree}");
    let rest = &lines[expected.len()..];
    // "Emoji & Symbols" (a lone `&` is a mnemonic marker), the section header, the custom copy.
    assert_eq!(rest.len(), 3, "{tree}");
    assert!(rest[0].ends_with("{disabled}"), "{tree}");
    assert_eq!(rest[1], "  Header {disabled}", "{tree}");
    assert_eq!(rest[2], "  My Copy (Ctrl+C)", "{tree}");
}

#[allow(deprecated)] // `Modifiers::SUPER` maps to Meta too
fn build_shortcuts() {
    init();
    let w = Win::new(1);
    let file = Submenu::with_items(
        "File",
        true,
        &[
            &MenuItem::new(
                "Save As",
                true,
                Some(Accelerator::new(
                    Modifiers::CONTROL | Modifiers::SHIFT,
                    Code::KeyS,
                )),
            ),
            &CheckMenuItem::new(
                "Wrap",
                true,
                false,
                Some(Accelerator::new(Modifiers::ALT, Code::KeyW)),
            ),
            &IconMenuItem::new("Print", true, None, ctrl(Code::KeyP)),
            &MenuItem::new(
                "Super",
                true,
                Some(Accelerator::new(Modifiers::SUPER, Code::F5)),
            ),
            &MenuItem::new(
                "All",
                true,
                Some(Accelerator::new(
                    Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::META,
                    Code::Delete,
                )),
            ),
            // No Qt name: shown without a shortcut.
            &MenuItem::new("NoName", true, ctrl(Code::Fn)),
        ],
    )
    .unwrap();
    let menu = Menu::with_items(&[&file]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    // Qt normalizes the modifier order (Meta first).
    assert_eq!(
        w.tree(),
        "File\n  Save As <Ctrl+Shift+S>\n  [ ] Wrap <Alt+W>\n  Print <Ctrl+P>\n  Super <Meta+F5>\n  \
         All <Meta+Ctrl+Alt+Shift+Del>\n  NoName\n"
    );
}

fn build_key_accelerators() {
    init();
    let w = Win::new(1);
    let a = MenuItem::new("A", true, None);
    a.set_key_accelerator(Some(KeyAccelerator::new(
        Modifiers::CONTROL,
        Key::Character("k".into()),
    )))
    .unwrap();
    let b = MenuItem::new("B", true, None);
    b.set_key_accelerator(Some(KeyAccelerator::new(
        Modifiers::SHIFT,
        Key::Named(NamedKey::F2),
    )))
    .unwrap();
    let c = CheckMenuItem::new("C", true, false, None);
    c.set_key_accelerator(Some(KeyAccelerator::new(
        Modifiers::empty(),
        Key::Named(NamedKey::Alt),
    )))
    .unwrap();
    let d = IconMenuItem::new("D", true, None, None);
    d.set_key_accelerator(Some(KeyAccelerator::new(
        Modifiers::CONTROL,
        Key::Character("+".into()),
    )))
    .unwrap();
    let menu = Menu::new();
    let sub = Submenu::with_items("S", true, &[&a, &b, &c, &d]).unwrap();
    menu.append(&sub).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    assert_eq!(
        w.tree(),
        "S\n  A <Ctrl+K>\n  B <Shift+F2>\n  [ ] C\n  D <Ctrl++>\n"
    );
}

/// Every key the backend maps must survive a round trip through `QKeySequence` (a name Qt
/// doesn't know gives an empty shortcut).
fn build_every_key() {
    use muda::accelerator::Code::*;
    init();
    let w = Win::new(1);
    let codes = [
        KeyA,
        KeyM,
        KeyZ,
        Digit0,
        Digit9,
        Numpad0,
        Numpad9,
        Backquote,
        Backslash,
        IntlBackslash,
        BracketLeft,
        BracketRight,
        Comma,
        NumpadComma,
        Equal,
        NumpadEqual,
        Minus,
        NumpadSubtract,
        Period,
        NumpadDecimal,
        Quote,
        Semicolon,
        Slash,
        NumpadDivide,
        NumpadAdd,
        NumpadMultiply,
        NumpadStar,
        Space,
        Tab,
        Enter,
        NumpadEnter,
        Backspace,
        NumpadBackspace,
        Delete,
        Insert,
        Home,
        End,
        PageUp,
        PageDown,
        ArrowUp,
        ArrowDown,
        ArrowLeft,
        ArrowRight,
        Escape,
        PrintScreen,
        ScrollLock,
        Pause,
        CapsLock,
        NumLock,
        ContextMenu,
        Help,
        F1,
        F12,
        F13,
        F24,
        AudioVolumeDown,
        AudioVolumeUp,
        AudioVolumeMute,
        MediaPlayPause,
        MediaStop,
        MediaTrackNext,
        MediaTrackPrevious,
    ];
    let named = [
        NamedKey::Enter,
        NamedKey::Escape,
        NamedKey::ContextMenu,
        NamedKey::Copy,
        NamedKey::Cut,
        NamedKey::Paste,
        NamedKey::Undo,
        NamedKey::Redo,
        NamedKey::Find,
        NamedKey::Open,
        NamedKey::Save,
        NamedKey::New,
        NamedKey::Close,
        NamedKey::Print,
        NamedKey::ZoomIn,
        NamedKey::ZoomOut,
        NamedKey::AudioVolumeMute,
        NamedKey::MediaPlayPause,
        NamedKey::MediaTrackPrevious,
        NamedKey::F24,
    ];
    let sub = Submenu::new("Keys", true);
    for code in codes {
        sub.append(&MenuItem::new(
            format!("{code:?}"),
            true,
            Some(Accelerator::new(Modifiers::CONTROL, code)),
        ))
        .unwrap();
    }
    for key in named {
        let item = MenuItem::new(format!("Named{key:?}"), true, None);
        item.set_key_accelerator(Some(KeyAccelerator::new(Modifiers::ALT, Key::Named(key))))
            .unwrap();
        sub.append(&item).unwrap();
    }
    for c in ["q", "Q", "5", "\\", "é", " "] {
        let item = MenuItem::new(format!("Char {c}"), true, None);
        item.set_key_accelerator(Some(KeyAccelerator::new(
            Modifiers::SHIFT,
            Key::Character(c.into()),
        )))
        .unwrap();
        sub.append(&item).unwrap();
    }
    let menu = Menu::with_items(&[&sub]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    let tree = w.tree();
    let items: Vec<&str> = tree.lines().skip(1).collect();
    assert_eq!(items.len(), codes.len() + named.len() + 6, "{tree}");
    for line in &items {
        assert!(line.ends_with('>'), "no shortcut for {line:?}:\n{tree}");
    }
    assert!(tree.contains("  KeyA <Ctrl+A>\n"), "{tree}");
    assert!(tree.contains("  Char é <Shift+É>\n"), "{tree}");
    assert!(tree.contains("  Char   <Shift+Space>\n"), "{tree}");
}

fn build_empty() {
    init();
    let w = Win::new(1);
    let menu = Menu::new();
    menu.init_for_qt_window(w.ptr()).unwrap();
    assert_eq!(w.tree(), "");
    // An (empty) bar is installed: a second menu is refused.
    let other = Menu::new();
    assert!(matches!(
        other.init_for_qt_window(w.ptr()),
        Err(Error::AlreadyInitialized)
    ));
    menu.append(&MenuItem::new("Late", true, None)).unwrap();
    w.wait_tree("the appended item", |t| t == "Late\n");
}

fn build_two_windows() {
    init();
    let (w1, w2) = (Win::new(1), Win::new(2));
    let item = MenuItem::with_id("i", "Item", true, None);
    let menu = Menu::with_items(&[&item]).unwrap();
    menu.init_for_qt_window(w1.ptr()).unwrap();
    menu.init_for_qt_window(w2.ptr()).unwrap();
    assert_eq!(w1.tree(), "Item\n");
    assert_eq!(w2.tree(), "Item\n");
    item.set_text("Renamed");
    let widget2 = w2.widget.clone();
    w1.wait_tree("both windows renamed", move |t| {
        t == "Renamed\n" && debug::menubar_tree(&widget2) == "Renamed\n"
    });
    assert!(debug::trigger_menu_item(&w2.widget, "Renamed"));
    assert_eq!(take_events(), ["i"]);
    menu.remove_for_qt_window(w1.ptr()).unwrap();
    assert_eq!(w1.tree(), "");
    item.set_text("Again");
    w2.wait_tree("the remaining window updated", |t| t == "Again\n");
    assert_eq!(w1.tree(), "");
}

fn build_before_init() {
    // Items and menus can be created and changed before the application exists.
    let item = MenuItem::new("Before", true, None);
    let check = CheckMenuItem::new("Check", true, false, None);
    let sub = Submenu::with_items("Sub", true, &[&item, &check]).unwrap();
    let menu = Menu::with_items(&[&sub]).unwrap();
    item.set_text("Changed");
    check.set_checked(true);
    sub.append(&MenuItem::new("Appended", true, None)).unwrap();
    init();
    let w = Win::new(1);
    menu.init_for_qt_window(w.ptr()).unwrap();
    assert_eq!(w.tree(), "Sub\n  Changed\n  [x] Check\n  Appended\n");
}

fn build_non_window_widget() {
    init();
    let w = Win::new(1);
    let menu = Menu::with_items(&[&MenuItem::new("Item", true, None)]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    // The menubar itself is a widget of the application: a menu can go into it too (the bar
    // gets a layout). Unusual, but must not crash.
    let other = Menu::with_items(&[&MenuItem::new("Inner", true, None)]).unwrap();
    let _ = other.init_for_qt_window(w.ptr());
    assert_eq!(w.tree(), "Item\n");
}

// -------------------------------------------------------------------------------------------
// live updates

fn update_cases() -> Vec<Case> {
    vec![
        Case::pass("update: set_text on every item kind", update_text),
        Case::pass("update: set_enabled", update_enabled),
        Case::pass("update: set_checked", update_checked),
        Case::pass("update: set_icon (no crash, item kept)", update_icon),
        Case::pass("update: set_accelerator", update_accelerator),
        Case::pass("update: append, prepend, insert and remove", update_items),
        Case::pass("update: nested submenu changes", update_nested),
        Case::pass(
            "update: many changes coalesce into one rebuild",
            update_coalesced,
        ),
    ]
}

/// A window with `menu` installed.
fn installed(menu: &Menu) -> Win {
    let w = Win::new(1);
    menu.init_for_qt_window(w.ptr()).unwrap();
    w
}

fn update_text() {
    init();
    let item = MenuItem::new("Item", true, ctrl(Code::KeyI));
    let check = CheckMenuItem::new("Check", true, false, None);
    let icon_item = IconMenuItem::new("Icon", true, None, None);
    let copy = PredefinedMenuItem::copy(None);
    let sub = Submenu::with_items("Sub", true, &[&item, &check, &icon_item, &copy]).unwrap();
    let menu = Menu::with_items(&[&sub]).unwrap();
    let w = installed(&menu);
    item.set_text("Item2");
    check.set_text("Check2");
    icon_item.set_text("Icon2");
    copy.set_text("Copy2");
    sub.set_text("&Sub2");
    w.wait_tree("renamed items", |t| {
        t == "Sub2\n  Item2 <Ctrl+I>\n  [ ] Check2\n  Icon2\n  Copy2 (Ctrl+C)\n"
    });
    item.set_styled_text([("Styled", muda::TextStyle::Secondary)]);
    w.wait_tree("styled text", |t| t.contains("  Styled <Ctrl+I>\n"));
}

fn update_enabled() {
    init();
    let item = MenuItem::new("Item", true, None);
    let check = CheckMenuItem::new("Check", true, false, None);
    let icon_item = IconMenuItem::new("Icon", true, None, None);
    let sub = Submenu::with_items("Sub", true, &[&item, &check, &icon_item]).unwrap();
    let menu = Menu::with_items(&[&sub]).unwrap();
    let w = installed(&menu);
    item.set_enabled(false);
    check.set_enabled(false);
    icon_item.set_enabled(false);
    w.wait_tree("disabled items", |t| {
        t == "Sub\n  Item {disabled}\n  [ ] Check {disabled}\n  Icon {disabled}\n"
    });
    // A disabled item does nothing when triggered.
    assert!(debug::trigger_menu_item(&w.widget, "Sub/Item"));
    assert!(take_events().is_empty());
    sub.set_enabled(false);
    w.wait_tree("disabled submenu", |t| t.starts_with("Sub {disabled}\n"));
    sub.set_enabled(true);
    item.set_enabled(true);
    w.wait_tree("re-enabled", |t| t.starts_with("Sub\n  Item\n"));
}

fn update_checked() {
    init();
    let check = CheckMenuItem::new("Check", true, false, None);
    let menu = Menu::with_items(&[&check]).unwrap();
    let w = installed(&menu);
    assert_eq!(w.tree(), "[ ] Check\n");
    check.set_checked(true);
    w.wait_tree("checked", |t| t == "[x] Check\n");
    assert!(check.is_checked());
    check.set_checked(false);
    w.wait_tree("unchecked", |t| t == "[ ] Check\n");
}

fn update_icon() {
    init();
    let icon_item = IconMenuItem::new("Icon", true, None, None);
    let sub = Submenu::with_items("Sub", true, &[&icon_item]).unwrap();
    let menu = Menu::with_items(&[&sub]).unwrap();
    let w = installed(&menu);
    icon_item.set_icon(Some(icon()));
    sub.set_icon(Some(icon()));
    settle();
    icon_item.set_native_icon(Some(NativeIcon::Add));
    sub.set_native_icon(Some(NativeIcon::Add));
    settle();
    icon_item.set_icon(None);
    sub.set_icon(None);
    icon_item.set_text("Icon2");
    w.wait_tree("the item after icon changes", |t| t == "Sub\n  Icon2\n");
}

fn update_accelerator() {
    init();
    let item = MenuItem::new("Item", true, None);
    let check = CheckMenuItem::new("Check", true, false, None);
    let icon_item = IconMenuItem::new("Icon", true, None, None);
    let menu = Menu::with_items(&[&item, &check, &icon_item]).unwrap();
    let w = installed(&menu);
    item.set_accelerator(ctrl(Code::KeyN)).unwrap();
    check.set_accelerator(ctrl(Code::KeyK)).unwrap();
    icon_item.set_accelerator(ctrl(Code::KeyL)).unwrap();
    w.wait_tree("accelerators", |t| {
        t == "Item <Ctrl+N>\n[ ] Check <Ctrl+K>\nIcon <Ctrl+L>\n"
    });
    item.set_accelerator(None).unwrap();
    w.wait_tree("accelerator removed", |t| t.starts_with("Item\n"));
}

fn update_items() {
    init();
    let a = MenuItem::new("A", true, None);
    let menu = Menu::with_items(&[&a]).unwrap();
    let w = installed(&menu);
    menu.append(&MenuItem::new("C", true, None)).unwrap();
    menu.prepend(&MenuItem::new("Z", true, None)).unwrap();
    menu.insert(&MenuItem::new("B", true, None), 2).unwrap();
    w.wait_tree("added items", |t| t == "Z\nA\nB\nC\n");
    menu.remove(&a).unwrap();
    w.wait_tree("removed A", |t| t == "Z\nB\nC\n");
    menu.remove_at(0).unwrap();
    w.wait_tree("removed Z", |t| t == "B\nC\n");
    assert!(menu.remove_at(10).is_none());
    menu.append_items(&[&PredefinedMenuItem::separator(), &a])
        .unwrap();
    w.wait_tree("appended items", |t| t == "B\nC\n---\nA\n");
    assert!(menu.remove(&MenuItem::new("x", true, None)).is_err());
}

fn update_nested() {
    init();
    let inner = Submenu::new("Inner", true);
    let outer = Submenu::with_items("Outer", true, &[&inner]).unwrap();
    let menu = Menu::with_items(&[&outer]).unwrap();
    let w = installed(&menu);
    let deep = MenuItem::with_id("deep", "Deep", true, None);
    inner.append(&deep).unwrap();
    inner
        .prepend(&CheckMenuItem::new("Chk", true, true, None))
        .unwrap();
    w.wait_tree("inner items", |t| {
        t == "Outer\n  Inner\n    [x] Chk\n    Deep\n"
    });
    inner.insert(&PredefinedMenuItem::separator(), 1).unwrap();
    inner.remove_at(0).unwrap();
    deep.set_text("Deeper");
    w.wait_tree("inner changes", |t| {
        t == "Outer\n  Inner\n    ---\n    Deeper\n"
    });
    outer.remove(&inner).unwrap();
    w.wait_tree("inner removed", |t| t == "Outer\n");
    // Changes to a detached submenu don't show up anywhere.
    inner.append(&MenuItem::new("Gone", true, None)).unwrap();
    settle();
    assert_eq!(w.tree(), "Outer\n");
    assert!(matches!(outer.append(&outer), Err(Error::WouldCreateCycle)));
}

fn update_coalesced() {
    init();
    let item = MenuItem::new("0", true, None);
    let menu = Menu::with_items(&[&item]).unwrap();
    let w = installed(&menu);
    for i in 1..=200 {
        item.set_text(i.to_string());
    }
    w.wait_tree("the last text", |t| t == "200\n");
}

// -------------------------------------------------------------------------------------------
// clicks

fn click_cases() -> Vec<Case> {
    vec![
        Case::pass("click: menu item sends its id", click_item),
        Case::pass("click: check item toggles and sends its id", click_check),
        Case::pass("click: icon item and submenu ids", click_icon),
        Case::pass(
            "click: predefined items act without sending events",
            click_predefined,
        ),
        Case::pass("click: About shows a non-blocking dialog", click_about),
        Case::pass("click: Quit stops the event loop", click_quit).timeout(Duration::from_secs(30)),
        Case::pass(
            "click: window actions on the active window",
            click_window_actions,
        ),
        Case::pass(
            "click: an item removed from the menu no longer exists",
            click_removed,
        ),
    ]
}

fn click_item() {
    init();
    let item = MenuItem::with_id("save", "&Save", true, ctrl(Code::KeyS));
    let file = Submenu::with_items("&File", true, &[&item]).unwrap();
    let menu = Menu::with_items(&[&file]).unwrap();
    let w = installed(&menu);
    assert!(debug::trigger_menu_item(&w.widget, "File/Save"));
    assert!(debug::trigger_menu_item(&w.widget, "File/Save"));
    assert_eq!(take_events(), ["save", "save"]);
    assert!(!debug::trigger_menu_item(&w.widget, "File/Missing"));
}

fn click_check() {
    init();
    let check = CheckMenuItem::with_id("wrap", "Wrap", true, false, None);
    let menu = Menu::with_items(&[&check]).unwrap();
    let w = installed(&menu);
    assert!(debug::trigger_menu_item(&w.widget, "Wrap"));
    assert_eq!(take_events(), ["wrap"]);
    assert!(check.is_checked());
    w.wait_tree("checked", |t| t == "[x] Wrap\n");
    assert!(debug::trigger_menu_item(&w.widget, "Wrap"));
    assert_eq!(take_events(), ["wrap"]);
    assert!(!check.is_checked());
    w.wait_tree("unchecked", |t| t == "[ ] Wrap\n");
}

fn click_icon() {
    init();
    let icon_item = IconMenuItem::with_id("ico", "Ico", true, Some(icon()), None);
    let sub = Submenu::with_id_and_items("sub", "Sub", true, &[&icon_item]).unwrap();
    let menu = Menu::with_items(&[&sub]).unwrap();
    let w = installed(&menu);
    assert!(debug::trigger_menu_item(&w.widget, "Sub/Ico"));
    assert_eq!(take_events(), ["ico"]);
}

fn click_predefined() {
    init();
    let w = Win::shown(1);
    let edit = Submenu::with_items(
        "Edit",
        true,
        &[
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::select_all(None),
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::delete(None),
            &PredefinedMenuItem::about(Some("About Nothing"), None),
            &PredefinedMenuItem::services(None),
        ],
    )
    .unwrap();
    let menu = Menu::with_items(&[&edit]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    for label in [
        "Copy",
        "Cut",
        "Paste",
        "Select All",
        "Undo",
        "Redo",
        "Delete",
        "About Nothing",
        // Disabled: does nothing.
        "Services",
    ] {
        assert!(
            debug::trigger_menu_item(&w.widget, &format!("Edit/{label}")),
            "{label}"
        );
    }
    settle();
    assert!(take_events().is_empty());
}

fn click_about() {
    init();
    let w = Win::shown(1);
    let about = PredefinedMenuItem::about(
        None,
        Some(AboutMetadata {
            name: Some("<App & Co>".into()),
            version: Some("1.2.3".into()),
            short_version: Some("1.2".into()),
            authors: Some(vec!["A".into(), "B".into()]),
            comments: Some("comments".into()),
            copyright: Some("(c) me".into()),
            license: Some("MIT".into()),
            website: Some("https://example.com".into()),
            website_label: Some("site".into()),
            icon: Some(icon()),
            ..Default::default()
        }),
    );
    let minimal = PredefinedMenuItem::about(Some("Minimal"), Some(AboutMetadata::default()));
    let website = PredefinedMenuItem::about(
        Some("Website"),
        Some(AboutMetadata {
            website: Some("https://example.com".into()),
            ..Default::default()
        }),
    );
    let help = Submenu::with_items("Help", true, &[&about, &minimal, &website]).unwrap();
    let menu = Menu::with_items(&[&help]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    assert_eq!(w.tree(), "Help\n  About\n  Minimal\n  Website\n");
    // Non-blocking: returns at once, with the dialog open.
    let start = Instant::now();
    assert!(debug::trigger_menu_item(&w.widget, "Help/About"));
    assert!(debug::trigger_menu_item(&w.widget, "Help/Minimal"));
    assert!(debug::trigger_menu_item(&w.widget, "Help/Website"));
    assert!(start.elapsed() < Duration::from_secs(5));
    settle();
    assert!(take_events().is_empty());
}

fn click_quit() {
    init();
    let w = Win::new(1);
    let menu = Menu::with_items(&[&PredefinedMenuItem::quit(None)]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    let widget = w.widget.clone();
    app::post(move || {
        assert!(debug::trigger_menu_item(&widget, "Quit"));
    });
    // Returns only because the item quit the loop.
    assert_eq!(app::exec(), 0);
}

fn click_window_actions() {
    init();
    let w = Win::shown(1);
    w.window.focus();
    let window = Submenu::with_items(
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(None),
            &PredefinedMenuItem::fullscreen(None),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::close_window(None),
        ],
    )
    .unwrap();
    let menu = Menu::with_items(&[&window]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    settle();
    // Offscreen there may be no active window; these must just not crash.
    for label in [
        "Minimize",
        "Maximize",
        "Maximize",
        "Toggle Full Screen",
        "Toggle Full Screen",
        "Hide",
        "Close Window",
    ] {
        assert!(debug::trigger_menu_item(
            &w.widget,
            &format!("Window/{label}")
        ));
        settle();
    }
    assert!(take_events().is_empty());
}

fn click_removed() {
    init();
    let item = MenuItem::with_id("gone", "Gone", true, None);
    let menu = Menu::with_items(&[&item]).unwrap();
    let w = installed(&menu);
    menu.remove(&item).unwrap();
    w.wait_tree("item removed", |t| t.is_empty());
    assert!(!debug::trigger_menu_item(&w.widget, "Gone"));
    assert!(take_events().is_empty());
}

// -------------------------------------------------------------------------------------------
// lifecycle: init twice, remove, hide/show, drop, destroyed windows, shutdown

fn lifecycle_cases() -> Vec<Case> {
    vec![
        Case::pass("lifecycle: init twice is AlreadyInitialized", init_twice),
        Case::pass(
            "lifecycle: second menu on a window is AlreadyInitialized",
            second_menu,
        ),
        Case::pass("lifecycle: remove, then init again", remove_and_reinit),
        Case::pass(
            "lifecycle: remove/hide/show on a window without the menu",
            not_attached,
        ),
        Case::pass("lifecycle: hide, show and is_visible", hide_show),
        Case::pass(
            "lifecycle: is_visible on a hidden window reports the menubar's own state",
            visible_on_hidden_window,
        ),
        Case::pass("lifecycle: dropping the menu removes the bar", drop_menu),
        Case::pass(
            "lifecycle: window destroyed while the menu is attached",
            window_destroyed,
        ),
        Case::pass(
            "lifecycle: changes and drop after app::shutdown",
            after_shutdown,
        ),
        Case::pass(
            "lifecycle: menu attached at shutdown, dropped afterwards",
            attached_at_shutdown,
        ),
        Case::pass(
            "lifecycle: rebuild pending at shutdown is dropped",
            pending_at_shutdown,
        ),
    ]
}

fn init_twice() {
    init();
    let w = Win::new(1);
    let menu = Menu::with_items(&[&MenuItem::new("Item", true, None)]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    assert!(matches!(
        menu.init_for_qt_window(w.ptr()),
        Err(Error::AlreadyInitialized)
    ));
    assert_eq!(w.tree(), "Item\n");
}

fn second_menu() {
    init();
    let w = Win::new(1);
    let first = Menu::with_items(&[&MenuItem::new("First", true, None)]).unwrap();
    let second = Menu::with_items(&[&MenuItem::new("Second", true, None)]).unwrap();
    first.init_for_qt_window(w.ptr()).unwrap();
    assert!(matches!(
        second.init_for_qt_window(w.ptr()),
        Err(Error::AlreadyInitialized)
    ));
    // Also while hidden.
    first.hide_for_qt_window(w.ptr()).unwrap();
    assert!(matches!(
        second.init_for_qt_window(w.ptr()),
        Err(Error::AlreadyInitialized)
    ));
    // The refused menu doesn't own the window.
    assert!(matches!(
        second.remove_for_qt_window(w.ptr()),
        Err(Error::NotInitialized)
    ));
    assert_eq!(w.tree(), "First\n");
    // Once the first is removed, the second can go in.
    first.remove_for_qt_window(w.ptr()).unwrap();
    second.init_for_qt_window(w.ptr()).unwrap();
    assert_eq!(w.tree(), "Second\n");
}

fn remove_and_reinit() {
    init();
    let w = Win::shown(1);
    let item = MenuItem::new("Item", true, None);
    let menu = Menu::with_items(&[&item]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    menu.remove_for_qt_window(w.ptr()).unwrap();
    assert_eq!(w.tree(), "");
    assert!(!menu.is_visible_on_qt_window(w.ptr()));
    assert!(matches!(
        menu.remove_for_qt_window(w.ptr()),
        Err(Error::NotInitialized)
    ));
    // Changes while removed don't bring the bar back.
    item.set_text("Changed");
    settle();
    assert_eq!(w.tree(), "");
    // Re-attach, before and after the old bar's deferred deletion.
    menu.init_for_qt_window(w.ptr()).unwrap();
    assert_eq!(w.tree(), "Changed\n");
    menu.remove_for_qt_window(w.ptr()).unwrap();
    settle();
    menu.init_for_qt_window(w.ptr()).unwrap();
    assert_eq!(w.tree(), "Changed\n");
    assert!(menu.is_visible_on_qt_window(w.ptr()));
}

fn not_attached() {
    init();
    let w = Win::shown(1);
    let menu = Menu::new();
    assert!(matches!(
        menu.remove_for_qt_window(w.ptr()),
        Err(Error::NotInitialized)
    ));
    assert!(matches!(
        menu.hide_for_qt_window(w.ptr()),
        Err(Error::NotInitialized)
    ));
    assert!(matches!(
        menu.show_for_qt_window(w.ptr()),
        Err(Error::NotInitialized)
    ));
    assert!(!menu.is_visible_on_qt_window(w.ptr()));
    // Another menu's bar on the window isn't this menu's.
    let other = Menu::new();
    other.init_for_qt_window(w.ptr()).unwrap();
    assert!(!menu.is_visible_on_qt_window(w.ptr()));
    assert!(matches!(
        menu.hide_for_qt_window(w.ptr()),
        Err(Error::NotInitialized)
    ));
    assert!(other.is_visible_on_qt_window(w.ptr()));
}

fn hide_show() {
    init();
    let w = Win::shown(1);
    let menu = Menu::with_items(&[&MenuItem::new("Item", true, None)]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    assert!(menu.is_visible_on_qt_window(w.ptr()));
    menu.hide_for_qt_window(w.ptr()).unwrap();
    assert!(!menu.is_visible_on_qt_window(w.ptr()));
    // Hidden twice is fine; the items stay.
    menu.hide_for_qt_window(w.ptr()).unwrap();
    assert_eq!(w.tree(), "Item\n");
    menu.show_for_qt_window(w.ptr()).unwrap();
    assert!(menu.is_visible_on_qt_window(w.ptr()));
    // A rebuild keeps the visibility.
    menu.hide_for_qt_window(w.ptr()).unwrap();
    menu.append(&MenuItem::new("More", true, None)).unwrap();
    w.wait_tree("appended", |t| t == "Item\nMore\n");
    assert!(!menu.is_visible_on_qt_window(w.ptr()));
}

fn visible_on_hidden_window() {
    init();
    let w = Win::new(1);
    let menu = Menu::with_items(&[&MenuItem::new("Item", true, None)]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    // Like GTK's `is_visible` (the bar's own flag), not whether it's on screen right now.
    assert!(menu.is_visible_on_qt_window(w.ptr()));
}

fn drop_menu() {
    init();
    let (w1, w2) = (Win::new(1), Win::new(2));
    let item = MenuItem::new("Item", true, None);
    let menu = Menu::with_items(&[&item]).unwrap();
    menu.init_for_qt_window(w1.ptr()).unwrap();
    menu.init_for_qt_window(w2.ptr()).unwrap();
    drop(menu);
    assert_eq!(w1.tree(), "");
    assert_eq!(w2.tree(), "");
    // The items outlive the menu; changing them rebuilds nothing.
    item.set_text("Changed");
    settle();
    assert_eq!(w1.tree(), "");
    // The windows are free for another menu.
    let other = Menu::with_items(&[&item]).unwrap();
    other.init_for_qt_window(w1.ptr()).unwrap();
    assert_eq!(w1.tree(), "Changed\n");
}

fn window_destroyed() {
    init();
    let item = MenuItem::new("Item", true, None);
    let check = CheckMenuItem::new("Check", true, false, None);
    let sub = Submenu::with_items("Sub", true, &[&check]).unwrap();
    let menu = Menu::with_items(&[&item, &sub]).unwrap();
    let keep = Win::new(2);
    menu.init_for_qt_window(keep.ptr()).unwrap();
    let ptr = {
        let w = Win::new(1);
        menu.init_for_qt_window(w.ptr()).unwrap();
        w.ptr() as usize
    };
    // Rebuilds after the window is gone skip it (and still update the live one).
    item.set_text("After");
    check.set_checked(true);
    sub.append(&MenuItem::new("New", true, None)).unwrap();
    menu.append(&MenuItem::new("Top", true, None)).unwrap();
    keep.wait_tree("the live window updated", |t| {
        t == "After\nSub\n  [x] Check\n  New\nTop\n"
    });
    // The old pointer no longer names a widget.
    let old = ptr as *mut c_void;
    assert!(!menu.is_visible_on_qt_window(old));
    assert!(matches!(
        menu.remove_for_qt_window(old),
        Err(Error::NotInitialized)
    ));
    assert!(matches!(
        menu.hide_for_qt_window(old),
        Err(Error::NotInitialized)
    ));
    assert!(!menu.show_context_menu_for_qt_window(old, None));
    // A new window: the menu can be attached (the dead one was forgotten).
    let w3 = Win::new(3);
    menu.init_for_qt_window(w3.ptr()).unwrap();
    assert_eq!(w3.tree(), "After\nSub\n  [x] Check\n  New\nTop\n");
    drop(menu);
    assert_eq!(keep.tree(), "");
}

fn after_shutdown() {
    init();
    let w = Win::new(1);
    let item = MenuItem::new("Item", true, None);
    let check = CheckMenuItem::new("Check", true, false, None);
    let sub = Submenu::with_items("Sub", true, &[&check]).unwrap();
    let menu = Menu::with_items(&[&item, &sub]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    let ptr = w.ptr();
    drop(w);
    app::shutdown();
    // Qt is gone: changes rebuild nothing, calls fail, nothing crashes.
    item.set_text("Changed");
    check.set_checked(true);
    sub.append(&MenuItem::new("New", true, None)).unwrap();
    menu.append(&MenuItem::new("Top", true, None)).unwrap();
    menu.remove(&item).unwrap();
    assert!(matches!(
        menu.init_for_qt_window(ptr),
        Err(Error::NotInitialized)
    ));
    assert!(matches!(
        menu.hide_for_qt_window(ptr),
        Err(Error::NotInitialized)
    ));
    assert!(!menu.is_visible_on_qt_window(ptr));
    assert!(!menu.show_context_menu_for_qt_window(ptr, None));
    assert!(!sub.show_context_menu_for_qt_window(ptr, None));
    drop(menu);
}

fn attached_at_shutdown() {
    init();
    let w = Win::shown(1);
    let menu = Menu::with_items(&[&MenuItem::new("Item", true, None)]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    // The window outlives the application too.
    app::shutdown();
    drop(menu);
    drop(w);
}

fn pending_at_shutdown() {
    init();
    let w = Win::new(1);
    let item = MenuItem::new("Item", true, None);
    let menu = Menu::with_items(&[&item]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    // Schedules a rebuild that never runs.
    item.set_text("Changed");
    drop(w);
    app::shutdown();
    item.set_text("Again");
    drop(menu);
}

// -------------------------------------------------------------------------------------------
// invalid pointers and other threads

fn invalid_cases() -> Vec<Case> {
    vec![
        Case::pass("invalid: null pointer", invalid_null),
        Case::pass("invalid: dangling pointer", invalid_dangling),
        Case::pass("invalid: pointer to something else", invalid_not_widget),
        Case::pass("invalid: menu used before app::init", invalid_before_init),
        Case::pass(
            "invalid: calls from another thread fail",
            invalid_other_thread,
        ),
    ]
}

/// Every pointer-taking call fails cleanly for `ptr`.
fn assert_rejected(ptr: *mut c_void) {
    let menu = Menu::with_items(&[&MenuItem::new("Item", true, None)]).unwrap();
    let sub = Submenu::with_items("Sub", true, &[&MenuItem::new("x", true, None)]).unwrap();
    assert!(matches!(
        menu.init_for_qt_window(ptr),
        Err(Error::NotInitialized)
    ));
    assert!(matches!(
        menu.remove_for_qt_window(ptr),
        Err(Error::NotInitialized)
    ));
    assert!(matches!(
        menu.hide_for_qt_window(ptr),
        Err(Error::NotInitialized)
    ));
    assert!(matches!(
        menu.show_for_qt_window(ptr),
        Err(Error::NotInitialized)
    ));
    assert!(!menu.is_visible_on_qt_window(ptr));
    assert!(!menu.show_context_menu_for_qt_window(ptr, None));
    assert!(!sub.show_context_menu_for_qt_window(ptr, None));
}

fn invalid_null() {
    init();
    assert_rejected(std::ptr::null_mut());
}

fn invalid_dangling() {
    init();
    let ptr = {
        let w = Win::new(1);
        w.ptr()
    };
    assert_rejected(ptr);
    assert_rejected(std::ptr::dangling_mut::<u64>().cast());
    assert_rejected(usize::MAX as *mut c_void);
}

fn invalid_not_widget() {
    init();
    assert_rejected(not_a_widget());
}

fn invalid_before_init() {
    // No application: everything fails, nothing panics, the pointer isn't touched.
    assert_rejected(not_a_widget());
    assert_rejected(std::ptr::null_mut());
}

fn invalid_other_thread() {
    init();
    let w = Win::shown(1);
    let shared = Menu::with_items(&[&MenuItem::new("Main", true, None)]).unwrap();
    shared.init_for_qt_window(w.ptr()).unwrap();
    let ptr = w.ptr() as usize;
    // Menus are `!Send`, so another thread can only use its own menus with a pointer it got from
    // the main thread. Qt isn't usable there: the calls fail without touching it.
    thread::spawn(move || {
        let ptr = ptr as *mut c_void;
        assert_rejected(ptr);
        let item = MenuItem::new("Item", true, None);
        let menu = Menu::with_items(&[&item]).unwrap();
        item.set_text("Changed");
        menu.append(&CheckMenuItem::new("C", true, false, None))
            .unwrap();
        drop(menu);
    })
    .join()
    .expect("no panic on the other thread");
    assert_eq!(w.tree(), "Main\n");
    assert!(shared.is_visible_on_qt_window(w.ptr()));
}

// -------------------------------------------------------------------------------------------
// context menus

fn context_cases() -> Vec<Case> {
    vec![
        Case::pass("context: Menu shows a popup", context_menu),
        Case::pass("context: Submenu shows its items", context_submenu),
        Case::pass("context: positions", context_positions),
        Case::pass("context: dead window returns false", context_dead_window),
        Case::pass(
            "context: menubar unaffected by a context menu of the same menu",
            context_with_menubar,
        ),
    ]
}

fn context_menu() {
    init();
    let w = Win::shown(1);
    debug::take_popup_menu(); // start recording
    let menu = Menu::with_items(&[
        &MenuItem::new("Open", true, ctrl(Code::KeyO)),
        &CheckMenuItem::new("Check", true, true, ctrl(Code::KeyK)),
        &IconMenuItem::new("Icon", false, Some(icon()), None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::copy(None),
        &PredefinedMenuItem::minimize(None),
        &PredefinedMenuItem::about(None, Some(AboutMetadata::default())),
        &PredefinedMenuItem::services(None),
        &Submenu::with_items("More", true, &[&MenuItem::new("Inner", true, None)]).unwrap(),
    ])
    .unwrap();
    assert!(menu.show_context_menu_for_qt_window(w.ptr(), None));
    let popup = Rc::new(RefCell::new(String::new()));
    {
        let popup = popup.clone();
        wait_for("the popup", move || {
            let p = debug::take_popup_menu();
            if !p.is_empty() {
                *popup.borrow_mut() = p;
            }
            !popup.borrow().is_empty()
        });
    }
    // Context menus only display shortcuts.
    assert_eq!(
        *popup.borrow(),
        "Open (Ctrl+O)\n[x] Check (Ctrl+K)\nIcon {disabled}\n---\nCopy (Ctrl+C)\nMinimize \
         (Ctrl+M)\nAbout\nServices {disabled}\nMore\n  Inner\n"
    );
    // A context menu doesn't install a menubar.
    assert_eq!(w.tree(), "");
}

/// Show `f`'s context menu and return its tree.
fn popup_of(f: impl FnOnce() -> bool) -> String {
    debug::take_popup_menu();
    assert!(f());
    let popup = Rc::new(RefCell::new(String::new()));
    let out = popup.clone();
    wait_for("the popup", move || {
        let p = debug::take_popup_menu();
        if !p.is_empty() {
            *popup.borrow_mut() = p;
        }
        !popup.borrow().is_empty()
    });
    let s = out.borrow().clone();
    s
}

fn context_submenu() {
    init();
    let w = Win::shown(1);
    let sub = Submenu::with_items(
        "Sub",
        true,
        &[
            &MenuItem::new("A", true, None),
            &Submenu::with_items("Nested", true, &[&MenuItem::new("B", true, None)]).unwrap(),
        ],
    )
    .unwrap();
    let ptr = w.ptr();
    let tree = popup_of(|| sub.show_context_menu_for_qt_window(ptr, None));
    assert_eq!(tree, "A\nNested\n  B\n");
}

fn context_positions() {
    init();
    let w = Win::shown(1);
    let menu = Menu::with_items(&[&MenuItem::new("Item", true, None)]).unwrap();
    let ptr = w.ptr();
    let logical: Position = LogicalPosition::new(10.0, 20.0).into();
    let physical: Position = PhysicalPosition::new(30, 40).into();
    assert_eq!(
        popup_of(|| menu.show_context_menu_for_qt_window(ptr, Some(logical))),
        "Item\n"
    );
    assert_eq!(
        popup_of(|| menu.show_context_menu_for_qt_window(ptr, Some(physical))),
        "Item\n"
    );
    // An empty menu can be shown too (nothing to record).
    assert!(Menu::new().show_context_menu_for_qt_window(ptr, None));
    settle();
    debug::take_popup_menu();
}

fn context_dead_window() {
    init();
    let menu = Menu::with_items(&[&MenuItem::new("Item", true, None)]).unwrap();
    let sub = Submenu::with_items("Sub", true, &[&MenuItem::new("x", true, None)]).unwrap();
    let ptr = {
        let w = Win::shown(1);
        w.ptr()
    };
    settle();
    assert!(!menu.show_context_menu_for_qt_window(ptr, None));
    assert!(!sub.show_context_menu_for_qt_window(ptr, None));
}

fn context_with_menubar() {
    init();
    let w = Win::shown(1);
    let check = CheckMenuItem::new("Check", true, false, ctrl(Code::KeyK));
    let menu = Menu::with_items(&[&check]).unwrap();
    menu.init_for_qt_window(w.ptr()).unwrap();
    let ptr = w.ptr();
    assert_eq!(
        popup_of(|| menu.show_context_menu_for_qt_window(ptr, None)),
        "[ ] Check (Ctrl+K)\n"
    );
    // The menubar still registers the shortcut.
    assert_eq!(w.tree(), "[ ] Check <Ctrl+K>\n");
}
