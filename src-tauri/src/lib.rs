mod commands;
mod hotkey;
mod notifications;
mod parser;
mod paste;
mod settings;
mod state;
mod storage;
mod toast;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

use settings::Settings;
use state::{AppState, ListenerState};
use storage::Db;

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn build_tray(app: &App) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "打开主窗口", true, None::<&str>)?;
    let pause_item = MenuItem::with_id(app, "pause", "暂停监听", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &pause_item, &quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("SnapCode 闪码")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "pause" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    let paused = !state.paused.load(Ordering::SeqCst);
                    commands::set_paused_impl(app, state.inner(), paused);
                    let _ = pause_item.set_text(if paused { "恢复监听" } else { "暂停监听" });
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Some(state) = app.try_state::<Arc<AppState>>() {
                            paste::paste_latest(app, state.inner());
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // 目录
            let config_dir = app_handle.path().app_config_dir()?;
            std::fs::create_dir_all(&config_dir)?;
            let data_dir = app_handle.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            // 设置与数据库
            let settings = Settings::load(&config_dir.join("settings.json"));
            let db = Db::open(&data_dir.join("snapcode.db"))
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

            let state = Arc::new(AppState {
                db: Arc::new(db),
                settings: Arc::new(RwLock::new(settings.clone())),
                status: RwLock::new(ListenerState::default()),
                paused: AtomicBool::new(false),
                monitor_alive: AtomicBool::new(false),
                shortcut_error: Mutex::new(None),
            });
            app_handle.manage(state.clone());

            // 开机自启与设置对齐
            if let Err(e) = commands::apply_autostart(&app_handle, settings.autostart) {
                eprintln!("设置开机自启失败: {e}");
            }

            // 确保开始菜单存在带 AUMID 的快捷方式，Toast 才能以本应用身份显示
            if let Err(e) = toast::ensure_app_shortcut() {
                eprintln!("创建应用快捷方式失败: {e}");
            }

            // 清理过期历史记录
            if let Err(e) = state.db.cleanup(settings.retention_days) {
                eprintln!("清理过期记录失败: {e}");
            }

            // 托盘
            build_tray(app)?;

            // 全局快捷键
            if let Err(e) = hotkey::register_from_settings(&app_handle, &state) {
                eprintln!("注册全局快捷键失败: {e}");
            }

            // 通知监听
            notifications::spawn_monitor(app_handle.clone(), state);

            Ok(())
        })
        .on_window_event(|window, event| {
            // 关闭主窗口时隐藏到托盘而不是退出
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_history,
            commands::clear_history,
            commands::delete_record,
            commands::copy_code,
            commands::get_settings,
            commands::update_settings,
            commands::get_listener_status,
            commands::retry_listener,
            commands::open_notification_settings,
            commands::set_paused,
            commands::simulate_notification,
            commands::complete_onboarding,
            commands::get_shortcut_error,
            commands::dump_notifications,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
