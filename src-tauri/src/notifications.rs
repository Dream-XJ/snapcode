//! Windows 通知监听：通过 WinRT UserNotificationListener 捕获 Toast 通知。

use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::AppHandle;

use crate::state::AppState;

/// 启动通知监听线程；CAS 保证不会重复启动。
#[cfg(windows)]
pub fn spawn_monitor(app: AppHandle, state: Arc<AppState>) {
    if state
        .monitor_alive
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    std::thread::spawn(move || {
        imp::run(&app, &state);
        state.monitor_alive.store(false, Ordering::SeqCst);
    });
}

#[cfg(not(windows))]
pub fn spawn_monitor(app: AppHandle, state: Arc<AppState>) {
    state.set_status(
        &app,
        "unsupported",
        Some("当前平台不支持通知监听".to_string()),
    );
}

/// 诊断用：一条系统 Toast 通知的来源与文本。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToastInfo {
    pub aumid: String,
    pub title: String,
    pub body: String,
}

/// 列出当前系统中的 Toast 通知（诊断来源过滤用）。
#[cfg(windows)]
pub fn dump_current_toasts() -> Result<Vec<ToastInfo>, String> {
    imp::dump_current_toasts()
}

/// 列出当前系统中的 Toast 通知（诊断来源过滤用）。
#[cfg(not(windows))]
pub fn dump_current_toasts() -> Result<Vec<ToastInfo>, String> {
    Err("当前平台不支持通知读取".to_string())
}

#[cfg(windows)]
mod imp {
    use std::collections::HashSet;
    use std::sync::atomic::Ordering;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::Duration;

    use tauri::{AppHandle, Emitter};
    use tauri_plugin_clipboard_manager::ClipboardExt;
    use windows::core::HSTRING;
    use windows::Foundation::TypedEventHandler;
    use windows::UI::Notifications::Management::{
        UserNotificationListener, UserNotificationListenerAccessStatus,
    };
    use windows::UI::Notifications::{
        KnownNotificationBindings, NotificationKinds, UserNotification,
        UserNotificationChangedEventArgs, UserNotificationChangedKind,
    };
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    use crate::parser::extract_code;
    use crate::state::AppState;
    use crate::storage::now_millis;

    /// 监听线程主函数。任何失败只记录日志，不 panic。
    pub fn run(app: &AppHandle, state: &Arc<AppState>) {
        unsafe {
            // 忽略返回值：已初始化或模式冲突都不影响后续 WinRT 调用
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }

        if !is_supported_build() {
            state.set_status(
                app,
                "unsupported",
                Some("需要 Windows 10 1809 或更高版本".to_string()),
            );
            return;
        }

        state.set_status(app, "starting", None);

        let listener = match UserNotificationListener::Current() {
            Ok(l) => l,
            Err(e) => {
                state.set_status(app, "error", Some(format!("无法初始化通知监听器: {e}")));
                return;
            }
        };

        match listener.RequestAccessAsync().and_then(|op| op.get()) {
            Ok(status) => {
                if status != UserNotificationListenerAccessStatus::Allowed {
                    state.set_status(
                        app,
                        "access_denied",
                        Some("未授予通知访问权限，请在系统设置中开启".to_string()),
                    );
                    return;
                }
            }
            Err(e) => {
                state.set_status(app, "error", Some(format!("请求通知访问权限失败: {e}")));
                return;
            }
        }

        let mut seen: HashSet<u32> = HashSet::new();

        if state.paused.load(Ordering::SeqCst) {
            state.set_status(app, "paused", None);
        } else {
            state.set_status(app, "running", None);
            // 补录当前已存在的 Toast，避免漏掉启动前收到的验证码
            poll_once(&listener, app, state, &mut seen);
        }

        // 订阅通知变更事件，Added 事件的通知 Id 通过 channel 送回主循环处理。
        // 部分环境（如无包身份的应用）订阅会失败（如 0x80070490 ELEMENT_NOT_FOUND），
        // 此时不报错退出，而是降级为轮询模式，状态保持 running。
        let (tx, rx) = mpsc::channel::<u32>();
        let handler = TypedEventHandler::new(
            move |_sender: &Option<UserNotificationListener>,
                  args: &Option<UserNotificationChangedEventArgs>|
             -> windows::core::Result<()> {
                if let Some(args) = args {
                    if args.ChangeKind()? == UserNotificationChangedKind::Added {
                        let _ = tx.send(args.UserNotificationId()?);
                    }
                }
                Ok(())
            },
        );

        // 事件模式：超时（60s）做一次低频兜底轮询；轮询模式：每 1s 全量拉取一次
        let poll_interval = match listener.NotificationChanged(&handler) {
            Ok(_) => Duration::from_secs(60),
            Err(e) => {
                eprintln!("注册通知变更事件失败: {e}");
                eprintln!("已降级为轮询模式：每 1 秒全量拉取一次 Toast 通知");
                Duration::from_secs(1)
            }
        };

        loop {
            match rx.recv_timeout(poll_interval) {
                Ok(id) => {
                    if state.paused.load(Ordering::SeqCst) {
                        continue; // 暂停期间直接丢弃
                    }
                    match listener.GetNotification(id) {
                        Ok(n) => handle_notification(app, state, &mut seen, &n),
                        Err(e) => eprintln!("获取通知 {id} 失败: {e}"),
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if state.paused.load(Ordering::SeqCst) {
                        continue; // 暂停期间跳过轮询
                    }
                    poll_once(&listener, app, state, &mut seen);
                }
                // handler 在本函数作用域内一直存活，channel 实际不会断开
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    /// 全量拉取现有 Toast 并逐条处理；依靠 seen 按通知 Id 去重，只处理新出现的 Id。
    fn poll_once(
        listener: &UserNotificationListener,
        app: &AppHandle,
        state: &Arc<AppState>,
        seen: &mut HashSet<u32>,
    ) {
        match listener
            .GetNotificationsAsync(NotificationKinds::Toast)
            .and_then(|op| op.get())
        {
            Ok(list) => {
                let size = list.Size().unwrap_or(0);
                for i in 0..size {
                    match list.GetAt(i) {
                        Ok(n) => handle_notification(app, state, seen, &n),
                        Err(e) => eprintln!("读取现有通知失败: {e}"),
                    }
                }
            }
            Err(e) => eprintln!("获取现有通知列表失败: {e}"),
        }
    }

    /// 处理单条通知：过滤来源、提取文本、识别验证码、入库并广播。
    fn handle_notification(
        app: &AppHandle,
        state: &Arc<AppState>,
        seen: &mut HashSet<u32>,
        n: &UserNotification,
    ) {
        let id = match n.Id() {
            Ok(id) => id,
            Err(e) => {
                eprintln!("读取通知 Id 失败: {e}");
                return;
            }
        };
        if !seen.insert(id) {
            return; // 已处理过
        }

        let source = match n.AppInfo().and_then(|info| info.AppUserModelId()) {
            Ok(a) => a.to_string_lossy(),
            Err(e) => {
                eprintln!("读取通知来源失败: {e}");
                return;
            }
        };

        let allowed = {
            let settings = state.settings.read().unwrap();
            let source_lc = source.to_lowercase();
            settings.aumids.iter().any(|a| {
                let configured = a.trim().to_lowercase();
                // 包含匹配：兼容 "PFN" 与 "PFN!App" 等 AUMID 书写变体
                !configured.is_empty() && source_lc.contains(&configured)
            })
        };
        if !allowed {
            return;
        }

        let (sender, body) = match read_toast_texts(n) {
            Some(t) => t,
            None => return,
        };
        let full_text = format!("{sender}\n{body}");

        let code = match extract_code(&full_text) {
            Some(c) => c,
            None => return,
        };

        // CreationTime 为 1601-01-01 起的 100ns 计数，换算成 unix 毫秒
        let received_at = match n.CreationTime() {
            Ok(t) => t.UniversalTime.saturating_sub(116_444_736_000_000_000) / 10_000,
            Err(_) => now_millis(),
        };

        let record = match state
            .db
            .insert(&source, Some(&sender), &body, &code, received_at)
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("验证码入库失败: {e}");
                return;
            }
        };

        if let Err(e) = app.emit("code-added", &record) {
            eprintln!("广播 code-added 事件失败: {e}");
        }

        let auto_copy = state.settings.read().unwrap().auto_copy;
        if auto_copy {
            if let Err(e) = app.clipboard().write_text(record.code.clone()) {
                eprintln!("自动复制到剪贴板失败: {e}");
            }
        }
    }

    /// 读取 Toast 的文本元素：首个非空元素为标题（发送人），其余拼接为正文。
    fn read_toast_texts(n: &UserNotification) -> Option<(String, String)> {
        let visual = n.Notification().and_then(|notif| notif.Visual()).ok()?;
        let binding_name = KnownNotificationBindings::ToastGeneric()
            .unwrap_or_else(|_| HSTRING::from("ToastGeneric"));
        let binding = visual.GetBinding(&binding_name).ok()?;
        let texts = binding.GetTextElements().ok()?;
        let size = texts.Size().unwrap_or(0);
        let mut parts: Vec<String> = Vec::new();
        for i in 0..size {
            if let Ok(item) = texts.GetAt(i) {
                if let Ok(t) = item.Text() {
                    let s = t.to_string_lossy();
                    if !s.trim().is_empty() {
                        parts.push(s);
                    }
                }
            }
        }
        if parts.is_empty() {
            return None;
        }
        let sender = parts.remove(0);
        Some((sender, parts.join("\n")))
    }

    /// 列出当前所有 Toast 通知的来源 AUMID 与文本（诊断用）。
    pub fn dump_current_toasts() -> Result<Vec<super::ToastInfo>, String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let listener = UserNotificationListener::Current().map_err(|e| e.to_string())?;
        let access = listener
            .RequestAccessAsync()
            .and_then(|op| op.get())
            .map_err(|e| e.to_string())?;
        if access != UserNotificationListenerAccessStatus::Allowed {
            return Err("未授予通知访问权限".to_string());
        }
        let list = listener
            .GetNotificationsAsync(NotificationKinds::Toast)
            .and_then(|op| op.get())
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for i in 0..list.Size().unwrap_or(0) {
            if let Ok(n) = list.GetAt(i) {
                let aumid = n
                    .AppInfo()
                    .and_then(|info| info.AppUserModelId())
                    .map(|s| s.to_string_lossy())
                    .unwrap_or_default();
                let (title, body) = read_toast_texts(&n).unwrap_or_default();
                out.push(super::ToastInfo { aumid, title, body });
            }
        }
        Ok(out)
    }

    /// 通过 RtlGetVersion 判断系统版本是否 >= Windows 10 1809 (build 17763)。
    /// 查询失败时乐观放行。
    fn is_supported_build() -> bool {
        use windows::Wdk::System::SystemServices::RtlGetVersion;
        use windows::Win32::System::SystemInformation::OSVERSIONINFOW;

        let mut info = OSVERSIONINFOW::default();
        info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
        let status = unsafe { RtlGetVersion(&mut info) };
        // NTSTATUS 成功值为非负
        if status.0 >= 0 {
            info.dwBuildNumber >= 17763
        } else {
            true
        }
    }
}
