//! 快捷粘贴：复制最新验证码并模拟 Ctrl+V 粘贴到当前焦点窗口。

use std::sync::Arc;

use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::state::AppState;
use crate::toast::show_toast;

/// 快捷键按下时调用：取最新验证码 → 写剪贴板 → 标记已用 → 模拟 Ctrl+V。
/// 注意：严禁在此激活/聚焦本程序窗口，否则粘贴目标会错误。
#[cfg(windows)]
pub fn paste_latest(app: &AppHandle, state: &Arc<AppState>) {
    let record = match state.db.latest() {
        Ok(Some(r)) => r,
        Ok(None) => {
            show_toast("SnapCode 闪码", "暂无可用验证码");
            return;
        }
        Err(e) => {
            eprintln!("读取最新验证码失败: {e}");
            show_toast("SnapCode 闪码", "读取验证码失败");
            return;
        }
    };

    // 热键回调在按下瞬间触发，此时用户手指往往仍按住 Ctrl/Shift。
    // 若立即注入 Ctrl+V，仍按住的 Shift 会把按键污染成 Ctrl+Shift+V，
    // 多数应用不视为粘贴。因此先等用户物理松开所有修饰键再继续。
    wait_for_modifiers_released();

    if let Err(e) = app.clipboard().write_text(&record.code) {
        eprintln!("写入剪贴板失败: {e}");
        show_toast("SnapCode 闪码", "复制到剪贴板失败");
        return;
    }
    let _ = state.db.mark_used(record.id);

    // 稍作等待，确保剪贴板写入生效
    std::thread::sleep(std::time::Duration::from_millis(30));
    unsafe {
        send_ctrl_v();
    }
    show_toast("SnapCode 闪码", "验证码已复制并粘贴");
}

/// 轮询等待用户物理松开所有修饰键（Ctrl/Shift/Alt/Win）。
/// 每 ~15ms 检查一次，全部松开即返回；总超时 ~600ms 后尽力而为地继续。
/// 主键（如 V）无需等待——全局热键已吞掉该按键，不会输入字符。
#[cfg(windows)]
fn wait_for_modifiers_released() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };

    const MODIFIERS: [u16; 5] = [
        VK_CONTROL.0,
        VK_SHIFT.0,
        VK_MENU.0,
        VK_LWIN.0,
        VK_RWIN.0,
    ];
    const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(15);
    const TIMEOUT: std::time::Duration = std::time::Duration::from_millis(600);

    let start = std::time::Instant::now();
    loop {
        // GetAsyncKeyState 返回 i16，最高位（负数）表示该键当前被按下
        let any_pressed = MODIFIERS
            .iter()
            .any(|&vk| unsafe { GetAsyncKeyState(vk as i32) } < 0);
        if !any_pressed {
            return;
        }
        if start.elapsed() >= TIMEOUT {
            eprintln!("等待修饰键松开超时（600ms），仍继续粘贴流程");
            return;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// 通过 SendInput 模拟一次完整的 Ctrl+V（Ctrl down / V down / V up / Ctrl up）。
#[cfg(windows)]
unsafe fn send_ctrl_v() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        KEYBD_EVENT_FLAGS, VIRTUAL_KEY, VK_CONTROL, VK_V,
    };

    fn key_input(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    let inputs = [
        key_input(VK_CONTROL, KEYBD_EVENT_FLAGS::default()),
        key_input(VK_V, KEYBD_EVENT_FLAGS::default()),
        key_input(VK_V, KEYEVENTF_KEYUP),
        key_input(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    if sent as usize != inputs.len() {
        eprintln!("SendInput 只发送了 {sent}/{} 个事件", inputs.len());
    }
}

#[cfg(not(windows))]
pub fn paste_latest(app: &AppHandle, state: &Arc<AppState>) {
    // 非 Windows 平台桩：仅复制到剪贴板
    match state.db.latest() {
        Ok(Some(record)) => {
            let _ = app.clipboard().write_text(&record.code);
            let _ = state.db.mark_used(record.id);
            show_toast("SnapCode 闪码", "验证码已复制");
        }
        _ => show_toast("SnapCode 闪码", "暂无可用验证码"),
    }
}
