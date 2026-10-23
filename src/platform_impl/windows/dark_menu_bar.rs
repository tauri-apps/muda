// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

// this is a port of combination of https://github.com/hrydgard/ppsspp/blob/master/Windows/W32Util/UAHMenuBar.cpp and https://github.com/ysc3839/win32-darkmode/blob/master/win32-darkmode/DarkMode.h

#![allow(non_snake_case, clippy::upper_case_acronyms)]

use std::cell::Cell;

use std::sync::LazyLock;
use windows_sys::{
    s,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::*,
        UI::{Accessibility::*, Controls::*, WindowsAndMessaging::*},
    },
};

pub const WM_UAHDRAWMENU: u32 = 0x0091;
pub const WM_UAHDRAWMENUITEM: u32 = 0x0092;

#[repr(C)]
struct UAHMENUITEMMETRICS0 {
    cx: u32,
    cy: u32,
}

#[repr(C)]
struct UAHMENUITEMMETRICS {
    rgsizeBar: [UAHMENUITEMMETRICS0; 2],
    rgsizePopup: [UAHMENUITEMMETRICS0; 4],
}

#[repr(C)]
struct UAHMENUPOPUPMETRICS {
    rgcx: [u32; 4],
    fUpdateMaxWidths: u32,
}

#[repr(C)]
struct UAHMENU {
    hmenu: HMENU,
    hdc: HDC,
    dwFlags: u32,
}
#[repr(C)]
struct UAHMENUITEM {
    iPosition: u32,
    umim: UAHMENUITEMMETRICS,
    umpm: UAHMENUPOPUPMETRICS,
}
#[repr(C)]
struct UAHDRAWMENUITEM {
    dis: DRAWITEMSTRUCT,
    um: UAHMENU,
    umi: UAHMENUITEM,
}

#[derive(Debug)]
struct Win32Brush(Cell<HBRUSH>);

impl Win32Brush {
    const fn null() -> Win32Brush {
        Self(Cell::new(0 as _))
    }

    fn get_or_set(&self, color: u32) -> HBRUSH {
        if self.0.get().is_null() {
            self.0.set(unsafe { CreateSolidBrush(color) });
        }
        self.0.get()
    }
}

impl Drop for Win32Brush {
    fn drop(&mut self) {
        unsafe { DeleteObject(self.0.get()) };
    }
}

fn background_brush() -> HBRUSH {
    thread_local! {
        static BACKGROUND_BRUSH: Win32Brush = const { Win32Brush::null() };
    }
    const BACKGROUND_COLOR: u32 = 2829099;
    BACKGROUND_BRUSH.with(|brush| brush.get_or_set(BACKGROUND_COLOR))
}

fn selected_background_brush() -> HBRUSH {
    thread_local! {
        static SELECTED_BACKGROUND_BRUSH: Win32Brush = const { Win32Brush::null() };
    }
    const SELECTED_BACKGROUND_COLOR: u32 = 4276545;
    SELECTED_BACKGROUND_BRUSH.with(|brush| brush.get_or_set(SELECTED_BACKGROUND_COLOR))
}

/// Draws a dark menu bar if needed and returns whether it draws it or not
pub fn draw(hwnd: super::Hwnd, msg: u32, _wparam: WPARAM, lparam: LPARAM) {
    match msg {
        // draw over the annoying white line blow menubar
        // ref: https://github.com/notepad-plus-plus/notepad-plus-plus/pull/9985
        WM_NCACTIVATE | WM_NCPAINT => {
            let mut mbi = MENUBARINFO {
                cbSize: std::mem::size_of::<MENUBARINFO>() as _,
                ..unsafe { std::mem::zeroed() }
            };
            unsafe { GetMenuBarInfo(hwnd as _, OBJID_MENU, 0, &mut mbi) };

            let mut client_rc: RECT = unsafe { std::mem::zeroed() };
            unsafe {
                GetClientRect(hwnd as _, &mut client_rc);
                MapWindowPoints(
                    hwnd as _,
                    std::ptr::null_mut(),
                    &mut client_rc as *mut _ as *mut _,
                    2,
                );
            };

            let mut window_rc: RECT = unsafe { std::mem::zeroed() };
            unsafe { GetWindowRect(hwnd as _, &mut window_rc) };

            unsafe { OffsetRect(&mut client_rc, -window_rc.left, -window_rc.top) };

            let mut annoying_rc = client_rc;
            annoying_rc.bottom = annoying_rc.top;
            annoying_rc.top -= 1;

            unsafe {
                let hdc = GetWindowDC(hwnd as _);
                FillRect(hdc, &annoying_rc, background_brush());
                ReleaseDC(hwnd as _, hdc);
            }
        }

        // draw menu bar background
        WM_UAHDRAWMENU => {
            let pudm = lparam as *const UAHMENU;

            // get the menubar rect
            let rc = {
                let mut mbi = MENUBARINFO {
                    cbSize: std::mem::size_of::<MENUBARINFO>() as _,
                    ..unsafe { std::mem::zeroed() }
                };
                unsafe { GetMenuBarInfo(hwnd as _, OBJID_MENU, 0, &mut mbi) };

                let mut window_rc: RECT = unsafe { std::mem::zeroed() };
                unsafe { GetWindowRect(hwnd as _, &mut window_rc) };

                let mut rc = mbi.rcBar;
                // the rcBar is offset by the window rect
                unsafe { OffsetRect(&mut rc, -window_rc.left, -window_rc.top) };
                rc.top -= 1;
                rc
            };

            unsafe { FillRect((*pudm).hdc, &rc, background_brush()) };
        }

        // draw menu bar items
        WM_UAHDRAWMENUITEM => {
            let pudmi = lparam as *mut UAHDRAWMENUITEM;

            // get the menu item string
            let (label, cch, bitmap) = {
                let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
                info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
                info.fMask = MIIM_STRING | MIIM_BITMAP;
                let ok = unsafe {
                    GetMenuItemInfoW(
                        (*pudmi).um.hmenu,
                        (*pudmi).umi.iPosition,
                        true.into(),
                        &mut info,
                    )
                };
                if ok == 0 {
                    return;
                }

                info.cch += 1; // add 1 for null terminator
                let mut label = vec![0u16; info.cch as usize];
                info.dwTypeData = label.as_mut_ptr();

                let ok = unsafe {
                    GetMenuItemInfoW(
                        (*pudmi).um.hmenu,
                        (*pudmi).umi.iPosition,
                        true.into(),
                        &mut info,
                    )
                };
                if ok == 0 {
                    return;
                }

                (label, info.cch, info.hbmpItem)
            };

            // get the item state for drawing
            let mut dw_flags = DT_CENTER | DT_SINGLELINE | DT_VCENTER;
            let mut i_text_state_id = 0;
            let mut i_background_state_id = 0;

            unsafe {
                if (((*pudmi).dis.itemState & ODS_INACTIVE)
                    | ((*pudmi).dis.itemState & ODS_DEFAULT))
                    != 0
                {
                    // normal display
                    i_text_state_id = MPI_NORMAL;
                    i_background_state_id = MPI_NORMAL;
                }
                if (*pudmi).dis.itemState & ODS_HOTLIGHT != 0 {
                    // hot tracking
                    i_text_state_id = MPI_HOT;
                    i_background_state_id = MPI_HOT;
                }
                if (*pudmi).dis.itemState & ODS_SELECTED != 0 {
                    // clicked -- MENU_POPUPITEM has no state for this, though MENU_BARITEM does
                    i_text_state_id = MPI_HOT;
                    i_background_state_id = MPI_HOT;
                }
                if ((*pudmi).dis.itemState & ODS_GRAYED) != 0
                    || ((*pudmi).dis.itemState & ODS_DISABLED) != 0
                {
                    // disabled / grey text
                    i_text_state_id = MPI_DISABLED;
                    i_background_state_id = MPI_DISABLED;
                }
                if ((*pudmi).dis.itemState & ODS_NOACCEL) != 0 {
                    dw_flags |= DT_HIDEPREFIX;
                }

                let bg_brush = match i_background_state_id {
                    MPI_HOT => selected_background_brush(),
                    _ => background_brush(),
                };

                FillRect((*pudmi).um.hdc, &(*pudmi).dis.rcItem, bg_brush);

                const TEXT_COLOR: u32 = 16777215;
                const DISABLED_TEXT_COLOR: u32 = 7171437;

                let text_brush = match i_text_state_id {
                    MPI_DISABLED => DISABLED_TEXT_COLOR,
                    _ => TEXT_COLOR,
                };

                SetBkMode((*pudmi).um.hdc, 0);
                SetTextColor((*pudmi).um.hdc, text_brush);
                let mut text_rect = (*pudmi).dis.rcItem;

                // The custom background covers the native icon as well as the text.
                // Draw the item's bitmap again, keeping its alpha channel intact.
                let mut bm: BITMAP = std::mem::zeroed();
                if !bitmap.is_null()
                    && GetObjectW(
                        bitmap,
                        std::mem::size_of::<BITMAP>() as _,
                        &mut bm as *mut _ as _,
                    ) != 0
                {
                    let mut label_rect: RECT = std::mem::zeroed();
                    DrawTextW(
                        (*pudmi).um.hdc,
                        label.as_ptr(),
                        cch as _,
                        &mut label_rect,
                        dw_flags | DT_CALCRECT,
                    );
                    let gap = if cch == 0 { 0 } else { 4 };
                    let width = bm.bmWidth + gap + label_rect.right - label_rect.left;
                    let x = text_rect.left + (text_rect.right - text_rect.left - width) / 2;
                    let y = text_rect.top + (text_rect.bottom - text_rect.top - bm.bmHeight) / 2;
                    let source_dc = CreateCompatibleDC((*pudmi).um.hdc);
                    if !source_dc.is_null() {
                        let old_bitmap = SelectObject(source_dc, bitmap);
                        AlphaBlend(
                            (*pudmi).um.hdc,
                            x,
                            y,
                            bm.bmWidth,
                            bm.bmHeight,
                            source_dc,
                            0,
                            0,
                            bm.bmWidth,
                            bm.bmHeight,
                            BLENDFUNCTION {
                                BlendOp: AC_SRC_OVER as _,
                                BlendFlags: 0,
                                SourceConstantAlpha: if i_text_state_id == MPI_DISABLED {
                                    128
                                } else {
                                    255
                                },
                                AlphaFormat: AC_SRC_ALPHA as _,
                            },
                        );
                        SelectObject(source_dc, old_bitmap);
                        DeleteDC(source_dc);
                    }
                    text_rect.left = x + bm.bmWidth + gap;
                    text_rect.right = text_rect.left + label_rect.right - label_rect.left;
                }
                DrawTextW(
                    (*pudmi).um.hdc,
                    label.as_ptr(),
                    cch as _,
                    &mut text_rect,
                    dw_flags,
                );
            }
        }

        _ => {}
    };
}

pub fn should_use_dark_mode(hwnd: super::Hwnd) -> bool {
    should_apps_use_dark_mode() && !is_high_contrast() && is_dark_mode_allowed_for_window(hwnd as _)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_menu_bar_draws_item_bitmap() {
        unsafe {
            // A red icon with a transparent border makes missing icons and opaque
            // copies distinguishable from correctly alpha-blended rendering.
            let mut rgba = vec![0; 16 * 16 * 4];
            for y in 4..12 {
                for x in 4..12 {
                    let offset = (y * 16 + x) * 4;
                    rgba[offset] = 255;
                    rgba[offset + 3] = 255;
                }
            }
            let icon = super::super::icon::WinIcon::from_rgba(rgba, 16, 16).unwrap();
            let bitmap = icon.to_hbitmap();
            let menu = CreateMenu();
            let mut label = [0u16];
            let info = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as _,
                fMask: MIIM_STRING | MIIM_BITMAP,
                dwTypeData: label.as_mut_ptr(),
                hbmpItem: bitmap,
                ..std::mem::zeroed()
            };
            assert_ne!(InsertMenuItemW(menu, 0, 1, &info), 0);

            let screen_dc = GetDC(std::ptr::null_mut());
            let dc = CreateCompatibleDC(screen_dc);
            let target = CreateCompatibleBitmap(screen_dc, 80, 24);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            let old_bitmap = SelectObject(dc, target);
            let mut item: UAHDRAWMENUITEM = std::mem::zeroed();
            item.um.hmenu = menu;
            item.um.hdc = dc;
            item.dis.rcItem = RECT {
                left: 0,
                top: 0,
                right: 80,
                bottom: 24,
            };
            draw(0, WM_UAHDRAWMENUITEM, 0, &mut item as *mut _ as _);
            let center = GetPixel(dc, 40, 12);
            let border = GetPixel(dc, 32, 4);
            let background = GetPixel(dc, 0, 0);

            SelectObject(dc, old_bitmap);
            DeleteObject(target);
            DeleteDC(dc);
            DestroyMenu(menu);
            DeleteObject(bitmap);

            assert_eq!(center, 0x000000ff, "the icon should remain visible");
            assert_eq!(border, background, "the icon border should be transparent");
        }
    }
}

static HUXTHEME: LazyLock<isize> =
    LazyLock::new(|| unsafe { LoadLibraryA(s!("uxtheme.dll")) as _ });

fn should_apps_use_dark_mode() -> bool {
    const UXTHEME_SHOULDAPPSUSEDARKMODE_ORDINAL: u16 = 132;
    type ShouldAppsUseDarkMode = unsafe extern "system" fn() -> bool;
    static SHOULD_APPS_USE_DARK_MODE: LazyLock<Option<ShouldAppsUseDarkMode>> =
        LazyLock::new(|| unsafe {
            if *HUXTHEME == 0 {
                return None;
            }

            GetProcAddress(
                (*HUXTHEME) as *mut _,
                UXTHEME_SHOULDAPPSUSEDARKMODE_ORDINAL as usize as *mut _,
            )
            .map(|handle| std::mem::transmute(handle))
        });

    SHOULD_APPS_USE_DARK_MODE
        .map(|should_apps_use_dark_mode| unsafe { (should_apps_use_dark_mode)() })
        .unwrap_or(false)
}

fn is_dark_mode_allowed_for_window(hwnd: HWND) -> bool {
    const UXTHEME_ISDARKMODEALLOWEDFORWINDOW_ORDINAL: u16 = 137;
    type IsDarkModeAllowedForWindow = unsafe extern "system" fn(HWND) -> bool;
    static IS_DARK_MODE_ALLOWED_FOR_WINDOW: LazyLock<Option<IsDarkModeAllowedForWindow>> =
        LazyLock::new(|| unsafe {
            if *HUXTHEME == 0 {
                return None;
            }

            GetProcAddress(
                (*HUXTHEME) as *mut _,
                UXTHEME_ISDARKMODEALLOWEDFORWINDOW_ORDINAL as usize as *mut _,
            )
            .map(|handle| std::mem::transmute(handle))
        });

    if let Some(_is_dark_mode_allowed_for_window) = *IS_DARK_MODE_ALLOWED_FOR_WINDOW {
        unsafe { _is_dark_mode_allowed_for_window(hwnd) }
    } else {
        false
    }
}

fn is_high_contrast() -> bool {
    const HCF_HIGHCONTRASTON: u32 = 1;

    let mut hc = HIGHCONTRASTA {
        cbSize: 0,
        dwFlags: Default::default(),
        lpszDefaultScheme: std::ptr::null_mut(),
    };

    let ok = unsafe {
        SystemParametersInfoA(
            SPI_GETHIGHCONTRAST,
            std::mem::size_of_val(&hc) as _,
            &mut hc as *mut _ as _,
            Default::default(),
        )
    };

    ok != 0 && (HCF_HIGHCONTRASTON & hc.dwFlags) != 0
}
