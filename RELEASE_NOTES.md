**SnapCode v0.2.0** — 应用内一键更新 / In-app updates

## 新功能 / What's New

- **应用内自动更新**：启动时自动检查新版本（也可在「设置 → 关于 → 版本更新」手动检查）。发现新版本时弹窗展示版本号与更新说明，点击「立即更新」即自动下载、签名校验、安装并重启——是否更新由你决定。
  **In-app updates**: SnapCode checks for new releases at startup (or manually via Settings → About). When one is found, a dialog shows the new version and its release notes; click "Update now" to download, verify, install, and relaunch — you choose whether to update.
- **统一英文名**：移除中文名「闪码」，应用名称统一为 **SnapCode**（开始菜单快捷方式同步更名并清理旧名残留）。
  **Renamed**: the Chinese name has been dropped; the app is now simply **SnapCode** everywhere, including the Start Menu shortcut.

## 修复 / Fixes

- **开机自启自愈**：每次启动时重写自启动注册表项，修复 exe 移动或覆盖安装后自启动路径失效且无法自动恢复的问题。
  **Autostart self-heal**: the startup registry entry is rewritten on every launch, so a stale path (e.g. after moving or reinstalling the app) now repairs itself.

## 安装说明 / Installation

- 运行要求：Windows 10 1809+ / Windows 11。
- 推荐使用 **MSI 安装包**：应用内一键更新按 MSI 原位升级，体验最佳（NSIS 安装包亦可手动安装）。
- **从 v0.1.0 升级**：v0.1.0 不含更新功能，请手动下载安装 v0.2.0（直接覆盖安装，历史记录与设置均保留）。自 v0.2.0 起，后续版本均可在应用内一键更新。

Requires Windows 10 1809+ / Windows 11. The **MSI installer** is recommended — in-app updates upgrade MSI installations in place. **Upgrading from v0.1.0**: v0.1.0 has no updater, so install v0.2.0 manually on top (your history and settings are preserved); from v0.2.0 onward, updates are one-click inside the app.
