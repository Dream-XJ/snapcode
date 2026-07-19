//! 用户可见文案的中英对照（Rust 侧）。
//! 语言来自 Settings.language（"zh-CN" | "en"），非 "en" 一律按中文处理。
//! 前端词典见 src/lib/i18n/。

/// 按语言二选一。
pub fn pick(lang: &str, zh: &'static str, en: &'static str) -> &'static str {
    if lang == "en" {
        en
    } else {
        zh
    }
}

fn is_en(lang: &str) -> bool {
    lang == "en"
}

/// 应用名（Toast 标题、托盘 tooltip）；统一英文名，不随语言变化
pub fn app_name(_lang: &str) -> &'static str {
    "SnapCode"
}

/* ---------- 托盘菜单 ---------- */

pub fn tray_open(lang: &str) -> &'static str {
    pick(lang, "打开主窗口", "Open SnapCode")
}

/// 暂停/恢复菜单项文本随状态变化
pub fn tray_pause(lang: &str, paused: bool) -> &'static str {
    match (is_en(lang), paused) {
        (false, false) => "暂停监听",
        (false, true) => "恢复监听",
        (true, false) => "Pause listening",
        (true, true) => "Resume listening",
    }
}

pub fn tray_quit(lang: &str) -> &'static str {
    pick(lang, "退出", "Quit")
}

/* ---------- 粘贴流程 Toast ---------- */
/* 注：部分文案仅在特定 cfg 平台的代码路径使用，用 cfg_attr 抑制跨平台死代码警告 */

pub fn no_code_available(lang: &str) -> &'static str {
    pick(lang, "暂无可用验证码", "No verification code yet")
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn read_code_failed(lang: &str) -> &'static str {
    pick(lang, "读取验证码失败", "Failed to read the code")
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn clipboard_write_failed(lang: &str) -> &'static str {
    pick(lang, "复制到剪贴板失败", "Failed to copy to clipboard")
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn pasted(lang: &str) -> &'static str {
    pick(lang, "验证码已复制并粘贴", "Code copied and pasted")
}

/// 非 Windows 桩：仅复制未粘贴
#[cfg_attr(windows, allow(dead_code))]
pub fn copied(lang: &str) -> &'static str {
    pick(lang, "验证码已复制", "Code copied")
}

/* ---------- 命令错误 ---------- */

pub fn record_not_found(lang: &str) -> &'static str {
    pick(lang, "记录不存在", "Record not found")
}

/// 模拟通知的发送人名称
pub fn simulated_sender(lang: &str) -> &'static str {
    pick(lang, "模拟通知", "Simulated")
}

/* ---------- 通知监听状态 / 诊断 ---------- */

#[cfg_attr(windows, allow(dead_code))]
pub fn unsupported_platform_listen(lang: &str) -> &'static str {
    pick(
        lang,
        "当前平台不支持通知监听",
        "Notification listening is not supported on this platform",
    )
}

#[cfg_attr(windows, allow(dead_code))]
pub fn unsupported_platform_dump(lang: &str) -> &'static str {
    pick(
        lang,
        "当前平台不支持通知读取",
        "Notification reading is not supported on this platform",
    )
}

pub fn unsupported_build(lang: &str) -> &'static str {
    pick(
        lang,
        "需要 Windows 10 1809 或更高版本",
        "Requires Windows 10 1809 or later",
    )
}

pub fn listener_init_failed(lang: &str, err: &str) -> String {
    if is_en(lang) {
        format!("Failed to initialize the notification listener: {err}")
    } else {
        format!("无法初始化通知监听器: {err}")
    }
}

pub fn access_denied_hint(lang: &str) -> &'static str {
    pick(
        lang,
        "未授予通知访问权限，请在系统设置中开启",
        "Notification access not granted. Enable it in system settings.",
    )
}

pub fn access_request_failed(lang: &str, err: &str) -> String {
    if is_en(lang) {
        format!("Failed to request notification access: {err}")
    } else {
        format!("请求通知访问权限失败: {err}")
    }
}

pub fn access_not_granted(lang: &str) -> &'static str {
    pick(lang, "未授予通知访问权限", "Notification access not granted")
}

/* ---------- 邮箱轮询（POP3） ---------- */

pub fn email_connect_failed(lang: &str, err: &str) -> String {
    if is_en(lang) {
        format!("Failed to connect to the mail server: {err}")
    } else {
        format!("无法连接邮箱服务器: {err}")
    }
}

pub fn email_auth_failed(lang: &str, err: &str) -> String {
    if is_en(lang) {
        format!("Mailbox login failed (check the auth code, not the login password): {err}")
    } else {
        format!("邮箱登录失败，请确认使用的是授权码而非登录密码: {err}")
    }
}

pub fn email_poll_failed(lang: &str, err: &str) -> String {
    if is_en(lang) {
        format!("Failed to poll the mailbox: {err}")
    } else {
        format!("轮询邮箱失败: {err}")
    }
}

pub fn email_not_configured(lang: &str) -> &'static str {
    pick(
        lang,
        "请先在设置中填写邮箱配置",
        "Configure your mailbox in Settings first",
    )
}

/// 账户已启用但关键字段（服务器/账号/授权码）未填全
pub fn email_config_incomplete(lang: &str) -> &'static str {
    pick(
        lang,
        "邮箱配置不完整，请在设置中补全服务器、账号与授权码",
        "Email settings incomplete: fill in server, account and auth code",
    )
}

/* ---------- 全局快捷键 ---------- */

pub fn shortcut_multi_keys(lang: &str, shortcut: &str) -> String {
    if is_en(lang) {
        format!("Shortcut \"{shortcut}\" is invalid: multiple keys")
    } else {
        format!("快捷键「{shortcut}」无效：包含多个按键")
    }
}

pub fn shortcut_no_key(lang: &str, shortcut: &str) -> String {
    if is_en(lang) {
        format!("Shortcut \"{shortcut}\" is invalid: missing key")
    } else {
        format!("快捷键「{shortcut}」无效：缺少按键")
    }
}

pub fn shortcut_no_modifier(lang: &str, shortcut: &str) -> String {
    if is_en(lang) {
        format!("Shortcut \"{shortcut}\" is invalid: at least one modifier is required")
    } else {
        format!("快捷键「{shortcut}」无效：至少需要一个修饰键")
    }
}

pub fn shortcut_unknown_key(lang: &str, key: &str) -> String {
    if is_en(lang) {
        format!("Unrecognized key \"{key}\"")
    } else {
        format!("无法识别的按键「{key}」")
    }
}

pub fn shortcut_register_failed(lang: &str, shortcut: &str, err: &str) -> String {
    if is_en(lang) {
        format!("Failed to register shortcut \"{shortcut}\", it may be in use by another program: {err}")
    } else {
        format!("快捷键「{shortcut}」注册失败，可能已被其他程序占用: {err}")
    }
}
