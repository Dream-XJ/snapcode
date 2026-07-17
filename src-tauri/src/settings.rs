use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// 应用设置，整体持久化到 app_config_dir/settings.json。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// 收到验证码后自动写入剪贴板
    pub auto_copy: bool,
    /// 全局快捷键，格式如 "Ctrl+Shift+V"
    pub shortcut: String,
    /// 开机自启
    pub autostart: bool,
    /// 历史记录保留天数，1/3/7/30，0 表示永久
    pub retention_days: i64,
    /// 监听的通知来源 AUMID 列表
    pub aumids: Vec<String>,
    /// 是否已完成首次引导
    pub onboarded: bool,
    /// 界面语言："zh-CN" | "en"
    pub language: String,
}

/// 默认界面语言：按系统 UI 语言判断，中文环境为 zh-CN，其余为 en。
/// 旧版 settings.json 缺少该字段时也经此回退（serde 容器级 default）。
#[cfg(windows)]
pub fn default_language() -> String {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;

    // LANGID 低 10 位为主语言标识，0x04 = 中文
    let primary = unsafe { GetUserDefaultUILanguage() } & 0x3ff;
    if primary == 0x04 {
        "zh-CN".to_string()
    } else {
        "en".to_string()
    }
}

/// 默认界面语言（非 Windows 桩）：开发预览默认为中文。
#[cfg(not(windows))]
pub fn default_language() -> String {
    "zh-CN".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_copy: true,
            shortcut: "Ctrl+Shift+V".to_string(),
            autostart: true,
            retention_days: 7,
            aumids: vec!["Microsoft.YourPhone_8wekyb3d8bbwe".to_string()],
            onboarded: false,
            language: default_language(),
        }
    }
}

impl Settings {
    /// 从 JSON 文件加载；文件缺失或损坏时回退到默认值。
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// 保存到 JSON 文件。
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())
    }
}
