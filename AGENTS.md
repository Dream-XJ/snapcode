# AGENTS.md — SnapCode 闪码

面向 AI 编码代理（Kimi Code 等）的项目指南。修改代码前请先读完本文件，尤其是「前后端契约」与「已知决策与坑」两节。

## 项目概览

SnapCode 闪码是一款 Windows 桌面工具：通过 WinRT `UserNotificationListener` 监听系统通知中由「手机连接」(Phone Link) 推送的短信 Toast，自动提取其中的短信验证码存入本地 SQLite，收到新码即复制到剪贴板；用户随时按下全局快捷键 **Ctrl+Shift+V**（可自定义），把最新一条验证码直接粘贴到当前焦点输入框。

一句话架构：**Tauri v2 应用 —— React 18 前端 ↔（14 个 invoke 命令 + 3 个事件）↔ Rust 后端 ↔ WinRT 通知/Toast/SendInput + rusqlite**。

- 技术栈：Tauri v2 + React 18 + TypeScript + Vite 5 + Tailwind CSS v3（shadcn 风格）+ Rust（windows-rs 0.58 / rusqlite bundled / regex）。
- 平台：**仅 Windows**（Windows 10 1809+）。Rust 侧 Windows 专用代码全部 `cfg(windows)`，非 Windows 平台保留编译可用的桩实现。
- 许可证：MIT（见 `LICENSE`，由远端合并引入）。
- 仓库：https://github.com/Dream-XJ/snapcode

## 目录结构与模块职责

### Rust 后端（`src-tauri/src/`）

| 文件 | 职责 |
| --- | --- |
| `main.rs` | 二进制入口，仅 `#![windows_subsystem = "windows"]` + 调用 `snapcode_lib::run()` |
| `lib.rs` | Tauri Builder 组装：插件（single-instance / autostart / global-shortcut / clipboard-manager）、`setup()` 初始化（目录、Settings、Db、AppState、托盘、热键、通知监听）、托盘菜单（打开/暂停/退出）、关闭窗口改为隐藏到托盘、`invoke_handler` 注册全部命令 |
| `state.rs` | `AppState`（db / settings / status / paused / monitor_alive / shortcut_error）与 `ListenerState`；`set_status()` 更新状态并广播 `listener-status` 事件 |
| `settings.rs` | `Settings` 结构体（serde）、`Default` 默认值、JSON 加载/保存（`app_config_dir/settings.json`，损坏时回退默认） |
| `storage.rs` | rusqlite 封装 `Db` 与 `CodeRecord`：`codes` 表 + `received_at` 索引，insert / list（LIKE 模糊过滤，上限 500 条）/ get / latest / clear / delete / mark_used / cleanup（按保留天数清理） |
| `parser.rs` | 验证码提取纯函数 `extract_code()`：规则 1 关键词（验证码/动态密码/verification code 等）后 12 字符内首个 4~8 位数字；规则 2 独立 4~6 位数字兜底（排除金额、年份、账号类语境）；含单元测试 |
| `notifications.rs` | 通知监听线程：`UserNotificationListener` 授权请求、Toast 来源 AUMID 过滤（大小写无关包含匹配）、解析→入库→广播 `code-added`→自动复制；事件订阅失败自动降级 1s 轮询；`dump_current_toasts()` 诊断用列出系统 Toast |
| `hotkey.rs` | 快捷键字符串解析（`"Ctrl+Shift+V"` → `Shortcut`）与全局注册；失败写入 `shortcut_error` 并广播 `shortcut-error`；含解析单测 |
| `paste.rs` | 快捷粘贴：取最新验证码 → 轮询 `GetAsyncKeyState` 等修饰键物理松开 → 写剪贴板 → `mark_used` → `SendInput` 模拟 Ctrl+V → Toast 提示 |
| `toast.rs` | Windows Toast 通知：`APP_AUMID = "com.snapcode.app"`；`ensure_app_shortcut()` 创建带 `AppUserModel.ID` 的开始菜单快捷方式；`show_toast()` 发送 ToastGeneric 通知 |
| `commands.rs` | 全部 14 个 `#[tauri::command]`：前后端契约的 Rust 侧实现，另有 `apply_autostart()` / `set_paused_impl()` 两个共享辅助函数 |

### 前端（`src/`）

- `main.tsx`：入口，首帧前应用主题避免亮暗闪烁。
- `App.tsx`：根组件。三种视图状态：加载中 → 首次引导（`Onboarding`）→ 主界面（`TopBar` + 历史/设置两个 Tab 页）；统一订阅 `code-added` / `listener-status` / `shortcut-error` 三个事件。
- `components/HistoryPage.tsx`：历史记录页（搜索、复制、删除）。
- `components/SettingsPage.tsx`：设置页（自动复制、快捷键、开机自启、保留策略、来源 AUMID 管理、主题、清空历史、调试区）。
- `components/Onboarding.tsx`：首次引导（通知权限授予流程）。
- `components/TopBar.tsx`：状态点 + Tab 切换 + 暂停/恢复。
- `components/TitleBar.tsx`：自定义标题栏（窗口 `decorations: false`，拖拽区 + 最小化/关闭按钮）。
- `components/ShortcutRecorder.tsx`：快捷键录制控件。
- `components/ui/`：shadcn 风格基础组件（`button.tsx` / `input.tsx` / `switch.tsx`）。
- `lib/tauri.ts`：**前后端契约的前端封装**——typed invoke + 事件订阅；无 Tauri 运行时（纯浏览器 `vite dev`）自动切换为内存 Mock，便于前端预览。
- `lib/utils.ts`：`cn()`、`DEFAULT_AUMID`（与 `settings.rs` 默认值一致）、`sourceDisplayName()`、`statusMeta()`。
- `lib/theme.ts`：亮/暗/跟随系统主题（localStorage 持久化）。
- `lib/time.ts`：时间格式化工具。
- `types.ts`：与 Rust serde 对应的契约类型（`CodeRecord` / `Settings` / `ListenerState` / `ToastInfo`）。

### 其他

- `scripts/gen-icon.mjs`：零依赖图标生成脚本（仅 Node 内置模块，手写 PNG/ICO 编码），输出到 `src-tauri/icons/`。
- `docs/USER_GUIDE.zh-CN.md`：中文用户手册。
- `README.md`：中英双语项目说明。
- `src-tauri/tauri.conf.json`：`identifier: com.snapcode.app`，主窗口 420×680、无边框（自定义标题栏）、`csp: null`。

## 常用命令

```bash
npm install                  # 安装前端依赖
npm run dev                  # 纯前端预览（vite，浏览器内自动使用内存 Mock，无 Tauri 后端）
npm run build                # tsc --noEmit && vite build —— 提交前必过
npm run tauri dev            # 桌面开发模式（前端热更新 + 真实 Rust 后端）
npm run tauri build          # 打包发布版本
cargo test --manifest-path src-tauri/Cargo.toml   # Rust 单测（parser + hotkey）—— 提交前必过
node scripts/gen-icon.mjs    # 重新生成应用图标（输出 src-tauri/icons/）
```

## 前后端契约（改动必读）

契约的单一事实来源是四组文件，必须保持同步：

- 命令封装与事件订阅：`src/lib/tauri.ts`
- 类型定义：`src/types.ts` ↔ `src-tauri/src/commands.rs` / `storage.rs`（`CodeRecord`）/ `settings.rs`（`Settings`）/ `state.rs`（`ListenerState`）/ `notifications.rs`（`ToastInfo`）
- 命令注册表：`src-tauri/src/lib.rs` 的 `invoke_handler`

**14 个命令**：`get_history` / `clear_history` / `delete_record` / `copy_code` / `get_settings` / `update_settings` / `get_listener_status` / `retry_listener` / `open_notification_settings` / `set_paused` / `simulate_notification` / `complete_onboarding` / `get_shortcut_error` / `dump_notifications`

**3 个事件**：`code-added`（新验证码入库后广播 `CodeRecord`）/ `listener-status`（监听状态变化广播 `ListenerState`）/ `shortcut-error`（快捷键注册失败广播 `string | null`）

修改任何一侧（增删命令、改字段、改事件 payload），必须同步：

1. Rust 侧结构体与 `#[tauri::command]` 实现、`lib.rs` 的 `invoke_handler` 列表；
2. `src/types.ts` 类型与 `src/lib/tauri.ts` 封装（含浏览器 Mock 分支）；
3. **Settings 默认值要三方对齐**：`src-tauri/src/settings.rs` 的 `Default` impl、`src/lib/utils.ts` 的 `DEFAULT_AUMID`、`src/lib/tauri.ts` 的 `mockSettings`。

注意：TS 侧 `number` = Rust `i64`；`received_at` 为 Unix 毫秒时间戳；`Settings` 带 `#[serde(default)]`，缺字段时回退默认值。

## 开发约定

- **Windows 专用代码必须 `cfg(windows)` 并保留非 Windows 桩**（参照 `notifications.rs` / `paste.rs` / `toast.rs` 的既有模式），保证 `cargo test` 在任意平台可编译运行。
- `parser.rs` 保持**纯函数**（不碰 IO / 全局状态），任何解析规则改动都要补单元测试。
- 不新增依赖（npm / cargo）除非确有必要；`gen-icon.mjs` 保持零依赖的传统。
- 改动完成后 **`npm run build` 与 `cargo test --manifest-path src-tauri/Cargo.toml` 双绿才算完成**。
- UI 文案使用简体中文；代码注释随现有风格（中文注释、说明「为什么」而非「是什么」）。
- 最小改动：不顺手重构、不改无关格式。

## 已知决策与坑（不要反复踩）

1. **windows 0.58 的 `RtlGetVersion` 在 `windows::Wdk::System::SystemServices`**（不在 `Win32` 下），用于判断系统 build ≥ 17763（Win10 1809）。见 `notifications.rs` 的 `is_supported_build()`；查询失败时乐观放行。
2. **`NotificationChanged` 事件订阅对无包身份应用必然失败**（`0x80070490 ELEMENT_NOT_FOUND`），因此订阅失败不是错误：自动降级为每 1s 全量轮询，状态保持 `running`（事件模式下也有 60s 低频兜底轮询）。见 `notifications.rs` 监听主循环。
3. **来源 AUMID 过滤是大小写无关的「包含匹配」**，兼容 `Microsoft.YourPhone_8wekyb3d8bbwe` 与 `...!App` 等书写变体。不要改成精确匹配。默认来源恢复按钮依赖 `DEFAULT_AUMID` 常量。
4. **`main.rs` 全构建形态 `windows_subsystem = "windows"`**：双击运行无控制台窗口；`eprintln!` 日志只在从终端 / `npm run tauri dev` 启动时经父进程管道可见。排查问题请从终端启动。
5. **Toast 身份依赖开始菜单快捷方式**：未打包应用没有系统 AUMID，`toast.rs` 的 `ensure_app_shortcut()` 在启动时于开始菜单 Programs 下创建带 `AppUserModel.ID = com.snapcode.app` 的 `.lnk`（先删旧再建，避免残留旧路径）。`show_toast()` 发送链：`CreateToastNotifier()` → `CreateToastNotifierWithId(APP_AUMID)` → 借用 PowerShell AUMID 兜底。windows 0.58 没有 `InitPropVariantFromString`，故先构造 `VT_BSTR` 再 `PropVariantChangeType` 转 `VT_LPWSTR`。
6. **粘贴前必须等修饰键物理松开**：热键回调在按下瞬间触发，此时用户手指往往仍按住 Ctrl/Shift，立即注入 Ctrl+V 会被污染成 Ctrl+Shift+V（多数应用不视为粘贴）。`paste.rs` 先以 ~15ms 间隔轮询 `GetAsyncKeyState`（总超时 ~600ms 后尽力继续），再写剪贴板、`SendInput`。**严禁在粘贴流程中激活/聚焦本程序窗口**，否则粘贴目标错误。
7. **目标应用以管理员权限运行时，UIPI 会拦截 `SendInput`**，粘贴静默失败属系统正常限制，不是 bug，不要试图「修复」。
8. **exe 被占用会导致 `cargo build` 链接失败**（`LNK1104` 之类）：dev 构建产物是 `target/debug/snapcode.exe`（tauri dev 直接运行它），先从托盘退出应用或 `taskkill //IM snapcode.exe //F` 再构建；安装版 exe 名为 `SnapCode.exe`。
9. 关闭主窗口是**隐藏到托盘**而非退出（`lib.rs` 的 `CloseRequested` 处理）；真正退出走托盘菜单「退出」。应用为单实例（second instance 只唤起主窗口）。
