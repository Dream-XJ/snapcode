**English** | [简体中文](README.zh-CN.md)

<div align="center">
  <img src="src-tauri/icons/icon.png" alt="SnapCode logo" width="96" />
  <h1>SnapCode 闪码</h1>
  <p><strong>Capture SMS 2FA codes from Windows notifications and paste them anywhere with one global hotkey.</strong></p>
  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT" /></a>
    <a href="https://github.com/Dream-XJ/snapcode"><img src="https://img.shields.io/badge/Platform-Windows-blue.svg" alt="Platform: Windows" /></a>
    <a href="https://tauri.app"><img src="https://img.shields.io/badge/Built%20with-Tauri%20v2-orange.svg" alt="Built with Tauri v2" /></a>
  </p>
</div>

<!-- screenshot placeholder -->

SnapCode is a Windows desktop utility that watches the SMS toasts your phone pushes to Windows through Phone Link (手机连接), extracts verification codes automatically, and keeps them one keystroke away. New codes are copied to the clipboard the moment they arrive; press `Ctrl+Shift+V` anywhere to paste the latest code straight into the focused input — no reaching for your phone, no retyping.

## Features

- **Notification listening** — captures SMS toasts in real time through the WinRT `UserNotificationListener` API (via windows-rs). When the OS refuses event subscription for unpackaged apps, SnapCode automatically falls back to a 1-second polling loop with equivalent behavior (at most ~1 s of added latency).
- **Automatic code extraction** — a dual-rule parser combines keyword rules for common Chinese and English code phrases (「验证码 123456」, "code is 123456") with a generic digit-sequence fallback. Covered by unit tests.
- **Copy on arrival** — recognized codes are written to the clipboard immediately; can be turned off in Settings.
- **Global paste hotkey** — `Ctrl+Shift+V` (customizable) pastes the newest code into the currently focused field. The backend waits until your modifier keys are physically released, then replays `Ctrl+V` via `SendInput`. Shortcut conflicts are detected and reported so you can pick another combo.
- **Local history** — every code is stored in a local SQLite database with search, one-click copy, and per-record deletion; retention is configurable at 1 / 3 / 7 / 30 days or forever (default: 7 days), plus a one-click "clear all".
- **Configurable sources** — notifications are filtered by AUMID with case-insensitive substring matching. Ships with the Phone Link AUMID (`Microsoft.YourPhone_8wekyb3d8bbwe`) and accepts additional sources, such as vendor phone-companion apps. A one-click restore link brings the default source back if you delete it by mistake.
- **System tray** — closing the window keeps SnapCode listening in the tray; the tray menu offers Open / Pause (Resume) listening / Quit. Single-instance enforced.
- **Launch at login** — optional autostart with Windows (on by default).
- **First-run onboarding** — walks you through granting notification access on first launch.
- **Light & dark themes** with a custom frameless title bar.
- **Built-in debugging** — simulate a notification to exercise the full parse-and-store pipeline, or list live system toasts with their real AUMIDs and add any of them as a source in one click.

## How it works

Phone Link (手机连接) mirrors your phone's SMS as Windows toast notifications → SnapCode subscribes to those toasts through the WinRT `UserNotificationListener` (automatically degrading to a 1-second poll when event subscription is unavailable) → every toast from a configured source AUMID is run through the regex code parser → recognized codes are stored in SQLite and copied to the clipboard → pressing the global hotkey waits for your modifier keys to be released, then pastes the latest code into the focused input via `SendInput`.

## Requirements

- Windows 10 version 1809 or later, or Windows 11
- Node.js 18+
- Rust stable (MSVC toolchain)
- WebView2 Runtime (preinstalled on Windows 11 and recent Windows 10; otherwise install the Evergreen Bootstrapper from Microsoft)
- Phone side: iPhone requires iOS 16+, a Bluetooth connection to the PC, message previews enabled, and SMS sync turned on in Phone Link; Android requires SMS sync enabled in the Link to Windows app

## Getting started

```bash
# Install dependencies
npm install

# Development mode (frontend HMR + desktop window)
npm run tauri dev

# Build a release bundle
npm run tauri build

# Run the code-parser unit tests
cargo test --manifest-path src-tauri/Cargo.toml
```

## Usage

1. **Grant notification access.** On first launch, Windows asks whether SnapCode may read your notifications — click Allow. If you missed the prompt, enable it under Windows Settings → Privacy & security → Notifications, or use the Retry / Open notification settings buttons in the app.
2. **Receive a code.** When an SMS containing a verification code arrives on your phone, Phone Link pushes it to Windows; SnapCode extracts the code, stores it in History, and (by default) copies it to the clipboard.
3. **Paste it anywhere.** Press `Ctrl+Shift+V` in any input field to paste the latest code. You can also click any record in History to copy it again.
4. **Tray control.** Left-click the tray icon to reopen the window; the tray menu lets you pause/resume listening or quit. Closing the window never stops listening — only Quit does.

## Configuration

Everything below lives on the Settings page and is stored locally:

| Setting | Description | Default |
| --- | --- | --- |
| Global shortcut | Paste the latest code into the focused input (`Modifier+Key`, e.g. `Ctrl+Shift+V`, `Alt+C`) | `Ctrl+Shift+V` |
| Auto-copy | Write each recognized code to the clipboard on arrival | On |
| Launch at login | Start SnapCode with Windows | On |
| Source AUMIDs | Notification sources to listen to; multiple entries, case-insensitive substring match | `Microsoft.YourPhone_8wekyb3d8bbwe` |
| Retention | Keep history for 1 / 3 / 7 / 30 days or forever | 7 days |
| Clear history | Delete all stored records | — |
| Simulate notification | Run any text through the full parse + store pipeline for debugging | — |
| List system notifications | Show live system toasts with their real AUMIDs; one click adds one as a source | — |

Adding extra source AUMIDs lets SnapCode capture codes pushed by apps other than Phone Link — see the user guide for the full walkthrough.

## Documentation

- [User Guide (English)](docs/USER_GUIDE.md) — complete setup and usage: notification permission, phone-side prerequisites, custom AUMID sources, FAQ.
- [中文使用指南 (User Guide, Simplified Chinese)](docs/USER_GUIDE.zh-CN.md) — complete setup and usage: notification permission, phone-side prerequisites, custom AUMID sources, FAQ.
- [AGENTS.md](AGENTS.md) — developer/agent guide: project structure, conventions, and the frontend↔backend contract (14 commands plus the `code-added`, `listener-status`, and `shortcut-error` events; see `src/lib/tauri.ts`, `src/types.ts`, and `src-tauri/src/commands.rs`).

## Privacy

SnapCode is fully local. All codes and history live in a SQLite database on your machine; nothing is uploaded anywhere and no data is collected. Delete individual records, clear all history, or shorten the retention window at any time to control what is kept.

## Tech stack

- **Shell** — Tauri v2 with the single-instance, global-shortcut, autostart, and clipboard-manager plugins
- **Frontend** — React 18, TypeScript, Vite, Tailwind CSS v3, shadcn/ui-style components (Radix primitives, lucide-react, sonner)
- **Backend** — Rust: windows-rs (WinRT `UserNotificationListener`, `SendInput`), rusqlite (bundled SQLite), regex

## License

[MIT](LICENSE)
