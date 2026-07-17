//! 前端可调用的 Tauri 命令。

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::hotkey;
use crate::i18n;
use crate::notifications;
use crate::parser::extract_code;
use crate::settings::Settings;
use crate::state::{AppState, ListenerState};
use crate::storage::{now_millis, CodeRecord};
use crate::TrayItems;

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

/// 对齐开机自启状态。
pub fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    let current = manager.is_enabled().map_err(|e| e.to_string())?;
    if enabled && !current {
        manager.enable().map_err(|e| e.to_string())?;
    } else if !enabled && current {
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
