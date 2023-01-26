# Changelog

## \[0.4.0]

- Bump gtk version: 0.15 -> 0.16
  - [fb3d0aa](https://www.github.com/tauri-apps/muda/commit/fb3d0aa303a0ee4ffff6d3de97cc363f1ef6d34b) chore(deps): bump gtk version 0.15 -> 0.16 ([#38](https://www.github.com/tauri-apps/muda/pull/38)) on 2023-01-26

## \[0.3.0]

- Add `MenuEvent::set_event_handler` to set a handler for new menu events.
  - [f871c68](https://www.github.com/tauri-apps/muda/commit/f871c68e81aa10f9541c386615a05a2e455e5a82) refactor: allow changing the menu event sender ([#35](https://www.github.com/tauri-apps/muda/pull/35)) on 2023-01-03
- **Breaking change** Remove `menu_event_receiver` function, use `MenuEvent::receiver` instead.
  - [f871c68](https://www.github.com/tauri-apps/muda/commit/f871c68e81aa10f9541c386615a05a2e455e5a82) refactor: allow changing the menu event sender ([#35](https://www.github.com/tauri-apps/muda/pull/35)) on 2023-01-03

## \[0.2.0]

- Add `IconMenuItem`
  - [7fc1b02](https://www.github.com/tauri-apps/muda/commit/7fc1b02cac65f2524220cb79263643505e286863) feat: add `IconMenuItem`, closes [#30](https://www.github.com/tauri-apps/muda/pull/30) ([#32](https://www.github.com/tauri-apps/muda/pull/32)) on 2022-12-30

## \[0.1.1]

- Derive `Copy` for `Accelerator` type.
  - [e80c113](https://www.github.com/tauri-apps/muda/commit/e80c113d8c8db8137f97829b071b443772d4805c) feat: derive `Copy` for `Accelerator` on 2022-12-12
- Fix parsing one letter string as valid accelerator without modifiers.
  - [0173987](https://www.github.com/tauri-apps/muda/commit/0173987ed5da605ddc20e49fce57ba884ed0d5f4) fix: parse one letter string to valid accelerator ([#28](https://www.github.com/tauri-apps/muda/pull/28)) on 2022-12-20

## \[0.1.0]

- Initial Release.
  - [0309d10](https://www.github.com/tauri-apps/muda/commit/0309d101b16663ce67b518f8aa1d2c4af0de6dee) chore: prepare for first release on 2022-12-05
