//! 前端可调用的 Tauri 命令。

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::hotkey;
use crate::i18n;
use crate::mail;
use crate::notifications;
use crate::parser::extract_code;
use crate::settings::{EmailSettings, Settings};
use crate::state::{AppState, EmailState, ListenerState};
use crate::storage::{now_millis, CodeRecord};
use crate::TrayItems;

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

/// 对齐开机自启状态。
/// enabled 时无条件重写注册表项：auto-launch 的 is_enabled 只判断键值存在、不校验路径，
/// exe 移动或覆盖安装后旧路径不会自愈，每次启动重写才能修正指向。
pub fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())?;
    } else if manager.is_enabled().map_err(|e| e.to_string())? {
        manager.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 暂停/恢复监听的统一入口（命令与托盘菜单共用）。
pub fn set_paused_impl(app: &AppHandle, state: &Arc<AppState>, paused: bool) {
    state.paused.store(paused, Ordering::SeqCst);
    if paused {
        state.set_status(app, "paused", None);
    } else if state.monitor_alive.load(Ordering::SeqCst) {
        state.set_status(app, "running", None);
    } else {
        // 监听线程已退出（如之前被拒绝授权），恢复时顺便重启
        notifications::spawn_monitor(app.clone(), state.clone());
    }
}

#[tauri::command]
pub fn get_history(
    state: State<'_, Arc<AppState>>,
    query: Option<String>,
) -> Result<Vec<CodeRecord>, String> {
    state.db.list(query.as_deref())
}

#[tauri::command]
pub fn clear_history(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.db.clear()
}

#[tauri::command]
pub fn delete_record(state: State<'_, Arc<AppState>>, id: i64) -> Result<(), String> {
    state.db.delete(id)
}

/// 复制指定记录的验证码到剪贴板并标记已用，返回验证码。
#[tauri::command]
pub fn copy_code(app: AppHandle, state: State<'_, Arc<AppState>>, id: i64) -> Result<String, String> {
    let record = state
        .db
        .get(id)?
        .ok_or_else(|| i18n::record_not_found(&state.lang()).to_string())?;
    app.clipboard()
        .write_text(&record.code)
        .map_err(|e| e.to_string())?;
    state.db.mark_used(id)?;
    Ok(record.code)
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, String> {
    Ok(state.settings.read().unwrap().clone())
}

/// 整体替换设置并应用副作用；快捷键注册等失败时拒绝整个更新。
#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    settings: Settings,
) -> Result<Settings, String> {
    let old = state.settings.read().unwrap().clone();

    // 快捷键重注册；失败则尝试还原旧快捷键并拒绝本次更新
    if let Err(e) = hotkey::register_shortcut(&app, state.inner(), &settings.shortcut) {
        let _ = hotkey::register_shortcut(&app, state.inner(), &old.shortcut);
        return Err(e);
    }

    // 开机自启对齐
    apply_autostart(&app, settings.autostart)?;

    // 持久化
    {
        let mut guard = state.settings.write().unwrap();
        *guard = settings.clone();
    }
    settings.save(&settings_path(&app)?)?;

    // 邮箱账户（host/username）变更后 UIDL 命名空间已不同：清空去重表，下次轮询重建基线
    if old.email.identity() != settings.email.identity() {
        let _ = state.db.email_seen_clear();
    }

    // 语言切换：托盘菜单与 tooltip 随新语言刷新
    if settings.language != old.language {
        let lang = &settings.language;
        if let Some(items) = app.try_state::<TrayItems>() {
            let _ = items.open.set_text(i18n::tray_open(lang));
            let _ = items.pause.set_text(i18n::tray_pause(
                lang,
                state.paused.load(Ordering::SeqCst),
            ));
            let _ = items.quit.set_text(i18n::tray_quit(lang));
        }
        if let Some(tray) = app.tray_by_id("tray") {
            let _ = tray.set_tooltip(Some(i18n::app_name(lang)));
        }
    }

    // 按新保留策略清理过期记录
    let _ = state.db.cleanup(settings.retention_days);

    Ok(settings)
}

#[tauri::command]
pub fn get_listener_status(state: State<'_, Arc<AppState>>) -> Result<ListenerState, String> {
    Ok(state.status.read().unwrap().clone())
}

/// 重新发起通知访问权限请求并重启监听线程。
#[tauri::command]
pub fn retry_listener(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    if state.monitor_alive.load(Ordering::SeqCst) {
        return Ok(()); // 监听线程仍在运行，无需重启
    }
    state.set_status(&app, "starting", None);
    notifications::spawn_monitor(app, state.inner().clone());
    Ok(())
}

#[tauri::command]
pub fn open_notification_settings() -> Result<(), String> {
    #[cfg(windows)]
    std::process::Command::new("cmd")
        .args(["/c", "start", "ms-settings:notifications"])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_paused(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    paused: bool,
) -> Result<(), String> {
    set_paused_impl(&app, state.inner(), paused);
    Ok(())
}

/// 模拟收到一条通知：走完整的解析 → 入库 → 广播 → 自动复制流程。
/// 返回识别到的验证码；未识别到返回 null。
#[tauri::command]
pub fn simulate_notification(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    text: String,
) -> Result<Option<String>, String> {
    let code = match extract_code(&text) {
        Some(c) => c,
        None => return Ok(None),
    };
    let record = state.db.insert(
        "debug",
        Some(i18n::simulated_sender(&state.lang())),
        &text,
        &code,
        now_millis(),
    )?;
    app.emit("code-added", &record)
        .map_err(|e| e.to_string())?;
    if state.settings.read().unwrap().auto_copy {
        let _ = app.clipboard().write_text(&code);
    }
    Ok(Some(code))
}

#[tauri::command]
pub fn complete_onboarding(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let guard = state.settings.read().unwrap();
    let mut settings = guard.clone();
    drop(guard);
    settings.onboarded = true;
    settings.save(&settings_path(&app)?)?;
    *state.settings.write().unwrap() = settings;
    Ok(())
}

#[tauri::command]
pub fn get_shortcut_error(state: State<'_, Arc<AppState>>) -> Result<Option<String>, String> {
    Ok(state.shortcut_error.lock().unwrap().clone())
}

/// 列出当前系统中的 Toast 通知（来源 AUMID + 文本），用于诊断来源过滤。
#[tauri::command]
pub fn dump_notifications(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<notifications::ToastInfo>, String> {
    notifications::dump_current_toasts(&state.lang())
}

/* ---------- 应用更新 ---------- */

/// 可用更新信息（契约类型，对应 src/types.ts 的 UpdateInfo）
#[derive(Clone, Debug, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub body: Option<String>,
    pub date: Option<String>,
}

/// 下载进度（update-download-progress 事件 payload）
#[derive(Clone, Debug, Serialize)]
struct UpdateProgress {
    downloaded: u64,
    total: Option<u64>,
}

/// 检查更新；已是最新返回 Ok(None)，网络或配置错误返回 Err（附端点诊断信息）。
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    use tauri_plugin_updater::UpdaterExt;

    let result = app.updater().map_err(|e| e.to_string())?.check().await;
    let update = match result {
        Ok(update) => update,
        Err(e) => {
            // 插件的错误不含具体状态码（如 ReleaseNotFound），补一次诊断性请求
            let detail = diagnose_update_endpoint(&app).await;
            return Err(format!("{e}{detail}"));
        }
    };
    Ok(update.map(|u| UpdateInfo {
        version: u.version,
        current_version: u.current_version,
        body: u.body,
        date: u
            .date
            .map(|d| format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())),
    }))
}

/// 更新检查失败时的辅助诊断：对同一端点再请求一次，报告 HTTP 状态或底层错误，
/// 弥补插件错误（如 ReleaseNotFound）不含具体原因（404 / 代理拦截 / 连接失败）的问题。
/// 注意：本函数总在插件 check() 之后调用，rustls 的 ring provider 已由插件安装。
async fn diagnose_update_endpoint(app: &AppHandle) -> String {
    // 端点与插件保持一致：取 tauri.conf.json plugins.updater.endpoints 的第一个
    let endpoint = app
        .config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("endpoints"))
        .and_then(|e| e.as_array())
        .and_then(|a| a.first())
        .and_then(|u| u.as_str())
        .map(str::to_owned);
    let Some(url) = endpoint else {
        return String::new();
    };
    let client = match reqwest::Client::builder()
        .user_agent("snapcode-update-diagnose")
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => return format!(" [diagnose: build client failed: {e}]"),
    };
    match client.get(&url).send().await {
        Ok(res) => format!(" [diagnose: endpoint returned {}]", res.status()),
        Err(e) => format!(" [diagnose: request failed: {e}]"),
    }
}

/// 下载并安装更新。下载完成后安装器以被动模式（仅进度条）接管，
/// 本进程随即自动退出，由安装器完成更新并重启应用——正常情况下不会返回 Ok。
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;

    let result = app.updater().map_err(|e| e.to_string())?.check().await;
    let update = match result {
        Ok(Some(update)) => update,
        Ok(None) => return Err("no update available".to_string()),
        Err(e) => {
            let detail = diagnose_update_endpoint(&app).await;
            return Err(format!("{e}{detail}"));
        }
    };

    let progress_app = app.clone();
    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            move |chunk_len, total| {
                downloaded += chunk_len as u64;
                let _ = progress_app.emit(
                    "update-download-progress",
                    UpdateProgress { downloaded, total },
                );
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/* ---------- 邮箱轮询（POP3） ---------- */

#[tauri::command]
pub fn get_email_status(state: State<'_, Arc<AppState>>) -> Result<EmailState, String> {
    Ok(state.email_status.read().unwrap().clone())
}

/// 测试邮箱连接（设置页「测试连接」）：连接 → 登录 → STAT，返回邮箱中的邮件总数。
/// config 为表单当前值（可能尚未保存）；阻塞网络调用放到线程池，避免卡住 UI。
#[tauri::command]
pub async fn test_email_connection(
    state: State<'_, Arc<AppState>>,
    config: EmailSettings,
) -> Result<i64, String> {
    let lang = state.lang();
    if !config.is_complete() {
        return Err(i18n::email_not_configured(&lang).to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut client =
            mail::connect(&config).map_err(|e| i18n::email_connect_failed(&lang, &e))?;
        let result = (|| -> Result<i64, String> {
            client
                .login(&config.username, &config.password)
                .map_err(|e| i18n::email_auth_failed(&lang, &e))?;
            let (count, _) = client
                .stat()
                .map_err(|e| i18n::email_poll_failed(&lang, &e))?;
            Ok(count as i64)
        })();
        client.quit();
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
