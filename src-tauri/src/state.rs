use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::settings::Settings;
use crate::storage::Db;

/// 监听器状态，state ∈ starting|running|paused|access_denied|unsupported|error
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListenerState {
    pub state: String,
    pub message: Option<String>,
}

impl ListenerState {
    pub fn new(state: &str, message: Option<String>) -> Self {
        Self {
            state: state.to_string(),
            message,
        }
    }
}

impl Default for ListenerState {
    fn default() -> Self {
        Self::new("starting", None)
    }
}

/// 单个账户的邮箱轮询状态，state ∈ disabled|running|paused|error
/// （disabled：未启用或配置不完整；error：message 带最近一次的失败原因）
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EmailState {
    pub state: String,
    pub message: Option<String>,
}

impl EmailState {
    pub fn new(state: &str, message: Option<String>) -> Self {
        Self {
            state: state.to_string(),
            message,
        }
    }
}

impl Default for EmailState {
    fn default() -> Self {
        Self::new("disabled", None)
    }
}

/// 带账户 id 的邮箱轮询状态（email-status 事件 payload / get_email_status 返回值的元素）
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EmailAccountStatus {
    pub account_id: String,
    pub state: String,
    pub message: Option<String>,
}

/// HashMap → 按 account_id 排序的列表（命令返回值与事件广播共用同一形状）。
fn sorted_status_list(map: &HashMap<String, EmailState>) -> Vec<EmailAccountStatus> {
    let mut list: Vec<EmailAccountStatus> = map
        .iter()
        .map(|(id, s)| EmailAccountStatus {
            account_id: id.clone(),
            state: s.state.clone(),
            message: s.message.clone(),
        })
        .collect();
    list.sort_by(|a, b| a.account_id.cmp(&b.account_id));
    list
}

pub struct AppState {
    pub db: Arc<Db>,
    pub settings: Arc<RwLock<Settings>>,
    pub status: RwLock<ListenerState>,
    pub paused: AtomicBool,
    pub monitor_alive: AtomicBool,
    pub shortcut_error: Mutex<Option<String>>,
    /// 各邮箱账户的轮询状态，键为账户 id
    pub email_status: RwLock<HashMap<String, EmailState>>,
    pub email_alive: AtomicBool,
}

impl AppState {
    /// 更新监听器状态并广播 "listener-status" 事件到全部窗口。
    pub fn set_status(&self, app: &AppHandle, state: &str, message: Option<String>) {
        let payload = {
            let mut guard = self.status.write().unwrap();
            *guard = ListenerState::new(state, message);
            guard.clone()
        };
        let _ = app.emit("listener-status", payload);
    }

    /// 更新指定账户的邮箱轮询状态并广播 "email-status" 事件到全部窗口。
    /// 该账户状态未变化时不重复广播（轮询线程每秒都会走到这里）。
    pub fn set_email_status(
        &self,
        app: &AppHandle,
        account_id: &str,
        state: &str,
        message: Option<String>,
    ) {
        let payload = {
            let mut guard = self.email_status.write().unwrap();
            let unchanged = guard
                .get(account_id)
                .map(|cur| cur.state == state && cur.message == message)
                .unwrap_or(false);
            if unchanged {
                return;
            }
            guard.insert(account_id.to_string(), EmailState::new(state, message));
            sorted_status_list(&guard)
        };
        let _ = app.emit("email-status", payload);
    }

    /// 移除指定账户的状态（账户被删除或不再是工作账户时）并广播；原本没有则不广播。
    pub fn remove_email_status(&self, app: &AppHandle, account_id: &str) {
        let payload = {
            let mut guard = self.email_status.write().unwrap();
            if guard.remove(account_id).is_none() {
                return;
            }
            sorted_status_list(&guard)
        };
        let _ = app.emit("email-status", payload);
    }

    /// 当前全部账户的轮询状态（按 account_id 排序）。
    pub fn email_status_list(&self) -> Vec<EmailAccountStatus> {
        sorted_status_list(&self.email_status.read().unwrap())
    }

    /// 当前界面语言（"zh-CN" | "en"），用于挑选用户可见文案。
    pub fn lang(&self) -> String {
        self.settings.read().unwrap().language.clone()
    }
}
