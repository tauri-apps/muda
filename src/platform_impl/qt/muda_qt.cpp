// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

// Qt Widgets glue for muda's `qt` backend. Builds QMenuBar / QMenu / QAction trees on request
// from Rust; everything runs on the thread that owns the QApplication.

#include <QAction>
#include <QApplication>
#include <QCursor>
#include <QIcon>
#include <QImage>
#include <QKeyEvent>
#include <QKeySequence>
#include <QLayout>
#include <QMainWindow>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QPixmap>
#include <QPointer>
#include <QVBoxLayout>
#include <QWidget>
#include <QWindow>

#include <cstdint>

extern "C" {
typedef void (*mq_fn)(void *ud);
typedef void (*mq_drop_fn)(void *ud);
}

namespace {

const char kMenuBarName[] = "muda-menubar";

QIcon iconFrom(const uint8_t *rgba, int w, int h) {
  if (!rgba || w <= 0 || h <= 0)
    return QIcon();
  QImage img(rgba, w, h, w * 4, QImage::Format_RGBA8888);
  return QIcon(QPixmap::fromImage(img.copy()));
}

QMenuBar *findMenuBar(QWidget *w) {
  if (auto *mw = qobject_cast<QMainWindow *>(w))
    if (mw->menuWidget() && mw->menuWidget()->objectName() == QLatin1String(kMenuBarName))
      return qobject_cast<QMenuBar *>(mw->menuWidget());
  return w->findChild<QMenuBar *>(QLatin1String(kMenuBarName), Qt::FindDirectChildrenOnly);
}

// The window a predefined action applies to: the active one, or the one owning the menu.
QWidget *targetWindow() {
  if (QWidget *w = QApplication::activeWindow())
    return w->window();
  if (QWidget *f = QApplication::focusWidget())
    return f->window();
  return nullptr;
}

void sendKey(int key, Qt::KeyboardModifiers mods) {
  QWidget *target = QApplication::focusWidget();
  if (!target)
    return;
  QKeyEvent press(QEvent::KeyPress, key, mods);
  QKeyEvent release(QEvent::KeyRelease, key, mods);
  QApplication::sendEvent(target, &press);
  QApplication::sendEvent(target, &release);
}

} // namespace

extern "C" {

int mq_has_app(void) { return QCoreApplication::instance() ? 1 : 0; }

// Run `f(ud)` on the QApplication thread from the event loop (or immediately without an app).
void mq_post(mq_fn f, void *ud) {
  if (!QCoreApplication::instance()) {
    f(ud);
    return;
  }
  QMetaObject::invokeMethod(QCoreApplication::instance(), [f, ud] { f(ud); },
                            Qt::QueuedConnection);
}

// --- widget tracking ----------------------------------------------------------------------------

// A heap QPointer, so Rust can tell when a window it attached a menu to has been destroyed.
void *mq_track(void *widget) { return new QPointer<QWidget>(static_cast<QWidget *>(widget)); }
void *mq_tracked(void *tracker) { return static_cast<QPointer<QWidget> *>(tracker)->data(); }
void mq_untrack(void *tracker) { delete static_cast<QPointer<QWidget> *>(tracker); }

// --- menubars --------------------------------------------------------------------------------

// Returns the muda menubar of `widget`, creating it if `create` is set. The menubar goes into
// QMainWindow::setMenuBar, or the top of the widget's layout (QLayout::setMenuBar).
// Returns NULL if it doesn't exist (and wasn't created) or the widget can't host one.
void *mq_menubar_for(void *widget, int create) {
  auto *w = static_cast<QWidget *>(widget);
  if (QMenuBar *bar = findMenuBar(w))
    return bar;
  if (!create)
    return nullptr;
  auto *bar = new QMenuBar(w);
  bar->setObjectName(QLatin1String(kMenuBarName));
  if (auto *mw = qobject_cast<QMainWindow *>(w)) {
    mw->setMenuBar(bar);
  } else {
    QLayout *layout = w->layout();
    if (!layout)
      layout = new QVBoxLayout(w);
    layout->setMenuBar(bar);
  }
  bar->show();
  return bar;
}

void mq_menubar_remove(void *widget) {
  auto *w = static_cast<QWidget *>(widget);
  if (QMenuBar *bar = findMenuBar(w)) {
    if (QLayout *layout = w->layout(); layout && layout->menuBar() == bar)
      layout->setMenuBar(nullptr);
    bar->hide();
    bar->deleteLater();
  }
}

void mq_menubar_set_visible(void *widget, int visible) {
  if (QMenuBar *bar = findMenuBar(static_cast<QWidget *>(widget)))
    bar->setVisible(visible != 0);
}

int mq_menubar_is_visible(void *widget) {
  QMenuBar *bar = findMenuBar(static_cast<QWidget *>(widget));
  return bar && bar->isVisible() ? 1 : 0;
}

// Remove (and schedule deletion of) everything in a menubar or menu.
void mq_container_clear(void *container) {
  auto *c = static_cast<QWidget *>(container);
  const auto actions = c->actions();
  for (QAction *a : actions) {
    c->removeAction(a);
    if (QMenu *m = a->menu(); m && m->parent() == c)
      m->deleteLater();
    if (a->parent() == c)
      a->deleteLater();
  }
}

// --- building ----------------------------------------------------------------------------------

// Add a submenu to a QMenuBar or QMenu. Returns the new QMenu*.
void *mq_container_add_menu(void *container, const char *text, int enabled, const uint8_t *rgba,
                            int w, int h) {
  auto *c = static_cast<QWidget *>(container);
  auto *menu = new QMenu(QString::fromUtf8(text), c);
  menu->setEnabled(enabled != 0);
  const QIcon icon = iconFrom(rgba, w, h);
  if (!icon.isNull())
    menu->setIcon(icon);
  if (auto *bar = qobject_cast<QMenuBar *>(c))
    bar->addMenu(menu);
  else if (auto *parent = qobject_cast<QMenu *>(c))
    parent->addMenu(menu);
  return menu;
}

// Add an action. `shortcut` is a portable QKeySequence string ("Ctrl+Shift+N") that is
// registered; `shortcut_hint` is only displayed (for keys the focused widget handles itself).
// `on_click(ud)` runs when triggered; `drop(ud)` when the action is destroyed.
void mq_container_add_action(void *container, const char *text, int enabled, int checkable,
                             int checked, const char *shortcut, const char *shortcut_hint,
                             const uint8_t *rgba, int w, int h, mq_fn on_click, void *ud,
                             mq_drop_fn drop) {
  auto *c = static_cast<QWidget *>(container);
  QString label = QString::fromUtf8(text);
  if (shortcut_hint && *shortcut_hint)
    label += QLatin1Char('\t') + QString::fromUtf8(shortcut_hint);
  auto *action = new QAction(label, c);
  action->setEnabled(enabled != 0);
  action->setCheckable(checkable != 0);
  if (checkable)
    action->setChecked(checked != 0);
  // Qt activates shortcuts of actions in a window's menubar (and its submenus) while the
  // window is active, even with the menus closed.
  if (shortcut && *shortcut)
    action->setShortcut(QKeySequence(QString::fromUtf8(shortcut), QKeySequence::PortableText));
  const QIcon icon = iconFrom(rgba, w, h);
  if (!icon.isNull())
    action->setIcon(icon);
  if (on_click)
    QObject::connect(action, &QAction::triggered, action, [on_click, ud] { on_click(ud); });
  if (drop)
    QObject::connect(action, &QObject::destroyed, [drop, ud] { drop(ud); });
  c->addAction(action);
}

void mq_container_add_separator(void *container) {
  auto *c = static_cast<QWidget *>(container);
  if (auto *menu = qobject_cast<QMenu *>(c))
    menu->addSeparator();
  else if (auto *bar = qobject_cast<QMenuBar *>(c))
    bar->addSeparator();
}

// --- context menus -----------------------------------------------------------------------------

// A popup menu parented to `widget` (needed for correct placement on Wayland).
void *mq_popup_menu_new(void *widget) {
  auto *menu = new QMenu(static_cast<QWidget *>(widget));
  menu->setAttribute(Qt::WA_DeleteOnClose);
  return menu;
}

// Show `menu` at (x, y) relative to the window's top-left corner, or at the cursor.
// `physical` means the coordinates are physical pixels.
void mq_popup_menu_show(void *menu, void *widget, int has_pos, double x, double y, int physical) {
  auto *m = static_cast<QMenu *>(menu);
  auto *w = static_cast<QWidget *>(widget);
  QPoint at = QCursor::pos();
  if (has_pos) {
    const double scale = physical ? w->devicePixelRatioF() : 1.0;
    at = w->window()->mapToGlobal(QPoint(qRound(x / scale), qRound(y / scale)));
  }
  m->popup(at);
}

// --- predefined items --------------------------------------------------------------------------

enum {
  MQ_COPY = 1,
  MQ_CUT,
  MQ_PASTE,
  MQ_SELECT_ALL,
  MQ_UNDO,
  MQ_REDO,
  MQ_DELETE,
  MQ_MINIMIZE,
  MQ_MAXIMIZE,
  MQ_FULLSCREEN,
  MQ_HIDE,
  MQ_CLOSE_WINDOW,
  MQ_QUIT,
};

void mq_run_predefined(int kind) {
  switch (kind) {
  case MQ_COPY: sendKey(Qt::Key_C, Qt::ControlModifier); break;
  case MQ_CUT: sendKey(Qt::Key_X, Qt::ControlModifier); break;
  case MQ_PASTE: sendKey(Qt::Key_V, Qt::ControlModifier); break;
  case MQ_SELECT_ALL: sendKey(Qt::Key_A, Qt::ControlModifier); break;
  case MQ_UNDO: sendKey(Qt::Key_Z, Qt::ControlModifier); break;
  case MQ_REDO: sendKey(Qt::Key_Z, Qt::ControlModifier | Qt::ShiftModifier); break;
  case MQ_DELETE: sendKey(Qt::Key_Delete, Qt::NoModifier); break;
  case MQ_QUIT: QCoreApplication::quit(); break;
  default: {
    QWidget *w = targetWindow();
    if (!w)
      return;
    switch (kind) {
    case MQ_MINIMIZE: w->showMinimized(); break;
    case MQ_MAXIMIZE: w->isMaximized() ? w->showNormal() : w->showMaximized(); break;
    case MQ_FULLSCREEN: w->isFullScreen() ? w->showNormal() : w->showFullScreen(); break;
    case MQ_HIDE: w->hide(); break;
    case MQ_CLOSE_WINDOW: w->close(); break; // goes through the window's close handling
    default: break;
    }
  }
  }
}

void mq_show_about(const char *title, const char *html, const uint8_t *rgba, int w, int h) {
  auto *box = new QMessageBox(targetWindow());
  box->setAttribute(Qt::WA_DeleteOnClose);
  box->setWindowTitle(QString::fromUtf8(title));
  box->setTextFormat(Qt::RichText);
  box->setText(QString::fromUtf8(html));
  box->setStandardButtons(QMessageBox::Ok);
  const QIcon icon = iconFrom(rgba, w, h);
  if (!icon.isNull())
    box->setIconPixmap(icon.pixmap(64, 64));
  else
    box->setIcon(QMessageBox::Information);
  box->open();
}

} // extern "C"
