# Tendedero for Windows (Rust Edition)

> A lightweight, blazing-fast **Rust & Tauri 2 port** of the macOS clothesline utility:
> screenshots, clipboard copies, and **TODO sticky notes** hang gracefully on a clothesline at the top of your screen.
> Rest your pointer at the top edge for 0.2s (or press `Ctrl+Alt+T`), and the line slides down with your cards. Move away or click outside, and it tucks back with full mouse pass-through.
>
> Inspired by the macOS original: [**alejandrobujan/tendedero**](https://github.com/alejandrobujan/tendedero) (MIT, by Alejandro Buján).
> Re-architected from scratch for Windows in pure **Rust**, completely replacing the heavy Electron runtime with native Win32 GDI & lightweight WebView2.

```
                                            ╭─╮ ╭─╮ ╭─╮
   ──●──────────────────────────────────────┴─┴─┴─┴─┴──────────   ← the line
   (rest pointer at the top edge for 0.2 s or press Ctrl+Alt+T to slide down)
```

[**简体中文文档**](README.zh-CN.md) | [**Download Latest Release**](https://github.com/aa1343/tendedero-rs/releases)

---

## ⚡ Rust Upgrade & Performance Comparison

| Metric | Original Electron Port | **New Rust (Tauri 2) Edition** | Improvement |
|---|---|---|---|
| **Portable Zip Size** | 104 MB (Extracted 250+ MB) | **3.0 MB** | **-96% smaller** |
| **RAM Usage** | 200 MB ~ 300 MB | **~25 MB** | **-90% memory** |
| **Runtime Architecture** | Heavy Chromium + Node.js | **Rust + Win32 GDI + System WebView2** | Ultra lightweight |
| **Cold Startup Latency** | 1.8 ~ 3.5 seconds | **< 0.1 second** (Instant) | Instant response |
| **New Capabilities** | Basic screenshots & copies | **+ Sticky Todo Notes & Native Auto-start** | Full productivity suite |

---

## ✨ Features

| Feature | Description |
|---|---|
| 📸 **Instant Screen Capture & 100% Lossless Original Quality** | Press `Ctrl+Alt+S` to capture your display. Gone are blurry downscaled thumbnails—**hangs 100% full-resolution lossless original images** with GPU hardware contrast optimization so fine text and sharp lines stay razor-sharp; features an intelligent 3.5s auto-preview grace period. |
| 📋 **Smart Clipboard Watcher** | Automatically hangs newly copied images, plain text, and web links (sensitive/password fields are never captured). |
| 📝 **NEW: Sticky Todo Notes** | Hang colorful sticky notes on the line! Check off tasks directly on the card, press Enter to add items inline, track real-time progress bars, and customize 5 theme colors (`Ctrl+Alt+N`). |
| 🗑️ **NEW: Quick Del Key Removal** | Hover over any card and press `Del` / `Delete` to take it down with a realistic falling drop animation. Supports rapid consecutive deletions and `Ctrl+Z` multi-step undo. |
| 🧺 **NEW: Auto-Tuck on Shutdown & Local Archive** | Cards without pins are automatically taken down and archived to local storage upon shutdown or app exit (persisted for 1 day, auto-cleaned after 3 days). |
| ⚡ **NEW: Zero-Delay Auto-Tuck** | Removed the awkward 450ms wait delay—the clothesline now tucks back within 40ms the moment your pointer leaves the card area, instantly restoring click-through pass-through. |
| 🚀 **NEW: Auto-start on Boot** | Right-click the tray icon to toggle "Launch at Login" anytime. Built with native Win32 registry standards (`HKCU\...\Run`), completely clean with zero background bloat. |
| 👻 **Flawless Mouse Pass-Through** | 100% click-through pass-through when tucked in—never blocks clicks on your browser tabs, window titlebars, or desktop icons. |
| 🔍 **Line Search** | Press `Ctrl+Alt+F` to search through notes, todos, and card titles across the clothesline. |
| 📌 **Pinning & Safety** | Pinned cards are protected from auto-cleaning; press `Ctrl+Z` anytime to undo card take-downs. |
| 🔒 **100% Offline & Private** | Zero telemetry, zero analytics, zero external network requests. Everything stays strictly on your local disk. |

---

## ⌨️ Global Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+Alt+T` | **Show / Hide Clothesline** (slide down or tuck back) |
| `Ctrl+Alt+N` | **Create New Sticky Todo Note** directly onto the line |
| `Ctrl+Alt+S` | **Capture Screen Immediately** and hang it up |
| `Ctrl+Alt+F` | **Search Clothesline** (todos, texts, and links) |
| `Ctrl+Z` | **Undo Last Removal** (restore deleted cards or todos) |
| `Del` (hovering) | **Take Down Card** under current pointer |
| `Esc` | Close search / todo modal or tuck the line |

---

## 👆 Gestures & Card Interactions

- **Left Click**: Copy content to clipboard (images copied as bitmaps, text/todos copied as text) with visual toast confirmation.
- **Double Click**: Open card (links in default browser, images in default viewer, todos in full modal editor).
- **Pin Icon (Top-Right)**: Pin / Unpin the card. Pinned cards are permanently kept on the line.
- **Close Button (Top-Left)**: Take card down into app Trash (press `Ctrl+Z` to undo anytime).
- **Inline Checkbox (Todo)**: Click checkbox on the sticky note to toggle completion with a smooth strike-through animation.
- **Click Outside / Move Away**: Pointer moving down tucks the line back within 40ms; screenshots and copied items tuck away smoothly after a 3.5s grace period if untouched.

---

## 🎯 Sticky Todo Notes (Deep Dive)

The clothesline isn't just for temporary clippings anymore—it's your glanceable daily scratchpad:

1. **5 Pastel Color Themes**: Yellow, Green, Blue, Pink, and Purple sticky papers with realistic wooden clothespins.
2. **Instant Inline Item Creation**: Type a task at the bottom of the card and hit **Enter** to add it immediately.
3. **Live Progress Tracking**: See completion stats at a glance (e.g. `2/3 Completed (66%)`).
4. **Full Modal Editor**: Double-click any note to open the modal dialog for editing titles, re-ordering, changing palette colors, or mass cleanups.

---

## 🚀 How to Run

### Portable Version (Recommended)
Download the latest portable zip from [Releases](https://github.com/aa1343/tendedero-rs/releases):
1. Extract the zip archive anywhere you like.
2. Double-click **`Start Tendedero.vbs`** (silent launch without any console window) or `tendedero-rs.exe`.
3. The clothesline will slide down to welcome you, then tuck itself quietly to the top edge.
4. Move your mouse to the **very top edge of your screen** for 0.2s or press **`Ctrl+Alt+T`** to summon it anytime!

### Building from Source
Ensure you have the Rust toolchain installed:
```powershell
git clone https://github.com/aa1343/tendedero-rs.git
cd tendedero-rs

# Build optimized release binary
cargo build --release
```
The resulting single standalone binary will be located at `target/release/tendedero-rs.exe` (~9.8 MB).

---

## 📂 Data Storage

All data is stored locally in `%APPDATA%\tendedero-rs\` (accessible via Tray → "Open Data Folder"):

| Directory / File | Description |
|---|---|
| `Screenshots/` | Native screen captures and clipboard image cache |
| `Notes/` | Automatically captured text notes and web links |
| `Todos/` | Todo note items, titles, and color preferences |
| `Trash/` | Taken-down items (enables undo with `Ctrl+Z`) |
| `cards.json` | Master metadata index of all active cards on the line |
| `settings.json` | User configuration and preferences |

Uninstalling is as simple as deleting the application folder and `%APPDATA%\tendedero-rs\`—no background services, no drivers, and no lingering system modifications.

---

## 📜 License & Acknowledgements

- Licensed under the [MIT License](LICENSE).
- Based on the concept and Swift implementation of [**alejandrobujan/tendedero**](https://github.com/alejandrobujan/tendedero) (MIT, Alejandro Buján).
- Original assets and trademark belong to their respective owners; this project uses independent code-generated icons and clean-room implementation.
