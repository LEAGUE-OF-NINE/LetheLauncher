# Lethe Launcher

A custom launcher for Limbus Company that provides intelligent file management, Steam integration, and modding support via BepInEx.

Built in Rust with [GPUI](https://www.gpui.rs) and [GPUI Kit](https://gpui-kit.com) components.

## Building

```sh
cd lethe-launcher
cargo build --release   # target/release/LimbusCompany.exe
cargo test
```

`rust-toolchain.toml` pins the Rust version GPUI needs; rustup installs it on the first build.

Put `LimbusCompany.exe` in the game folder next to `UnityPlayer.dll`, the same as before.

## Customizing

- **Look:** the launcher follows lethelc.site: Barlow Condensed headings, Inter body text, crimson primary, gold accents, square corners, notched cards. All colors and fonts live in `lethe-launcher/src/ui/theme.rs`, so a site restyle only needs changes there.
- **Fonts:** bundled in `lethe-launcher/assets/fonts` under the SIL Open Font License (license texts included).
- **Background art:** replace `lethe-launcher/assets/background.jpg` (16:9, 1920x1080). It is compiled into the exe. Keep the left side and bottom corners dark, since the title, news card and controls sit there.
- **Logo and exe icon:** `lethe-launcher/assets/logo.png` (title bar) and `lethe-launcher/assets/icon.ico` (exe), both the Lethe rose.
- **News card:** the launcher reads `https://files.lethelc.site/news.json` (on the CDN, next to the game files). If that file is missing, the card is hidden. If the CDN caches it, purge the cache or keep the cache time short after editing it, or players will see the old news. Format:

  ```json
  {
    "banners": [
      { "image": "https://example.com/event.png", "title": "Custom identities", "subtitle": "Mod by Water", "url": "https://example.com/event" }
    ],
    "items": [
      { "title": "Server maintenance complete", "date": "11/04", "url": "https://..." },
      { "title": "Patch 1.4.2", "date": "11/03" }
    ]
  }
  ```

  The card shows the first 4 items, so list the newest first. `url` is optional, and so are a banner's `title` and `subtitle` (shown as a caption). The card shows up to 5 banners, rotating every 7 seconds.

- **Previewing UI states:** debug builds accept `LETHE_DEMO=login|checking|syncing|ready|error|update`. Each value shows that state with fake data and makes no network calls.

## Logging

All operations are logged to `lethe-launcher.log` with timestamps:

```
=== Lethe Launcher Started at 2025-10-26 12:43:48 ===
[12:43:48] Configuration loaded from lethe-launcher.ini:
[12:43:48]   DisableAutoUpdate = false
[12:43:49] Downloaded manifest: 150 files, 2048576 bytes
[12:43:49] Starting file synchronization...
[12:43:50] Copied from local: Data/file.dat (1.2 MB)
[12:43:51] Downloaded: Assets/texture.png (512 KB)
[12:43:52] Update complete. Starting game...
[12:43:52] ✓ Found UnityPlayer.dll at: C:\Game\UnityPlayer.dll
[12:43:52] ✓ UnityPlayer.dll loaded successfully
[12:43:52] ✓ doorstop.dll loaded successfully
[12:43:52] UnityMain returned: 0
```
