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

pub struct AppState {
    pub db: Arc<Db>,
    pub settings: Arc<RwLock<Settings>>,
    pub status: RwLock<ListenerState>,
    pub paused: AtomicBool,
    pub monitor_alive: AtomicBool,
    pub shortcut_error: Mutex<Option<String>>,
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

    /// 当前界面语言（"zh-CN" | "en"），用于挑选用户可见文案。
    pub fn lang(&self) -> String {
        self.settings.read().unwrap().language.clone()
    }
}
