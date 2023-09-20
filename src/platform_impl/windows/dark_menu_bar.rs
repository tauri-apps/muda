// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

// this is a port of combination of https://github.com/hrydgard/ppsspp/blob/master/Windows/W32Util/UAHMenuBar.cpp and https://github.com/ysc3839/win32-darkmode/blob/master/win32-darkmode/DarkMode.h

#![allow(non_snake_case, clippy::upper_case_acronyms)]

use once_cell::sync::Lazy;
use windows_sys::{
    s, w,
    Win32::{
        Foundation::{HMODULE, HWND, LPARAM, RECT, WPARAM},
        Graphics::Gdi::{
            GetWindowDC, MapWindowPoints, OffsetRect, ReleaseDC, DT_CENTER, DT_HIDEPREFIX,
            DT_SINGLELINE, DT_VCENTER, HDC,
        },
        System::LibraryLoader::{GetProcAddress, LoadLibraryA},
        UI::{
            Accessibility::HIGHCONTRASTA,
            Controls::{
                CloseThemeData, DrawThemeBackground, DrawThemeText, OpenThemeData, DRAWITEMSTRUCT,
                MENU_POPUPITEM, MPI_DISABLED, MPI_HOT, MPI_NORMAL, ODS_DEFAULT, ODS_DISABLED,
                ODS_GRAYED, ODS_HOTLIGHT, ODS_INACTIVE, ODS_NOACCEL, ODS_SELECTED,
            },
            WindowsAndMessaging::{
                GetClientRect, GetMenuBarInfo, GetMenuItemInfoW, GetWindowRect,
                SystemParametersInfoA, HMENU, MENUBARINFO, MENUITEMINFOW, MIIM_STRING, OBJID_MENU,
                SPI_GETHIGHCONTRAST, WM_NCACTIVATE, WM_NCPAINT,
            },
        },
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

/// Draws a dark menu bar if needed and returns whether it draws it or not
pub fn draw(hwnd: HWND, msg: u32, _wparam: WPARAM, lparam: LPARAM) {
    match msg {
        // draw over the annoying white line blow menubar
        // ref: https://github.com/notepad-plus-plus/notepad-plus-plus/pull/9985
        WM_NCACTIVATE | WM_NCPAINT => {
            let mut mbi = MENUBARINFO {
                cbSize: std::mem::size_of::<MENUBARINFO>() as _,
                ..unsafe { std::mem::zeroed() }
            };
            unsafe { GetMenuBarInfo(hwnd, OBJID_MENU, 0, &mut mbi) };

            let mut client_rc: RECT = unsafe { std::mem::zeroed() };
            unsafe {
                GetClientRect(hwnd, &mut client_rc);
                MapWindowPoints(hwnd, 0, &mut client_rc as *mut _ as *mut _, 2);
            };

            let mut window_rc: RECT = unsafe { std::mem::zeroed() };
            unsafe { GetWindowRect(hwnd, &mut window_rc) };

            unsafe { OffsetRect(&mut client_rc, -window_rc.left, -window_rc.top) };

            let mut annoying_rc = client_rc;
            annoying_rc.bottom = annoying_rc.top;
            annoying_rc.top -= 1;

            unsafe {
                let theme = OpenThemeData(hwnd, w!("Menu"));
                let hdc = GetWindowDC(hwnd);
                DrawThemeBackground(
                    theme,
                    hdc,
                    MENU_POPUPITEM,
                    MPI_NORMAL,
                    &annoying_rc,
                    std::ptr::null(),
                );
                ReleaseDC(hwnd, hdc);
                CloseThemeData(theme);
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
                unsafe { GetMenuBarInfo(hwnd, OBJID_MENU, 0, &mut mbi) };

                let mut window_rc: RECT = unsafe { std::mem::zeroed() };
                unsafe { GetWindowRect(hwnd, &mut window_rc) };

                let mut rc = mbi.rcBar;
                // the rcBar is offset by the window rect
                unsafe { OffsetRect(&mut rc, -window_rc.left, -window_rc.top) };
                rc.top -= 1;
                rc
            };

            unsafe {
                let theme = OpenThemeData(hwnd, w!("Menu"));
                DrawThemeBackground(
                    theme,
                    (*pudm).hdc,
                    MENU_POPUPITEM,
                    MPI_NORMAL,
                    &rc,
                    std::ptr::null(),
                );
                CloseThemeData(theme);
            }
        }

        // draw menu bar items
        WM_UAHDRAWMENUITEM => {
            let pudmi = lparam as *const UAHDRAWMENUITEM;

            // get the menu item string
            let (label, cch) = {
                let mut label = Vec::<u16>::with_capacity(256);
                let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
                info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as _;
                info.fMask = MIIM_STRING;
                info.dwTypeData = label.as_mut_ptr();
                info.cch = (std::mem::size_of_val(&label) / 2 - 1) as _;
                unsafe {
                    GetMenuItemInfoW(
                        (*pudmi).um.hmenu,
                        (*pudmi).umi.iPosition,
                        true.into(),
                        &mut info,
                    )
                };
                (label, info.cch)
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

                let theme = OpenThemeData(hwnd, w!("Menu"));
                DrawThemeBackground(
                    theme,
                    (*pudmi).um.hdc,
                    MENU_POPUPITEM,
                    i_background_state_id,
                    &(*pudmi).dis.rcItem,
                    std::ptr::null(),
                );
                DrawThemeText(
                    theme,
                    (*pudmi).um.hdc,
                    MENU_POPUPITEM,
                    i_text_state_id,
                    label.as_ptr(),
                    cch as _,
                    dw_flags,
                    0,
                    &(*pudmi).dis.rcItem,
                );
                CloseThemeData(theme);
            }
        }

        _ => {}
    };
}

pub fn should_use_dark_mode(hwnd: HWND) -> bool {
    should_apps_use_dark_mode() && !is_high_contrast() && is_dark_mode_allowed_for_window(hwnd)
}

static HUXTHEME: Lazy<HMODULE> = Lazy::new(|| unsafe { LoadLibraryA(s!("uxtheme.dll")) });

fn should_apps_use_dark_mode() -> bool {
    const UXTHEME_SHOULDAPPSUSEDARKMODE_ORDINAL: u16 = 132;
    type ShouldAppsUseDarkMode = unsafe extern "system" fn() -> bool;
    static SHOULD_APPS_USE_DARK_MODE: Lazy<Option<ShouldAppsUseDarkMode>> = Lazy::new(|| unsafe {
        if *HUXTHEME == 0 {
            return None;
        }

        GetProcAddress(
            *HUXTHEME,
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
    static IS_DARK_MODE_ALLOWED_FOR_WINDOW: Lazy<Option<IsDarkModeAllowedForWindow>> =
        Lazy::new(|| unsafe {
            if *HUXTHEME == 0 {
                return None;
            }

            GetProcAddress(
                *HUXTHEME,
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
