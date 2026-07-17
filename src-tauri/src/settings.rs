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
