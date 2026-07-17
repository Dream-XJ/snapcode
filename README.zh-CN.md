[English](README.md) | **简体中文**

<div align="center">
  <img src="src-tauri/icons/icon.png" alt="SnapCode 闪码 logo" width="96" />
  <h1>SnapCode 闪码</h1>
  <p><strong>从 Windows 通知中自动捕获短信验证码，按一下全局快捷键即可粘贴到任何地方。</strong></p>
  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT" /></a>
    <a href="https://github.com/Dream-XJ/snapcode"><img src="https://img.shields.io/badge/Platform-Windows-blue.svg" alt="Platform: Windows" /></a>
    <a href="https://tauri.app"><img src="https://img.shields.io/badge/Built%20with-Tauri%20v2-orange.svg" alt="Built with Tauri v2" /></a>
  </p>
</div>

<!-- screenshot placeholder -->

SnapCode 闪码是一款 Windows 桌面工具：监听手机通过「手机连接」(Phone Link) 推送到 Windows 的短信 Toast，自动提取其中的验证码，让验证码始终只差一次按键。新验证码到达时立即复制到剪贴板；在任何输入框按下 `Ctrl+Shift+V`，即可把最新一条验证码直接粘贴进去——不用掏手机，不用手打。

## 功能特性

- **通知监听**——通过 windows-rs 调用 WinRT `UserNotificationListener` 实时捕获短信 Toast。当系统不允许无包身份应用订阅通知事件时，自动降级为 1 秒轮询，功能完全等价（最多约 1 秒延迟）。
- **验证码自动提取**——双规则解析器：关键词规则覆盖常见中英文验证码文案（「验证码 123456」、"code is 123456"），辅以通用数字序列兜底规则；配有单元测试。
- **收到即复制**——识别到验证码后立即写入系统剪贴板，可在设置中关闭。
- **全局快捷粘贴**——`Ctrl+Shift+V`（可自定义）把最新验证码粘贴到当前焦点输入框：后端会等待修饰键物理松开后，再通过 `SendInput` 回放 `Ctrl+V`。快捷键被占用时会检测并提示，方便更换组合。
- **本地历史记录**——所有验证码存入本机 SQLite 数据库，支持搜索、一键复制、逐条删除；保留策略可选 1 / 3 / 7 / 30 天或永久（默认 7 天），另有一键清空。
- **来源可配置**——按 AUMID 过滤通知，大小写无关的包含匹配。默认内置「手机连接」的 AUMID（`Microsoft.YourPhone_8wekyb3d8bbwe`），可添加多个来源（如厂商自家的手机互联应用）；误删默认来源后，列表底部会出现一键恢复链接。
- **系统托盘常驻**——关闭窗口后继续在托盘监听；托盘菜单提供 打开主窗口 / 暂停（恢复）监听 / 退出。单实例运行。
- **开机自启**——可选随 Windows 登录自动启动（默认开启）。
- **首次引导**——首次启动时引导完成通知访问权限授予。
- **亮暗双主题**，搭配自定义无边框标题栏。
- **内置调试**——「模拟通知」可输入任意文本走完整的解析 + 入库流程；「列出系统通知」显示当前系统 Toast 的真实 AUMID，可一键加为来源。

## 工作原理

「手机连接」(Phone Link) 把手机短信镜像为 Windows Toast 通知 → SnapCode 通过 WinRT `UserNotificationListener` 订阅这些通知（事件订阅不可用时自动降级为 1 秒轮询）→ 来自已配置来源 AUMID 的每条 Toast 都会经过正则验证码解析器 → 识别到的验证码写入 SQLite 并复制到剪贴板 → 按下全局快捷键时，先等待修饰键松开，再通过 `SendInput` 把最新验证码粘贴到当前焦点输入框。

## 环境要求

- Windows 10 1809 或更高版本 / Windows 11
- Node.js 18+
- Rust stable（MSVC 工具链）
- WebView2 运行时（Windows 11 及新版 Windows 10 通常已内置；如无请从微软官网安装 Evergreen Bootstrapper）
- 手机端：iPhone 要求 iOS 16+、与电脑保持蓝牙连接、信息通知预览开启，并在「手机连接」中开启短信同步；Android 需在「连接至 Windows」(Link to Windows) 应用中开启短信同步

## 快速开始

```bash
# 安装依赖
npm install

# 开发模式（前端热更新 + 桌面窗口）
npm run tauri dev

# 打包发布版本
npm run tauri build

# 运行验证码解析器单元测试
cargo test --manifest-path src-tauri/Cargo.toml
```

## 使用方法

1. **授予通知权限。** 首次启动时 Windows 会弹出通知访问请求，请点击「允许」。若错过弹窗，可到 Windows 设置 → 隐私和安全性 → 通知 中开启，或使用应用内的「重试授权」/「打开通知设置」按钮。
2. **接收验证码。** 手机收到含验证码的短信后，「手机连接」会将其推送到 Windows；SnapCode 自动提取验证码、存入历史记录，并（默认）复制到剪贴板。
3. **随处粘贴。** 在任意输入框按下 `Ctrl+Shift+V` 即可粘贴最新验证码；也可以在历史记录中点击任意一条记录重新复制。
4. **托盘控制。** 左键点击托盘图标重新打开窗口；托盘菜单可暂停/恢复监听或退出程序。关闭窗口不会停止监听——只有「退出」才会。

## 配置项

以下设置均位于设置页，全部保存在本机：

| 设置项 | 说明 | 默认值 |
| --- | --- | --- |
| 全局快捷键 | 粘贴最新验证码到当前焦点（格式 `修饰键+键名`，如 `Ctrl+Shift+V`、`Alt+C`） | `Ctrl+Shift+V` |
| 自动复制 | 收到验证码即写入剪贴板 | 开启 |
| 开机自启 | 登录 Windows 后自动启动 | 开启 |
| 来源 AUMID | 监听的推送来源，可添加多个，大小写无关的包含匹配 | `Microsoft.YourPhone_8wekyb3d8bbwe` |
| 保留策略 | 历史记录保留 1 / 3 / 7 / 30 天或永久 | 7 天 |
| 一键清空 | 清空全部历史记录 | — |
| 模拟通知 | 输入任意文本，走完整的解析 + 入库调试流程 | — |
| 列出系统通知 | 显示当前系统 Toast 的真实 AUMID，可一键加为来源 | — |

添加其他来源 AUMID 后，SnapCode 也能捕获「手机连接」以外的互联应用推送的验证码——完整操作步骤见使用指南。

## 文档

- [User Guide (English)](docs/USER_GUIDE.md)——English edition of the complete setup and usage guide.
- [中文使用指南](docs/USER_GUIDE.zh-CN.md)——完整的安装与使用说明：通知权限授予、手机端前提、自定义 AUMID 来源、常见问题。
- [AGENTS.md](AGENTS.md)——开发者/代理指南：项目结构、约定与前后端契约（14 个命令 + `code-added`、`listener-status`、`shortcut-error` 事件；见 `src/lib/tauri.ts`、`src/types.ts`、`src-tauri/src/commands.rs`）。

## 隐私

SnapCode 完全在本地运行。所有验证码与历史记录仅保存在本机 SQLite 数据库中，不进行任何网络上传，不收集任何数据。随时可通过删除单条记录、清空历史或缩短保留策略来控制数据留存。

## 技术栈

- **外壳**——Tauri v2，配合 single-instance、global-shortcut、autostart、clipboard-manager 插件
- **前端**——React 18、TypeScript、Vite、Tailwind CSS v3、shadcn/ui 风格组件（Radix 原语、lucide-react、sonner）
- **后端**——Rust：windows-rs（WinRT `UserNotificationListener`、`SendInput`）、rusqlite（内置捆绑 SQLite）、regex

## 许可证

[MIT](LICENSE)
