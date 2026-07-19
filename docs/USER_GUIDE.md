# SnapCode · User Guide

[Back to project home (GitHub README)](../README.md)

SnapCode is a small Windows utility that lives in the system tray: it listens for the SMS notifications that Phone Link (手机连接) syncs to the Windows notification center, automatically extracts the verification codes they contain, and saves them to a local history. When a code arrives it is copied to the clipboard right away; afterwards, pressing `Ctrl+Shift+V` (customizable) in any app pastes the latest code straight into the current input field — no more picking up your phone, no more switching back and forth between windows.

## Features at a glance

- **Automatic capture**: listens to Windows system notifications in real time; SMS pushed by Phone Link is recognized the moment it arrives
- **Smart recognition**: automatically extracts verification codes from common Chinese and English phrasings (e.g. 「验证码 123456」, "code is 123456")
- **Copy on arrival**: recognized codes are written to the clipboard automatically (can be turned off in Settings)
- **One-keystroke paste**: press `Ctrl+Shift+V` and the latest code is pasted directly into the focused input field
- **History**: codes are saved locally; view, copy, or delete them, with a retention of 1 / 3 / 7 / 30 days or forever
- **Extensible sources**: listens to Phone Link by default; other phone-companion apps can be added as notification sources
- **Email codes**: besides SMS, SnapCode can watch multiple email accounts via POP3 polling or IMAP (IDLE real-time push) and recognize codes in newly arrived mail
- **Quietly resident**: closing the window minimizes SnapCode to the system tray where it keeps listening; supports pausing the listener and launching at login
- **Auto-update**: checks for new versions at startup; signature-verified, one-click in-app updates — you choose whether to install
- **Light & dark themes**: Light / Dark / Follow system

## Requirements

- Windows 10 version 1809 or later / Windows 11
- WebView2 Runtime (usually built into Windows 11 and recent Windows 10; if missing, download and install the Evergreen Bootstrapper from Microsoft's website)

The above are runtime requirements. Node.js, Rust, and the like are development-only dependencies that regular users do not need; to build from source yourself, see the Getting started section of [README.md](../README.md).

## Installation

1. Open the [Releases page](https://github.com/Dream-XJ/snapcode/releases) of the GitHub repository;
2. Download the latest Windows installer (either `.msi` or `.exe`);
3. Double-click the installer and follow the prompts, then launch **SnapCode** from the Start menu.

## Granting notification access

SnapCode relies on Windows notification access to read SMS notifications:

1. On **first launch**, Windows pops up a notification access request — click "Allow".
2. If you missed the prompt or need to enable it manually:
   - Open Windows **Settings → Privacy & security → Notifications** (设置 → 隐私和安全性 → 通知);
   - Turn on "**Let apps access notifications**" (called "Notification access" in some versions);
   - Find **SnapCode** in the app list and turn on its toggle.
3. The app also provides an entry point: when access is denied, the "Notification listener" (通知监听) section of the Settings page shows "**Open system settings**" (打开系统设置 — jumps straight to the system settings page above) and "**Retry**" (重新检测 — click it after granting access to refresh the status).

The access status is shown in real time at the top of the main window. Without access the app still opens, but it cannot capture any codes.

## Phone-side prerequisites

SnapCode does not talk to your phone directly — SMS reaches Windows notifications via Phone Link, so pairing between the phone and the PC must be set up first.

### iOS (iPhone)

- Requires **iOS 16 or later**;
- Pair your iPhone with the PC in the "**Phone Link**" (手机连接) app;
- Keep the **Bluetooth connection** between the iPhone and the PC while in use (SMS sync on iOS depends on Bluetooth);
- On the iPhone, go to **Settings → Notifications → Messages → Show Previews** (设置 → 通知 → 信息 → 显示预览) and set it to "Always" or "When Unlocked" — if it is set to "Never", notification bodies contain no message content and SnapCode has nothing to extract codes from.

### Android

- Pair with the PC in the phone's "**Link to Windows**" (连接至 Windows) app and enable **SMS sync**;
- Confirm that SMS notifications actually reach the Windows notification center.

## Usage

### First-run onboarding

On first launch an onboarding page briefly introduces how SnapCode works — "listen to notifications → detect codes → quick paste" — and walks you through granting notification access. Once access is granted, click "**Get started**" (我已完成授权，开始使用) to enter the main window.

### Tray and window

- **Left-click the tray icon**: open the main window;
- **Right-click the tray icon**: menu — **Open SnapCode** (打开主窗口) / **Pause listening** (暂停监听; shown as "Resume listening" 恢复监听 while paused) / **Quit** (退出);
- **Closing the main window does not exit the program** — it only hides to the tray and keeps listening; choosing "Quit" in the tray menu is what actually terminates it.

### Pasting a code

Press `Ctrl+Shift+V` in any app's input field: SnapCode takes the latest code, writes it to the clipboard, and simulates a `Ctrl+V` to paste it into the currently focused input field.

The shortcut can be changed in Settings, in the format `Modifier+Key` (e.g. `Ctrl+Shift+V`, `Alt+C`). If it conflicts with another program's shortcut, saving will fail with a prompt — pick a different combination and try again.

### History

The main window lists captured codes in chronological order; each one can be copied or deleted individually. Expired records are cleaned up automatically according to the retention policy, and Settings offers a one-click way to clear the entire history.

### Settings overview

| Setting | Description | Default |
| --- | --- | --- |
| Theme | Light / Dark / Follow system | Follow system |
| Paste shortcut | Paste the latest code into the focused input | `Ctrl+Shift+V` |
| Auto copy | Write recognized codes to the clipboard as soon as they arrive | On |
| Launch at startup | Start automatically after signing in to Windows | On |
| Notification sources | App sources (AUMIDs) to listen to; multiple entries allowed, substring matching | `Microsoft.YourPhone_8wekyb3d8bbwe` (Phone Link) |
| Email codes | Multiple accounts, each polling via POP3 or watching via IMAP (IDLE push) to detect codes in new mail | Off |
| Retention | Keep history for 1 / 3 / 7 / 30 days or forever | 7 days |
| Clear history | Delete all history records in one click | — |
| Simulate notification | Enter a piece of text to verify the parse-and-store pipeline | — |
| List system notifications | List the real AUMIDs of current system notifications; add one as a source in one click | — |

### Custom source AUMIDs

An AUMID (App User Model ID) is the ID Windows uses to identify which app a notification comes from. SnapCode only processes notifications from apps on the "Notification sources" list, matched with **case-insensitive substring matching** — for example, the default `Microsoft.YourPhone_8wekyb3d8bbwe` also matches suffixed variants such as `Microsoft.YourPhone_8wekyb3d8bbwe!App`.

By adding other AUMIDs, SnapCode can also capture SMS notifications pushed by phone-companion software other than Phone Link (such as a vendor's own companion app):

1. Have the target app push a notification and keep it in the Windows notification center;
2. Open SnapCode **Settings → Debug → List system notifications** (设置 → 调试 → 列出系统通知), find that app's notification, and confirm its real AUMID;
3. Click "**Add as source**" (加为来源) on the notification card (you can also type the AUMID manually under "Notification sources").

From then on, code-bearing SMS pushed by that app is captured automatically.

> Deleted the default Phone Link source by mistake? A "**Restore default source (Phone Link)**" (恢复默认来源（手机连接）) entry appears at the bottom of the "Notification sources" list — one click adds it back.

### Email codes

Besides SMS, SnapCode can also watch your mailboxes for verification codes: you can add **multiple email accounts**, each independently using **POP3 (polling)** or **IMAP (IDLE real-time push)**, and detected codes run through exactly the same "store → copy → paste" pipeline as SMS codes.

**Step 1: enable POP3 / IMAP in your mailbox and get an auth code**

Most providers keep client protocols disabled by default. Enable them in the webmail settings and generate a "client auth code" (not your login password):

- **QQ Mail** (QQ邮箱): Settings → Account → "POP3/IMAP/SMTP…" → enable POP3/SMTP or IMAP/SMTP (whichever protocol you plan to use) and generate an auth code;
- **163 Mail** (网易163): Settings → "POP3/SMTP/IMAP" → enable POP3/SMTP or IMAP/SMTP and generate an auth code;
- **Outlook**: usually your account password works directly (accounts with two-step verification need an app password); the server is `outlook.office365.com`;
- **Gmail is not supported**: Gmail requires OAuth2 sign-in for IMAP/POP3, which this version cannot do — please don't add Gmail accounts.

**Step 2: add the account in SnapCode**

Open **Settings → Email codes** (设置 → 邮箱验证码), click "**Add account**" (添加账户), and fill in the editor:

| Field | Description | Example |
| --- | --- | --- |
| Label | Optional display name; the account address is shown when empty. History entries carry it, so you can tell which account a code came from | `Work` |
| Protocol | `POP3 (polling)` or `IMAP (IDLE push)` — see the comparison below | IMAP |
| Server / Port | Server address and port; switching the protocol flips the port between 995 ↔ 993 automatically | `imap.qq.com` / `993` |
| Account | Usually your full email address | `you@qq.com` |
| Auth code | The client auth code from step 1 (not the login password) | — |
| Poll interval | How often new mail is checked (30 s / 1 / 2 / 5 min); for IMAP it only applies when the server does not support IDLE | 1 min |
| SSL/TLS | Keep on; turn off only for plaintext (POP3 port 110 / IMAP port 143) | On |

Then click "**Test connection**" (测试连接) to verify — on success it shows how many messages the mailbox holds.

Saved accounts appear as cards showing a status dot, the display name, a protocol badge (POP3 / IMAP), and an on/off toggle; expand a card to edit or delete it. Adding, changing, or removing accounts **takes effect immediately — no restart needed**. The "Pause" button in the top bar pauses notification listening and all email accounts together.

**POP3 or IMAP?**

- **IMAP (recommended)**: when the server supports IDLE, new mail is **pushed in real time** and codes arrive with almost no delay; servers without IDLE automatically fall back to polling at the configured interval, behaving like POP3; dropped connections are re-established automatically.
- **POP3 (polling)**: checks for new mail on a fixed interval — the most widely compatible option, with timeliness determined by the poll interval.

**Things to know:**

- **Existing mail is not imported when an account is first enabled**: that account's first connection only establishes a baseline; only mail arriving afterwards is recognized. Baselines are per-account — deleting an account and adding it back rebuilds its baseline;
- Each account's state is shown by the status dot on its card: green for **Watching** (监听中), grey for **Paused / Disabled** (已暂停 / 未启用), red for **Error** (轮询出错, with the reason attached);
- The auth code is stored **in plaintext in the local** `settings.json`, like every other setting, and is never sent anywhere except your mail server (see "Privacy").

### Debugging tools

The "Debug" (调试) section at the bottom of the Settings page provides two troubleshooting tools:

- **Simulate notification** (模拟通知): enter a piece of text (e.g. "Your verification code is 482913, valid for 5 minutes.") and run it through the full detect-and-store pipeline to verify parsing;
- **List system notifications** (列出系统通知): lists the toasts currently in the Windows notification center with their real AUMIDs — when no codes are captured, use it to confirm whether the source is configured correctly, and add one as a source in one click.

### App updates

SnapCode checks for a new release on every launch (a failed check, e.g. while offline, stays silent). When one is found, a dialog shows the new version and its release notes:

- Click "**Update now**" (立即更新) to download the update; the installer then takes over, finishes the update, and relaunches the app automatically;
- Click "**Not now**" (暂不更新) to dismiss — you will be reminded again on the next launch.

You can also check manually at any time via **Settings → About → App update** (设置 → 关于 → 版本更新). All update packages are signature-verified and come only from this project's GitHub Releases.

## Privacy

All codes and history are stored **only on this machine** (in a local SQLite database). SnapCode uploads nothing over the network and collects no data. You can delete individual records, clear the entire history in one click, or control how long data is kept via the retention policy at any time. If email listening is configured, each account's auth code is likewise stored only in the local `settings.json` (in plaintext) and is never sent anywhere except its own mail server.

## FAQ

**Not receiving codes?**

- Confirm the phone shows as **connected** in Phone Link and that SMS sync is enabled;
- Confirm Windows notification access has been granted to SnapCode (see "Granting notification access" above);
- Confirm the SMS notification actually appears in the Windows notification center — if it is not there, the problem is on the phone-sync side;
- First use "**Simulate notification**" (模拟通知) in the Settings Debug section to verify the detect-and-store pipeline works, then use "**List system notifications**" (列出系统通知) to confirm the source AUMID is on the listen list.

**Not receiving email codes?**

- First verify the configuration with "**Test connection**" (测试连接) in that account's editor — failures are reported with their cause (server unreachable / bad auth code). For QQ/163 and similar providers you must use the **auth code**, not the login password;
- Check the status dot on the account card: red means error (the reason is shown), grey means disabled or globally paused;
- Make sure the mail arrived **after** that account was enabled — existing mail is never imported on first enable;
- Make sure POP3 / IMAP is still enabled in the webmail settings; some providers invalidate auth codes when security settings change — generate a fresh one if in doubt;
- With IMAP, new mail usually arrives instantly; if the server does not support IDLE or you use POP3, timeliness depends on the poll interval — don't poll more often than every 30 seconds, as aggressive polling may be rejected by the server;
- Gmail requires OAuth2 sign-in for IMAP/POP3 and is not supported in this version — a Gmail account will keep reporting errors.

**iPhone not receiving codes, or suddenly stopped receiving them?**

- **A dropped Bluetooth connection is the most common cause**: iOS SMS sync depends on Bluetooth — check the Bluetooth connection between the iPhone and the PC, and re-pair in Phone Link if necessary;
- Check whether the iPhone's **Settings → Notifications → Messages → Show Previews** (设置 → 通知 → 信息 → 显示预览) has been set to "Never" — change it to "Always" or "When Unlocked".

**Shortcut not responding?**

- The shortcut may be occupied by another program: change it to a different combination in Settings (saving fails with a prompt when it is taken);
- Confirm SnapCode is running (its icon is in the tray) and listening is not paused.

**Pressed the shortcut, but nothing was pasted into the target app?**

- If the target app **runs as administrator**, Windows security mechanisms prevent a normal-privilege SnapCode from injecting keystrokes into it. The code is still copied to the clipboard in this case — press `Ctrl+V` manually in that input field;
- If you frequently need to paste into such apps, right-click SnapCode's shortcut and choose "Run as administrator".

**Why do codes sometimes appear a second or two late?**

- On some Windows versions, the OS does not allow unpackaged apps (those without package identity) to subscribe to real-time notification events. In that case SnapCode automatically falls back to **polling system notifications once per second** — functionally equivalent and usually only about 1 second late. This is normal and needs no action.

**Is the program still running after the window is closed?**

- Yes. Closing the window only hides it to the system tray and listening continues; only choosing "Quit" in the tray menu actually terminates the program.

**Update check failed?**

- Update checks need access to GitHub Releases. If direct access is unstable on your network, enable your proxy client's "system proxy" or TUN mode and retry. v0.2.0's update check ignored the system proxy (always connected directly) — upgrade to v0.2.1 or later;
- For a few minutes right after a release is published, `releases/latest` may still point to the previous release and return 404 — just retry later;
- The error message includes the concrete cause (HTTP status or network error) to help troubleshooting. You can also grab the latest installer from the [Releases page](https://github.com/Dream-XJ/snapcode/releases) and install it on top manually.

---

SnapCode is open source under the MIT License; the full license text is in [LICENSE](../LICENSE). Project home and source code: <https://github.com/Dream-XJ/snapcode>
