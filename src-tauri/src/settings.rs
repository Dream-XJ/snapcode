use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// 邮箱验证码监听（POP3 轮询）配置。嵌套在 Settings.email 下；
/// 旧版 settings.json 没有该字段，经 serde 容器级 default 回退为本默认值。
///
/// 安全说明：password（邮箱授权码）目前与其他设置一样明文存于 settings.json，
/// 该权衡已在用户手册中说明。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EmailSettings {
    /// 是否启用邮箱轮询
    pub enabled: bool,
    /// POP3 服务器地址，如 pop.qq.com
    pub host: String,
    /// 端口，SSL/TLS 一般为 995，明文为 110（TS number = Rust i64）
    pub port: i64,
    /// 邮箱账号（一般即完整邮箱地址）
    pub username: String,
    /// 密码或客户端授权码（QQ/163 等需用授权码）
    pub password: String,
    /// true = POP3S（隐式 TLS），false = 明文连接
    pub use_tls: bool,
    /// 轮询间隔秒数，最小 15
    pub poll_interval_secs: i64,
}

impl Default for EmailSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            host: String::new(),
            port: 995,
            username: String::new(),
            password: String::new(),
            use_tls: true,
            poll_interval_secs: 60,
        }
    }
}

impl EmailSettings {
    /// 关键字段是否足以发起连接（不含 enabled——「测试连接」可能发生在启用前）。
    pub fn is_complete(&self) -> bool {
        !self.host.trim().is_empty()
            && !self.username.trim().is_empty()
            && !self.password.is_empty()
            && self.port > 0
    }

    /// 归一化后的轮询间隔（秒），下限 15s，避免过于频繁被服务器拒绝。
    pub fn interval(&self) -> u64 {
        self.poll_interval_secs.max(15) as u64
    }

    /// 邮箱身份标识：host + username。变化时历史 UIDL 去重表需重建（不同邮箱 UIDL 命名空间不同）。
    pub fn identity(&self) -> String {
        format!("{}:{}", self.host.trim().to_lowercase(), self.username.trim())
    }
}

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
    /// 邮箱验证码监听配置
    pub email: EmailSettings,
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
            email: EmailSettings::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 旧版 settings.json（无 email 字段）加载后邮箱配置回退为默认值。
    #[test]
    fn legacy_json_without_email_gets_defaults() {
        let json = r#"{
            "auto_copy": true,
            "shortcut": "Ctrl+Shift+V",
            "autostart": true,
            "retention_days": 7,
            "aumids": ["Microsoft.YourPhone_8wekyb3d8bbwe"],
            "onboarded": true,
            "language": "zh-CN"
        }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.email, EmailSettings::default());
        assert!(!s.email.enabled);
        assert!(!s.email.is_complete());
    }

    /// email 字段部分缺失时，缺失项回退默认、已有项保留。
    #[test]
    fn partial_email_json_fills_defaults() {
        let json = r#"{ "email": { "enabled": true, "host": "pop.qq.com" } }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert!(s.email.enabled);
        assert_eq!(s.email.host, "pop.qq.com");
        assert_eq!(s.email.port, 995);
        assert!(s.email.use_tls);
        assert_eq!(s.email.poll_interval_secs, 60);
        // 缺账号/授权码，不算完整配置
        assert!(!s.email.is_complete());
    }

    /// 保存→加载往返后邮箱配置保持一致。
    #[test]
    fn save_load_roundtrip_preserves_email() {
        let mut s = Settings::default();
        s.email.enabled = true;
        s.email.host = "pop.163.com".to_string();
        s.email.port = 995;
        s.email.username = "user@163.com".to_string();
        s.email.password = "auth-code".to_string();
        s.email.poll_interval_secs = 120;

        let path = std::env::temp_dir().join(format!("snapcode-settings-test-{}.json", std::process::id()));
        s.save(&path).unwrap();
        let loaded = Settings::load(&path);
        let _ = fs::remove_file(&path);
        assert_eq!(loaded.email, s.email);
    }

    /// 完整性判断与轮询间隔下限。
    #[test]
    fn completeness_and_interval_clamp() {
        let mut e = EmailSettings::default();
        assert!(!e.is_complete());
        e.enabled = true;
        e.host = " pop.qq.com ".to_string();
        e.username = "u@qq.com".to_string();
        e.password = "x".to_string();
        assert!(e.is_complete());

        e.poll_interval_secs = 5;
        assert_eq!(e.interval(), 15);
        e.poll_interval_secs = 300;
        assert_eq!(e.interval(), 300);

        assert_eq!(e.identity(), "pop.qq.com:u@qq.com");
    }
}
