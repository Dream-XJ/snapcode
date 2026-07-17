//! 全局快捷键：字符串解析与注册。

use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

use crate::i18n;
use crate::state::AppState;

/// 解析 "Ctrl+Shift+V" 形式的快捷键字符串。
pub fn parse_shortcut(lang: &str, s: &str) -> Result<Shortcut, String> {
    let mut modifiers = Modifiers::empty();
    let mut code: Option<Code> = None;

    for part in s.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()) {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "meta" | "win" | "command" | "super" => modifiers |= Modifiers::META,
            _ => {
                if code.is_some() {
                    return Err(i18n::shortcut_multi_keys(lang, s));
                }
                code = Some(parse_key_code(lang, part)?);
            }
        }
    }

    let code = code.ok_or_else(|| i18n::shortcut_no_key(lang, s))?;
    if modifiers.is_empty() {
        return Err(i18n::shortcut_no_modifier(lang, s));
    }
    Ok(Shortcut::new(Some(modifiers), code))
}

fn parse_key_code(lang: &str, key: &str) -> Result<Code, String> {
    let upper = key.to_uppercase();
    let code = match upper.as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "F13" => Code::F13,
        "F14" => Code::F14,
        "F15" => Code::F15,
        "F16" => Code::F16,
        "F17" => Code::F17,
        "F18" => Code::F18,
        "F19" => Code::F19,
        "F20" => Code::F20,
        "F21" => Code::F21,
        "F22" => Code::F22,
        "F23" => Code::F23,
        "F24" => Code::F24,
        "SPACE" => Code::Space,
        "TAB" => Code::Tab,
        "ENTER" | "RETURN" => Code::Enter,
        "ESCAPE" | "ESC" => Code::Escape,
        "BACKSPACE" => Code::Backspace,
        "DELETE" | "DEL" => Code::Delete,
        "INSERT" | "INS" => Code::Insert,
        "HOME" => Code::Home,
        "END" => Code::End,
        "PAGEUP" => Code::PageUp,
        "PAGEDOWN" => Code::PageDown,
        "UP" | "ARROWUP" => Code::ArrowUp,
        "DOWN" | "ARROWDOWN" => Code::ArrowDown,
        "LEFT" | "ARROWLEFT" => Code::ArrowLeft,
        "RIGHT" | "ARROWRIGHT" => Code::ArrowRight,
        _ => return Err(i18n::shortcut_unknown_key(lang, key)),
    };
    Ok(code)
}

/// 注销旧快捷键并注册新的；失败时写入 shortcut_error 并广播 "shortcut-error"。
pub fn register_shortcut(app: &AppHandle, state: &AppState, shortcut_str: &str) -> Result<(), String> {
    let lang = state.lang();
    let shortcut = parse_shortcut(&lang, shortcut_str)?;
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    match app.global_shortcut().register(shortcut) {
        Ok(()) => {
            set_shortcut_error(app, state, None);
            Ok(())
        }
        Err(e) => {
            let msg = i18n::shortcut_register_failed(&lang, shortcut_str, &e.to_string());
            set_shortcut_error(app, state, Some(msg.clone()));
            Err(msg)
        }
    }
}

/// 按当前设置注册快捷键。
pub fn register_from_settings(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let shortcut = state.settings.read().unwrap().shortcut.clone();
    register_shortcut(app, state, &shortcut)
}

fn set_shortcut_error(app: &AppHandle, state: &AppState, error: Option<String>) {
    *state.shortcut_error.lock().unwrap() = error.clone();
    let _ = app.emit("shortcut-error", error);
}

#[cfg(test)]
mod tests {
    use super::parse_shortcut;

    #[test]
    fn parses_valid_shortcuts() {
        assert!(parse_shortcut("zh-CN", "Ctrl+Shift+V").is_ok());
        assert!(parse_shortcut("zh-CN", "Alt+C").is_ok());
        assert!(parse_shortcut("zh-CN", "ctrl+shift+f5").is_ok());
        assert!(parse_shortcut("en", "Ctrl+Space").is_ok());
    }

    #[test]
    fn rejects_invalid_shortcuts() {
        assert!(parse_shortcut("zh-CN", "").is_err());
        assert!(parse_shortcut("zh-CN", "V").is_err()); // 缺少修饰键
        assert!(parse_shortcut("zh-CN", "Ctrl+Shift").is_err()); // 缺少按键
        assert!(parse_shortcut("zh-CN", "Ctrl+NotAKey").is_err());
        assert!(parse_shortcut("en", "Ctrl+NotAKey").is_err());
    }
}
