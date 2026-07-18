use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// 邮箱协议。serde 小写序列化（"pop3" | "imap"），与前端 EmailProtocol 对应。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EmailProtocol {
    Pop3,
    Imap,
}

impl Default for EmailProtocol {
    fn default() -> Self {
        Self::Pop3
    }
}

/// 单个邮箱账户配置。
///
/// 安全说明：password（邮箱授权码）目前与其他设置一样明文存于 settings.json，
/// 该权衡已在用户手册中说明。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EmailAccount {
    /// 账户唯一标识（前端生成）；UIDL 去重、IMAP 同步状态与状态上报均以它为键
    pub id: String,
    /// 备注名；空时显示账号地址
    pub name: String,
    pub protocol: EmailProtocol,
    /// 是否启用轮询
    pub enabled: bool,
    /// 服务器地址，如 pop.qq.com / imap.qq.com
    pub host: String,
    /// 端口，SSL/TLS 一般为 995（POP3）/ 993（IMAP）（TS number = Rust i64）
    pub port: i64,
    /// 邮箱账号（一般即完整邮箱地址）
    pub username: String,
    /// 密码或客户端授权码（QQ/163 等需用授权码）
    pub password: String,
    /// true = 隐式 TLS，false = 明文连接
    pub use_tls: bool,
    /// 轮询间隔秒数，最小 15
    pub poll_interval_secs: i64,
}

impl Default for EmailAccount {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            protocol: EmailProtocol::Pop3,
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

impl EmailAccount {
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

    /// 账户身份标识：host + username。变化时该账户的 UIDL 去重记录需重建（不同邮箱 UIDL 命名空间不同）。
    pub fn identity(&self) -> String {
        format!("{}:{}", self.host.trim().to_lowercase(), self.username.trim())
    }

    /// 界面显示名：备注名优先，空时回退账号地址。
    pub fn display_name(&self) -> &str {
        let name = self.name.trim();
        if name.is_empty() {
            self.username.trim()
        } else {
            name
        }
    }
}

/// 邮箱验证码监听配置：多账户列表。嵌套在 Settings.email 下；
/// 旧版 settings.json 没有该字段，经 serde 容器级 default 回退为空列表。
#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub struct EmailSettings {
    pub accounts: Vec<EmailAccount>,
}

/// 自定义反序列化以兼容旧版扁平单账户 JSON：
/// 含 accounts 键 → 新版，直接使用；否则按旧字段读取——enabled 为 true 或
/// host/username/password 任一非空时迁移为 id="default" 的 POP3 单账户（其余
/// 字段取旧值），全为空默认则视为从未配置（空列表）。
impl<'de> Deserialize<'de> for EmailSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        /// 新旧两种形态的并集；旧字段的默认值与旧版默认配置一致
        #[derive(Deserialize)]
        #[serde(default)]
        struct Compat {
            accounts: Option<Vec<EmailAccount>>,
            enabled: bool,
            host: String,
            port: i64,
            username: String,
            password: String,
            use_tls: bool,
            poll_interval_secs: i64,
        }

        impl Default for Compat {
            fn default() -> Self {
                Self {
                    accounts: None,
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

        let compat = Compat::deserialize(deserializer)?;
        if let Some(accounts) = compat.accounts {
            return Ok(Self { accounts });
        }
        if compat.enabled
            || !compat.host.is_empty()
            || !compat.username.is_empty()
            || !compat.password.is_empty()
        {
            Ok(Self {
                accounts: vec![EmailAccount {
                    id: "default".to_string(),
                    name: String::new(),
                    protocol: EmailProtocol::Pop3,
                    enabled: compat.enabled,
                    host: compat.host,
                    port: compat.port,
                    username: compat.username,
                    password: compat.password,
                    use_tls: compat.use_tls,
                    poll_interval_secs: compat.poll_interval_secs,
                }],
            })
        } else {
            Ok(Self::default())
        }
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

    /// 旧版 settings.json（无 email 字段）加载后邮箱账户列表为空。
    #[test]
    fn legacy_json_without_email_gets_empty_accounts() {
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
        assert!(s.email.accounts.is_empty());
    }

    /// 旧版扁平单账户 email JSON（非默认值）迁移为 id="default" 的 POP3 账户，字段值保留。
    #[test]
    fn legacy_flat_email_migrates_to_default_account() {
        let json = r#"{ "email": {
            "enabled": true,
            "host": "pop.qq.com",
            "port": 110,
            "username": "u@qq.com",
            "password": "auth-code",
            "use_tls": false,
            "poll_interval_secs": 120
        } }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.email.accounts.len(), 1);
        let acc = &s.email.accounts[0];
        assert_eq!(acc.id, "default");
        assert_eq!(acc.name, "");
        assert_eq!(acc.protocol, EmailProtocol::Pop3);
        assert!(acc.enabled);
        assert_eq!(acc.host, "pop.qq.com");
        assert_eq!(acc.port, 110);
        assert_eq!(acc.username, "u@qq.com");
        assert_eq!(acc.password, "auth-code");
        assert!(!acc.use_tls);
        assert_eq!(acc.poll_interval_secs, 120);
        assert!(acc.is_complete());
    }

    /// 旧版扁平配置只填了部分字段（未启用但填了服务器）同样迁移，缺失项回退旧默认值。
    #[test]
    fn legacy_partial_email_still_migrates() {
        let json = r#"{ "email": { "host": "pop.163.com" } }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.email.accounts.len(), 1);
        let acc = &s.email.accounts[0];
        assert_eq!(acc.id, "default");
        assert!(!acc.enabled);
        assert_eq!(acc.host, "pop.163.com");
        // 缺失字段回退旧版默认值
        assert_eq!(acc.port, 995);
        assert!(acc.use_tls);
        assert_eq!(acc.poll_interval_secs, 60);
        // 缺账号/授权码，不算完整配置
        assert!(!acc.is_complete());
    }

    /// 旧版 email 全默认（空对象或各字段默认值）视为从未配置：accounts 为空。
    #[test]
    fn legacy_default_email_yields_no_accounts() {
        let s: Settings = serde_json::from_str(r#"{ "email": {} }"#).unwrap();
        assert!(s.email.accounts.is_empty());

        let json = r#"{ "email": {
            "enabled": false,
            "host": "",
            "port": 995,
            "username": "",
            "password": "",
            "use_tls": true,
            "poll_interval_secs": 60
        } }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert!(s.email.accounts.is_empty());
    }

    /// 新版 accounts JSON 保存→加载往返保持一致。
    #[test]
    fn save_load_roundtrip_preserves_accounts() {
        let mut s = Settings::default();
        s.email.accounts.push(EmailAccount {
            id: "a1".to_string(),
            name: "工作邮箱".to_string(),
            protocol: EmailProtocol::Imap,
            enabled: true,
            host: "imap.163.com".to_string(),
            port: 993,
            username: "user@163.com".to_string(),
            password: "auth-code".to_string(),
            use_tls: true,
            poll_interval_secs: 120,
        });
        s.email.accounts.push(EmailAccount {
            id: "a2".to_string(),
            enabled: true,
            host: "pop.qq.com".to_string(),
            username: "u@qq.com".to_string(),
            password: "x".to_string(),
            ..EmailAccount::default()
        });

        let path = std::env::temp_dir().join(format!("snapcode-settings-test-{}.json", std::process::id()));
        s.save(&path).unwrap();
        let loaded = Settings::load(&path);
        let _ = fs::remove_file(&path);
        assert_eq!(loaded.email, s.email);
    }

    /// 损坏的 settings.json 整体回退默认。
    #[test]
    fn corrupted_json_falls_back_to_default() {
        let path = std::env::temp_dir().join(format!("snapcode-settings-corrupt-{}.json", std::process::id()));
        fs::write(&path, "{ not valid json !!!").unwrap();
        let loaded = Settings::load(&path);
        let _ = fs::remove_file(&path);
        assert_eq!(loaded.email, EmailSettings::default());
        assert_eq!(loaded.shortcut, Settings::default().shortcut);
    }

    /// 完整性判断、轮询间隔下限、身份标识与显示名。
    #[test]
    fn account_completeness_interval_identity_display_name() {
        let mut acc = EmailAccount::default();
        assert!(!acc.is_complete());
        acc.enabled = true;
        acc.host = " pop.qq.com ".to_string();
        acc.username = "u@qq.com".to_string();
        acc.password = "x".to_string();
        assert!(acc.is_complete());

        acc.poll_interval_secs = 5;
        assert_eq!(acc.interval(), 15);
        acc.poll_interval_secs = 300;
        assert_eq!(acc.interval(), 300);

        assert_eq!(acc.identity(), "pop.qq.com:u@qq.com");

        // 备注名优先；为空时回退账号地址
        assert_eq!(acc.display_name(), "u@qq.com");
        acc.name = "  QQ 邮箱 ".to_string();
        assert_eq!(acc.display_name(), "QQ 邮箱");
    }
}
